using System;
using System.Collections.Generic;
using System.Net.Http;
using System.Net.Http.Headers;
using System.Text;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;

namespace Lingara.Embed.Server.Http;

/// <summary>
/// Contract K1's client-credentials source (ADR 1.10.26af D10): one cached
/// token, stale at <c>obtained_at + expires_in − min(60 s, expires_in / 2)</c>;
/// one exchange shared by every concurrent caller (the flight is started and
/// cleared under one <see cref="SemaphoreSlim"/>, and a caller's cancellation
/// abandons only its own wait); compare-and-clear on a 401; and a 30 s
/// deadline on each exchange attempt, since no caller's cancellation reaches it.
/// </summary>
internal sealed class TokenCache : IDisposable
{
    private static readonly TimeSpan MaxSkew = TimeSpan.FromSeconds(60);

    private readonly Transport transport;
    private readonly Uri tokenUrl;
    private readonly string clientId;
    private readonly string clientSecret;
    private readonly IReadOnlyList<string>? scopes;
    private readonly TimeSpan requestTimeout;
    private readonly SemaphoreSlim gate = new(1, 1);
    private Cached? cached;
    private Task<string>? flight;

    internal TokenCache(Transport transport, Uri tokenUrl, string clientId, string clientSecret, IReadOnlyList<string>? scopes, TimeSpan requestTimeout)
    {
        this.transport = transport;
        this.tokenUrl = tokenUrl;
        this.clientId = clientId;
        this.clientSecret = clientSecret;
        this.scopes = scopes;
        this.requestTimeout = requestTimeout;
    }

    /// <summary>The cached token while fresh; otherwise the one shared exchange's.</summary>
    internal async Task<string> GetAsync(CancellationToken ct)
    {
        Task<string> shared;
        await gate.WaitAsync(ct).ConfigureAwait(false);
        try
        {
            if (cached is { } c && transport.Time.GetUtcNow() < c.StaleAt)
            {
                return c.Token;
            }

            // Stored under the gate the flight must take to clear it, so it can
            // never be cleared before it is filled.
            shared = flight ??= FlyAsync();
        }
        finally
        {
            gate.Release();
        }

        return await shared.WaitAsync(ct).ConfigureAwait(false);
    }

    /// <summary>Forgets <paramref name="token"/> only if it is still the cached one.</summary>
    internal void Invalidate(string token)
    {
        gate.Wait();
        try
        {
            if (cached is { } c && string.Equals(c.Token, token, StringComparison.Ordinal))
            {
                cached = null;
            }
        }
        finally
        {
            gate.Release();
        }
    }

    private async Task<string> FlyAsync()
    {
        Cached? got = null;
        try
        {
            got = await ExchangeAsync().ConfigureAwait(false);
            return got.Token;
        }
        finally
        {
            await gate.WaitAsync().ConfigureAwait(false);
            flight = null;
            cached = got ?? cached;
            gate.Release();
        }
    }

    private async Task<Cached> ExchangeAsync()
    {
        // `obtained_at` is when the request that succeeded was sent.
        var sentAt = transport.Time.GetUtcNow();
        var attempt = () =>
        {
            sentAt = transport.Time.GetUtcNow();
            return Request();
        };
        using var res = await Retry.SendAsync(transport, attempt, requestTimeout, CancellationToken.None).ConfigureAwait(false);
        var body = await res.Content.ReadAsByteArrayAsync().ConfigureAwait(false);
        if (!res.IsSuccessStatusCode)
        {
            throw Refusals.FromToken(res, body, transport.Time);
        }

        var (token, expiresIn) = Grant(body);
        var skew = expiresIn / 2 < MaxSkew ? expiresIn / 2 : MaxSkew;
        return new Cached(token, sentAt + expiresIn - skew);
    }

    /// <summary>
    /// <c>client_secret_basic</c> with form-encoded halves (RFC 6749 §2.3.1);
    /// the body carries the grant and, only when configured, <c>scope</c>.
    /// </summary>
    private HttpRequestMessage Request()
    {
        var form = "grant_type=client_credentials";
        if (scopes is { Count: > 0 })
        {
            form += "&scope=" + FormEncode(string.Join(' ', scopes));
        }

        var req = new HttpRequestMessage(HttpMethod.Post, tokenUrl)
        {
            Content = new StringContent(form, Encoding.ASCII),
        };
        req.Content.Headers.ContentType = new MediaTypeHeaderValue("application/x-www-form-urlencoded");
        req.Headers.Accept.ParseAdd("application/json");
        req.Headers.TryAddWithoutValidation("User-Agent", transport.UserAgent);
        var pair = FormEncode(clientId) + ":" + FormEncode(clientSecret);
        req.Headers.Authorization = new AuthenticationHeaderValue("Basic", Convert.ToBase64String(Encoding.UTF8.GetBytes(pair)));
        return req;
    }

    /// <summary>
    /// A <c>200</c>'s token and lifetime: an <c>access_token</c>, an
    /// <c>expires_in</c> and a <c>Bearer</c> <c>token_type</c>, or the
    /// response is malformed.
    /// </summary>
    private static (string Token, TimeSpan ExpiresIn) Grant(byte[] body)
    {
        try
        {
            using var doc = JsonDocument.Parse(body);
            var root = doc.RootElement;
            if (root.ValueKind == JsonValueKind.Object
                && root.TryGetProperty("access_token", out var token) && token.ValueKind == JsonValueKind.String
                && root.TryGetProperty("expires_in", out var seconds) && seconds.TryGetDouble(out var s) && s >= 0
                && root.TryGetProperty("token_type", out var type) && string.Equals(type.GetString(), "bearer", StringComparison.OrdinalIgnoreCase))
            {
                return (token.GetString()!, TimeSpan.FromSeconds(s));
            }
        }
        catch (JsonException)
        {
            // Falls through to the malformed answer below.
        }
        catch (InvalidOperationException)
        {
            // token_type was not a string.
        }

        throw new HttpRequestException(HttpRequestError.InvalidResponse, "the token endpoint's answer is malformed");
    }

    /// <summary>The WHATWG form serializer: space as <c>+</c>; <c>*-._</c> and alphanumerics kept.</summary>
    internal static string FormEncode(string value)
    {
        var sb = new StringBuilder();
        foreach (var b in Encoding.UTF8.GetBytes(value))
        {
            if (b == (byte)' ')
            {
                sb.Append('+');
            }
            else if (char.IsAsciiLetterOrDigit((char)b) || b == (byte)'*' || b == (byte)'-' || b == (byte)'.' || b == (byte)'_')
            {
                sb.Append((char)b);
            }
            else
            {
                sb.Append('%').Append(b.ToString("X2", System.Globalization.CultureInfo.InvariantCulture));
            }
        }

        return sb.ToString();
    }

    public void Dispose() => gate.Dispose();

    public override string ToString() => $"TokenCache {{ ClientId = {clientId}, ClientSecret = [REDACTED], Token = [REDACTED] }}";

    private sealed record Cached(string Token, DateTimeOffset StaleAt)
    {
        public override string ToString() => "Cached { Token = [REDACTED] }";
    }
}
