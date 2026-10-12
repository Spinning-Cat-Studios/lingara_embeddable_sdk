using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.Net.Http;
using System.Text.Json;

namespace Lingara.Embed.Server;

/// <summary>
/// A player's embed token, as <see cref="EmbedServer.MintTokenAsync"/>
/// returns it: B1's six fields (ADR 1.10.26af D10). The token has no public
/// property, so no serialiser, debugger or log line sees it:
/// <see cref="ExposeToken"/> is the one accessor, and
/// <see cref="TokenHandoff.Body"/> the one place a kit uses it.
/// </summary>
[DebuggerDisplay("MintedToken {{ Token = [REDACTED], Subject = {Subject} }}")]
public sealed class MintedToken
{
    private const string Prefix = "lgr_et_";

    private readonly string token;

    private MintedToken(string token, string expiresAtText, DateTimeOffset expiresAt, TimeSpan expiresIn, string subject, IReadOnlyList<string> scopes, bool accountLinked)
    {
        this.token = token;
        ExpiresAtText = expiresAtText;
        ExpiresAt = expiresAt;
        ExpiresIn = expiresIn;
        Subject = subject;
        Scopes = scopes;
        AccountLinked = accountLinked;
    }

    /// <summary>When the token stops working. Lingara never refreshes it: mint again.</summary>
    public DateTimeOffset ExpiresAt { get; }

    /// <summary><c>expires_at</c> exactly as the server wrote it, which the handoff copies byte for byte.</summary>
    public string ExpiresAtText { get; }

    /// <summary>The token's lifetime, counted from the answer.</summary>
    public TimeSpan ExpiresIn { get; }

    /// <summary>The player's pairwise <c>lgr_sub_</c>, stable across mints: how an event names them.</summary>
    public string Subject { get; }

    /// <summary>The scopes granted.</summary>
    public IReadOnlyList<string> Scopes { get; }

    /// <summary>Whether the player has linked a Lingara account.</summary>
    public bool AccountLinked { get; }

    /// <summary>The raw <c>lgr_et_</c> token. The one accessor that does not redact.</summary>
    public string ExposeToken() => token;

    public override string ToString() =>
        $"MintedToken {{ Token = [REDACTED], ExpiresAt = {ExpiresAtText}, Subject = {Subject}, AccountLinked = {AccountLinked} }}";

    /// <summary>
    /// The builder: refuses an answer missing any of the six fields, or whose
    /// token is not an <c>lgr_et_</c> token, with a message carrying none of it.
    /// </summary>
    internal static MintedToken FromAnswer(ReadOnlySpan<byte> json)
    {
        try
        {
            using var doc = JsonDocument.Parse(json.ToArray());
            var root = doc.RootElement;
            var token = root.GetProperty("token").GetString()!;
            var expiresAt = root.GetProperty("expires_at").GetString()!;
            var expiresIn = root.GetProperty("expires_in").GetInt64();
            var subject = root.GetProperty("subject").GetString()!;
            var scopes = Strings(root.GetProperty("scopes"));
            var linked = root.GetProperty("account_linked").GetBoolean();
            if (token.StartsWith(Prefix, StringComparison.Ordinal) && Instant(expiresAt) is { } at && expiresIn >= 0)
            {
                return new MintedToken(token, expiresAt, at, TimeSpan.FromSeconds(expiresIn), subject, scopes, linked);
            }
        }
        catch (Exception e) when (e is JsonException or KeyNotFoundException or InvalidOperationException or FormatException)
        {
            // Falls through: the answer is not B1's, and the cause may hold the token.
        }

        throw new HttpRequestException(HttpRequestError.InvalidResponse, "the mint's answer is not an embed token");
    }

    private static string[] Strings(JsonElement array)
    {
        var items = new string[array.GetArrayLength()];
        var i = 0;
        foreach (var item in array.EnumerateArray())
        {
            items[i++] = item.GetString()!;
        }

        return items;
    }

    /// <summary>RFC 3339, either case of <c>T</c> and <c>Z</c>.</summary>
    private static DateTimeOffset? Instant(string text) =>
        DateTimeOffset.TryParse(text.ToUpperInvariant(), CultureInfo.InvariantCulture, DateTimeStyles.AssumeUniversal, out var at) ? at : null;
}
