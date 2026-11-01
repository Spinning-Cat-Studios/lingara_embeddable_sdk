using System;
using System.Globalization;
using System.Text;

namespace Lingara.Embed.Server;

/// <summary>
/// The handoff body (ADR 1.10.26af D6, ES6): exactly
/// <c>{"token":"lgr_et_…","expires_at":"…"}</c>, the shape every player kit
/// and the widget read. This is the one place a kit exposes the token on purpose.
/// </summary>
public static class TokenHandoff
{
    /// <summary>
    /// The JSON your token route returns to the player. <c>expires_at</c> is
    /// the server's string, copied byte for byte; <c>expires_in</c> and
    /// <c>subject</c> are not forwarded. The mint's
    /// <see cref="MintedToken.UpdateAvailable"/> is forwarded as a third key,
    /// <c>update_available</c>, when present (ADR 31.10.26f).
    /// </summary>
    public static string Body(MintedToken minted)
    {
        ArgumentNullException.ThrowIfNull(minted);
        return HandoffBody(minted.ExposeToken(), minted.ExpiresAtText, minted.UpdateAvailable);
    }

    /// <summary>
    /// The pure body, held to <c>token-handoff.json</c>: compact, <c>token</c>
    /// then <c>expires_at</c>, then <c>update_available</c> only when given
    /// (its keys <c>latest</c>, <c>deprecated</c>, <c>message</c>, nulls kept);
    /// <c>/</c> and non-ASCII unescaped.
    /// </summary>
    internal static string HandoffBody(string token, string expiresAt, UpdateAvailable? updateAvailable = null)
    {
        var notice = updateAvailable is { } n
            ? ",\"update_available\":{\"latest\":" + Nullable(n.Latest) + ",\"deprecated\":" + (n.Deprecated ? "true" : "false") + ",\"message\":" + Nullable(n.Message) + "}"
            : string.Empty;
        return "{\"token\":" + JsonString(token) + ",\"expires_at\":" + JsonString(expiresAt) + notice + "}";
    }

    private static string Nullable(string? value) => value is null ? "null" : JsonString(value);

    /// <summary>A JSON string escaping only what JSON requires: <c>"</c>, <c>\</c> and U+0000–U+001F.</summary>
    private static string JsonString(string value)
    {
        var sb = new StringBuilder("\"", value.Length + 2);
        foreach (var c in value)
        {
            if (c < ' ')
            {
                sb.Append(Control(c));
            }
            else
            {
                sb.Append(c is '"' or '\\' ? "\\" + c : c.ToString());
            }
        }

        return sb.Append('"').ToString();
    }

    /// <summary>serde_json's spelling: the five short escapes, else lower-case <c>\u00xx</c>.</summary>
    private static string Control(char c) => c switch
    {
        '\n' => "\\n",
        '\r' => "\\r",
        '\t' => "\\t",
        '\b' => "\\b",
        '\f' => "\\f",
        _ => "\\u" + ((int)c).ToString("x4", CultureInfo.InvariantCulture),
    };
}
