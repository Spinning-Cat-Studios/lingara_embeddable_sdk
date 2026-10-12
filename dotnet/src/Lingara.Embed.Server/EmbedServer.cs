using System;
using System.Collections.Generic;
using System.IO;
using System.Net.Http;
using System.Text;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;
using Lingara.Embed.Server.Http;

namespace Lingara.Embed.Server;

/// <summary>
/// Your client credentials and where Lingara is. The secret is rendered
/// <c>[REDACTED]</c> by <see cref="ToString"/>.
/// </summary>
public sealed class EmbedServerOptions
{
    /// <summary>Your client id, <c>lgr_cid_…</c>.</summary>
    public required string ClientId { get; init; }

    /// <summary>Your client secret, <c>lgr_cs_…</c>. Read it from your secret store, never from source.</summary>
    public required string ClientSecret { get; init; }

    /// <summary>The API's origin. Default <c>https://api.getlingara.com</c>.</summary>
    public Uri BaseUrl { get; init; } = new("https://api.getlingara.com");

    /// <summary>The token endpoint. Default <see cref="BaseUrl"/>'s <c>/oauth/token</c>.</summary>
    public Uri? TokenUrl { get; init; }

    /// <summary>Scopes to ask the token endpoint for; none asks for every scope the client holds.</summary>
    public IReadOnlyList<string>? Scopes { get; init; }

    /// <summary>A <c>Lingara-Version</c> pin, sent on <c>/v1/</c> only. None uses your client's server-side pin.</summary>
    public string? LingaraVersion { get; init; }

    /// <summary>The clock and the <c>Retry-After</c> sleeper, a testing seam. Default the system's.</summary>
    public TimeProvider TimeProvider { get; init; } = TimeProvider.System;

    public override string ToString() => $"EmbedServerOptions {{ ClientId = {ClientId}, ClientSecret = [REDACTED], BaseUrl = {BaseUrl} }}";
}

/// <summary>What a mint may send beside the player: each sent only when given.</summary>
public sealed class MintOptions
{
    /// <summary>A subset of the handable scopes; none grants every one the client holds.</summary>
    public IReadOnlyList<string>? Scopes { get; init; }

    /// <summary>Your page's origin, one of your client's embed origins. A game's server mints without one.</summary>
    public string? Origin { get; init; }
}

/// <summary>
/// Mint and delete (ADR 1.10.26af D4, D9): one call each, the directory
/// written only after a success. There is no cache: every mint is one
/// <c>POST</c>, so a token a delete revoked on another instance is never
/// handed out again. A failed call raises its <see cref="LingaraException"/>
/// unchanged.
/// </summary>
public sealed class EmbedServer : IDisposable
{
    private static readonly TimeSpan TokenRequestTimeout = TimeSpan.FromSeconds(30);

    private readonly EmbedServerOptions options;
    private readonly IPlayerDirectory? directory;
    private readonly HttpClient? owned;
    private readonly TokenCache tokens;
    private readonly LingaraHttp http;

    /// <param name="options">Your client credentials.</param>
    /// <param name="directory">Records each mint's subject and forgets a deleted player; optional.</param>
    /// <param name="http">Your <see cref="HttpClient"/>, or null for one the server owns and disposes.</param>
    public EmbedServer(EmbedServerOptions options, IPlayerDirectory? directory = null, HttpClient? http = null)
    {
        ArgumentNullException.ThrowIfNull(options);
        this.options = options;
        this.directory = directory;
        owned = http is null ? new HttpClient() : null;
        var transport = new Transport(http ?? owned!, KitVersion.UserAgent, options.TimeProvider);
        tokens = new TokenCache(transport, options.TokenUrl ?? new Uri(options.BaseUrl, "/oauth/token"), options.ClientId, options.ClientSecret, options.Scopes, TokenRequestTimeout);
        this.http = new LingaraHttp(transport, tokens, options.BaseUrl, options.LingaraVersion);
    }

    /// <summary>
    /// Mints a token for one player: one <c>POST /v1/embed/tokens</c>, then
    /// <see cref="IPlayerDirectory.RememberAsync"/>. On a failure nothing is remembered.
    /// </summary>
    public async Task<MintedToken> MintTokenAsync(string playerRef, MintOptions? options = null, CancellationToken ct = default)
    {
        var body = await http.SendAsync(HttpMethod.Post, "/v1/embed/tokens", MintBody(playerRef, options), ct).ConfigureAwait(false);
        var minted = MintedToken.FromAnswer(body);
        if (directory is not null)
        {
            await directory.RememberAsync(playerRef, minted.Subject, ct).ConfigureAwait(false);
        }

        return minted;
    }

    /// <summary>
    /// Deletes a player and revokes their tokens, then forgets them. An
    /// unknown player is a success; on a failure the row is kept.
    /// </summary>
    public async Task DeletePlayerAsync(string playerRef, CancellationToken ct = default)
    {
        ArgumentNullException.ThrowIfNull(playerRef);
        await http.SendAsync(HttpMethod.Delete, "/v1/embed/players/" + EncodeSegment(playerRef), null, ct).ConfigureAwait(false);
        if (directory is not null)
        {
            await directory.ForgetAsync(playerRef, ct).ConfigureAwait(false);
        }
    }

    public void Dispose()
    {
        tokens.Dispose();
        owned?.Dispose();
    }

    public override string ToString() => $"EmbedServer {{ ClientId = {options.ClientId}, ClientSecret = [REDACTED], Directory = {directory is not null} }}";

    /// <summary><c>{player_ref, scopes?, origin?}</c>, the options present only when given.</summary>
    private static byte[] MintBody(string playerRef, MintOptions? options)
    {
        ArgumentNullException.ThrowIfNull(playerRef);
        var bytes = Encoding.UTF8.GetByteCount(playerRef);
        if (bytes is < 1 or > 128)
        {
            throw new ArgumentException("player_ref must be 1–128 bytes of UTF-8", nameof(playerRef));
        }

        using var buffer = new MemoryStream();
        using (var w = new Utf8JsonWriter(buffer))
        {
            w.WriteStartObject();
            w.WriteString("player_ref", playerRef);
            if (options?.Scopes is { } scopes)
            {
                w.WriteStartArray("scopes");
                foreach (var scope in scopes)
                {
                    w.WriteStringValue(scope);
                }

                w.WriteEndArray();
            }

            if (options?.Origin is { } origin)
            {
                w.WriteString("origin", origin);
            }

            w.WriteEndObject();
        }

        return buffer.ToArray();
    }

    /// <summary>
    /// One path segment: UTF-8, every byte outside RFC 3986's unreserved set
    /// as upper-case <c>%XX</c>, so <c>guild/42 (é)*</c> is <c>guild%2F42%20%28%C3%A9%29%2A</c>.
    /// </summary>
    internal static string EncodeSegment(string segment)
    {
        var sb = new StringBuilder();
        foreach (var b in Encoding.UTF8.GetBytes(segment))
        {
            if (char.IsAsciiLetterOrDigit((char)b) || b == (byte)'-' || b == (byte)'.' || b == (byte)'_' || b == (byte)'~')
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
}
