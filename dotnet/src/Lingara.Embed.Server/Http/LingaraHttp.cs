using System;
using System.Globalization;
using System.Net;
using System.Net.Http;
using System.Net.Http.Headers;
using System.Reflection;
using System.Text;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;

namespace Lingara.Embed.Server.Http;

/// <summary>
/// What every request shares: the HTTP client, the <c>User-Agent</c>, and
/// the two test seams (contract "Test seams"): the clock, and the sleeper
/// that <see cref="Task.Delay(TimeSpan, TimeProvider, CancellationToken)"/> reads.
/// </summary>
internal sealed record Transport(HttpClient Http, string UserAgent, TimeProvider Time);

/// <summary>
/// The <c>/v1/</c> half of ADR 1.10.26af D10: contract K2 (the pin), K4 (the
/// <c>Retry-After</c> loop) and K6 (identification), around K1's one 401
/// retry against <see cref="TokenCache"/>. Exactly two calls ride on it.
/// </summary>
internal sealed class LingaraHttp
{
    private readonly Transport transport;
    private readonly TokenCache tokens;
    private readonly Uri baseUrl;
    private readonly string? version;

    internal LingaraHttp(Transport transport, TokenCache tokens, Uri baseUrl, string? version)
    {
        this.transport = transport;
        this.tokens = tokens;
        this.baseUrl = baseUrl;
        this.version = version;
    }

    /// <summary>
    /// One <c>/v1/</c> call under a client-credentials token; the 2xx body's
    /// bytes, or the call's K3 refusal raised.
    /// </summary>
    internal async Task<byte[]> SendAsync(HttpMethod method, string path, byte[]? json, CancellationToken ct)
    {
        var url = new Uri(baseUrl.AbsoluteUri.TrimEnd('/') + path);
        var token = await tokens.GetAsync(ct).ConfigureAwait(false);
        var res = await Retry.SendAsync(transport, () => Request(method, url, json, token), null, ct).ConfigureAwait(false);
        if (res.StatusCode == HttpStatusCode.Unauthorized)
        {
            // K1: forget that token only if it is still the cached one, then once more.
            res.Dispose();
            tokens.Invalidate(token);
            token = await tokens.GetAsync(ct).ConfigureAwait(false);
            res = await Retry.SendAsync(transport, () => Request(method, url, json, token), null, ct).ConfigureAwait(false);
        }

        using (res)
        {
            var body = await res.Content.ReadAsByteArrayAsync(ct).ConfigureAwait(false);
            if (res.IsSuccessStatusCode)
            {
                return body;
            }

            throw Refusals.FromV1(res, body, transport.Time);
        }
    }

    private HttpRequestMessage Request(HttpMethod method, Uri url, byte[]? json, string token)
    {
        var req = new HttpRequestMessage(method, url);
        req.Headers.Accept.ParseAdd("application/json");
        req.Headers.TryAddWithoutValidation("User-Agent", transport.UserAgent);
        req.Headers.Authorization = new AuthenticationHeaderValue("Bearer", token);
        if (version is not null)
        {
            req.Headers.TryAddWithoutValidation("Lingara-Version", version);
        }

        if (json is not null)
        {
            req.Content = new ByteArrayContent(json);
            req.Content.Headers.ContentType = new MediaTypeHeaderValue("application/json");
        }

        return req;
    }
}

/// <summary>
/// Contract K4: a <c>429</c> or <c>503</c> carrying a <c>Retry-After</c> of at
/// most 60 s is sent again after exactly that wait, three attempts in all.
/// Nothing else is retried: no <c>Retry-After</c>, a longer one, any other
/// status, a transport failure. Decided on the status line and headers alone.
/// </summary>
internal static class Retry
{
    internal const int MaxAttempts = 3;
    internal static readonly TimeSpan Cap = TimeSpan.FromSeconds(60);

    /// <summary>
    /// The last response, its body buffered. <paramref name="attemptTimeout"/>
    /// bounds each attempt alone (never a <c>Retry-After</c> sleep), and its
    /// expiry is a <see cref="TimeoutException"/>.
    /// </summary>
    internal static async Task<HttpResponseMessage> SendAsync(Transport t, Func<HttpRequestMessage> build, TimeSpan? attemptTimeout, CancellationToken ct)
    {
        for (var tries = 1; ; tries++)
        {
            var res = await AttemptAsync(t, build, attemptTimeout, ct).ConfigureAwait(false);
            var wait = Wait(res, t.Time, tries);
            if (wait is null)
            {
                return res;
            }

            res.Dispose();
            await Task.Delay(wait.Value, t.Time, ct).ConfigureAwait(false);
        }
    }

    private static async Task<HttpResponseMessage> AttemptAsync(Transport t, Func<HttpRequestMessage> build, TimeSpan? timeout, CancellationToken ct)
    {
        using var req = build();
        using var deadline = timeout is { } limit ? new CancellationTokenSource(limit, t.Time) : null;
        using var linked = CancellationTokenSource.CreateLinkedTokenSource(ct, deadline?.Token ?? CancellationToken.None);
        try
        {
            return await t.Http.SendAsync(req, HttpCompletionOption.ResponseContentRead, linked.Token).ConfigureAwait(false);
        }
        catch (OperationCanceledException) when (deadline is { IsCancellationRequested: true } && !ct.IsCancellationRequested)
        {
            throw new TimeoutException("the token exchange timed out");
        }
    }

    private static TimeSpan? Wait(HttpResponseMessage res, TimeProvider time, int tries)
    {
        var status = res.StatusCode;
        if ((status != HttpStatusCode.TooManyRequests && status != HttpStatusCode.ServiceUnavailable) || tries >= MaxAttempts)
        {
            return null;
        }

        var wait = RetryAfter(res, time);
        return wait <= Cap ? wait : null;
    }

    /// <summary>Delta-seconds, or an HTTP-date as <c>max(0, date − now)</c> rounded up.</summary>
    internal static TimeSpan? RetryAfter(HttpResponseMessage res, TimeProvider time)
    {
        var header = res.Headers.RetryAfter;
        if (header?.Delta is TimeSpan delta)
        {
            return delta;
        }

        if (header?.Date is DateTimeOffset at)
        {
            var wait = at - time.GetUtcNow();
            return wait <= TimeSpan.Zero ? TimeSpan.Zero : TimeSpan.FromSeconds(Math.Ceiling(wait.TotalSeconds));
        }

        return null;
    }
}

/// <summary>Contract K3's mapping, in its precedence: a non-JSON 503 is maintenance first.</summary>
internal static class Refusals
{
    private const int MaintenanceBodyBytes = 1024;

    internal static LingaraException FromV1(HttpResponseMessage res, byte[] body, TimeProvider time)
    {
        var status = (int)res.StatusCode;
        var retryAfter = Retry.RetryAfter(res, time);
        if (Maintenance(res, body, retryAfter) is { } maintenance)
        {
            return maintenance;
        }

        var code = Field(body, "code");
        var message = Field(body, "error");
        return code is not null && message is not null
            ? new LingaraApiException(status, code, message, retryAfter)
            : new LingaraApiException(status, Http(status), "HTTP " + status.ToString(CultureInfo.InvariantCulture), retryAfter);
    }

    internal static LingaraException FromToken(HttpResponseMessage res, byte[] body, TimeProvider time)
    {
        var status = (int)res.StatusCode;
        var retryAfter = Retry.RetryAfter(res, time);
        if (Maintenance(res, body, retryAfter) is { } maintenance)
        {
            return maintenance;
        }

        var error = Field(body, "error");
        return error is not null
            ? new OAuthException(status, error, Field(body, "error_description"), retryAfter)
            : new OAuthException(status, Http(status), null, retryAfter);
    }

    private static MaintenanceException? Maintenance(HttpResponseMessage res, byte[] body, TimeSpan? retryAfter)
    {
        if (res.StatusCode != HttpStatusCode.ServiceUnavailable || IsJson(res))
        {
            return null;
        }

        var text = Encoding.UTF8.GetString(body, 0, Math.Min(body.Length, MaintenanceBodyBytes));
        return new MaintenanceException(text.TrimEnd('�'), retryAfter);
    }

    private static string Http(int status) => "http_" + status.ToString(CultureInfo.InvariantCulture);

    private static bool IsJson(HttpResponseMessage res)
    {
        var media = res.Content.Headers.ContentType?.MediaType ?? string.Empty;
        return media.Equals("application/json", StringComparison.OrdinalIgnoreCase)
            || media.EndsWith("+json", StringComparison.OrdinalIgnoreCase);
    }

    /// <summary>A string member of a JSON object body, or null for anything else.</summary>
    private static string? Field(byte[] body, string name)
    {
        try
        {
            using var doc = JsonDocument.Parse(body);
            return doc.RootElement.ValueKind == JsonValueKind.Object
                && doc.RootElement.TryGetProperty(name, out var v)
                && v.ValueKind == JsonValueKind.String
                ? v.GetString()
                : null;
        }
        catch (JsonException)
        {
            return null;
        }
    }
}

/// <summary>The kit's one version, the trio's <c>VERSION</c> (D1), stamped by the build.</summary>
internal static class KitVersion
{
    internal static readonly string Value = Read();

    /// <summary>Contract K6: <c>lingara-embed-dotnet/&lt;version&gt; (dotnet/&lt;runtime&gt;)</c>.</summary>
    internal static readonly string UserAgent = $"lingara-embed-dotnet/{Value} (dotnet/{Environment.Version})";

    private static string Read()
    {
        var informational = typeof(KitVersion).Assembly.GetCustomAttribute<AssemblyInformationalVersionAttribute>()?.InformationalVersion ?? "0.0.0";
        var plus = informational.IndexOf('+', StringComparison.Ordinal);
        return plus < 0 ? informational : informational[..plus];
    }
}
