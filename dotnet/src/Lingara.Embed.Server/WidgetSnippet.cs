using System;
using System.Linq;
using System.Text;
using System.Text.RegularExpressions;
using Lingara.Embed.Server.Http;

namespace Lingara.Embed.Server;

/// <summary>
/// What <see cref="WidgetSnippet.Render"/> renders. <see cref="PlanId"/> and
/// <see cref="TokenEndpoint"/> are required; every other option is written
/// only when set.
/// </summary>
public sealed class SnippetOptions
{
    /// <summary>The lesson plan your server made for this player, a UUID.</summary>
    public required string PlanId { get; init; }

    /// <summary>
    /// Your token route: a path (<c>/lingara/token</c>) or an <c>https://</c>
    /// URL. The element <c>POST</c>s to it with the page's cookies and reads
    /// <see cref="TokenHandoff.Body"/>'s JSON.
    /// </summary>
    public required string TokenEndpoint { get; init; }

    /// <summary>Default <c>practice</c>.</summary>
    public string? Surface { get; init; }

    /// <summary><c>light</c>, <c>dark</c> or <c>auto</c>.</summary>
    public string? Theme { get; init; }

    public string? Locale { get; init; }

    public string? LingaraOrigin { get; init; }

    public string? Title { get; init; }

    /// <summary>Default <c>lingara-embed</c>; two widgets on one page need two ids.</summary>
    public string? Id { get; init; }

    /// <summary>Your page's CSP nonce, written on both scripts. A strict CSP blocks the inline script without it.</summary>
    public string? Nonce { get; init; }

    /// <summary>Default <see cref="WidgetSnippet.DefaultElementSrc"/>; an override must be <c>https://</c>.</summary>
    public string? ElementSrc { get; init; }
}

/// <summary>The first refused option, by its contract name (<c>planId</c>, <c>surface</c>, …). It never holds the value.</summary>
public sealed class SnippetException : ArgumentException
{
    internal SnippetException(string field)
        : base($"the widget snippet's {field} is not valid")
    {
        Field = field;
    }

    public string Field { get; }
}

/// <summary>
/// The widget snippet (ADR 1.10.26af D7, ES4): the <c>&lt;lingara-embed&gt;</c>
/// element and its two module scripts, rendered identically by all eight
/// server kits. The token never appears in it, only the endpoint the element
/// asks at run time, with <c>credentials: "same-origin"</c>.
/// </summary>
public static class WidgetSnippet
{
    /// <summary>The script-tag build of this release's element.</summary>
    public static readonly string DefaultElementSrc = $"https://cdn.jsdelivr.net/npm/@lingara/embed@{KitVersion.Value}/dist/lingara-embed.min.js";

    /// <summary>The three lines to paste into your page, with no trailing newline.</summary>
    /// <exception cref="SnippetException">Naming the first refused option.</exception>
    public static string Render(SnippetOptions options)
    {
        ArgumentNullException.ThrowIfNull(options);
        var surface = options.Surface ?? "practice";
        var id = options.Id ?? "lingara-embed";
        var src = options.ElementSrc ?? DefaultElementSrc;
        if (SnippetRules.FirstRefused(options, surface, id) is { } field)
        {
            throw new SnippetException(field);
        }

        var element = new StringBuilder($"<lingara-embed id=\"{Attr(id)}\" surface=\"{Attr(surface)}\" plan-id=\"{Attr(options.PlanId)}\"");
        foreach (var (name, value) in new[] { ("theme", options.Theme), ("locale", options.Locale), ("lingara-origin", options.LingaraOrigin), ("title", options.Title) })
        {
            if (value is not null)
            {
                element.Append(' ').Append(name).Append("=\"").Append(Attr(value)).Append('"');
            }
        }

        var nonce = options.Nonce is null ? string.Empty : $" nonce=\"{Attr(options.Nonce)}\"";
        return $"{element}></lingara-embed>\n"
            + $"<script type=\"module\"{nonce} src=\"{Attr(src)}\"></script>\n"
            + $"<script type=\"module\"{nonce}>document.getElementById({ScriptString(id)}).tokenProvider = async () => (await fetch({ScriptString(options.TokenEndpoint)}, {{ method: \"POST\", credentials: \"same-origin\" }})).json();</script>";
    }

    /// <summary>An attribute value: the five characters, and nothing else.</summary>
    private static string Attr(string value)
    {
        var sb = new StringBuilder(value.Length);
        foreach (var c in value)
        {
            sb.Append(c switch
            {
                '&' => "&amp;",
                '<' => "&lt;",
                '>' => "&gt;",
                '"' => "&quot;",
                '\'' => "&#39;",
                _ => c.ToString(),
            });
        }

        return sb.ToString();
    }

    /// <summary>A JSON string literal that can never close the script or open a comment.</summary>
    private static string ScriptString(string value)
    {
        var sb = new StringBuilder("\"", value.Length + 2);
        foreach (var c in value)
        {
            sb.Append(c switch
            {
                '"' => "\\\"",
                '\\' => "\\\\",
                '<' => "\\u003c",
                '>' => "\\u003e",
                '&' => "\\u0026",
                '\u2028' => "\\u2028",
                '\u2029' => "\\u2029",
                _ => c.ToString(),
            });
        }

        return sb.Append('"').ToString();
    }
}

/// <summary>ES4's option rules, one per row of the contract's table. Every pattern is ASCII-only.</summary>
internal static partial class SnippetRules
{
    /// <summary>The first option refused, in the contract's order, or null.</summary>
    internal static string? FirstRefused(SnippetOptions o, string surface, string id)
    {
        var rules = new (string Field, bool Ok)[]
        {
            ("planId", o.PlanId is not null && PlanId().IsMatch(o.PlanId)),
            ("surface", Surface().IsMatch(surface)),
            ("theme", o.Theme is null or "light" or "dark" or "auto"),
            ("locale", o.Locale is null || Locale().IsMatch(o.Locale)),
            ("lingaraOrigin", o.LingaraOrigin is null || Origin().IsMatch(o.LingaraOrigin)),
            ("title", o.Title is null || Title(o.Title)),
            ("id", Id().IsMatch(id)),
            ("tokenEndpoint", o.TokenEndpoint is not null && TokenEndpoint(o.TokenEndpoint)),
            ("nonce", o.Nonce is null || Nonce().IsMatch(o.Nonce)),
            ("elementSrc", o.ElementSrc is null || (o.ElementSrc.StartsWith("https://", StringComparison.Ordinal) && UrlText(o.ElementSrc))),
        };
        return rules.FirstOrDefault(r => !r.Ok).Field;
    }

    [GeneratedRegex(@"^[0-9A-Fa-f]{8}(-[0-9A-Fa-f]{4}){3}-[0-9A-Fa-f]{12}\z", RegexOptions.CultureInvariant)]
    private static partial Regex PlanId();

    [GeneratedRegex(@"^[a-z][a-z0-9-]{0,31}\z", RegexOptions.CultureInvariant)]
    private static partial Regex Surface();

    [GeneratedRegex(@"^[A-Za-z]{2,3}(-[A-Za-z0-9]{2,8}){0,3}\z", RegexOptions.CultureInvariant)]
    private static partial Regex Locale();

    [GeneratedRegex(@"^(https://[A-Za-z0-9-]+(\.[A-Za-z0-9-]+)*(:[0-9]{1,5})?|http://(localhost|127\.0\.0\.1)(:[0-9]{1,5})?)/?\z", RegexOptions.CultureInvariant)]
    private static partial Regex Origin();

    [GeneratedRegex(@"^[A-Za-z][A-Za-z0-9_-]{0,63}\z", RegexOptions.CultureInvariant)]
    private static partial Regex Id();

    [GeneratedRegex(@"^[A-Za-z0-9+/=_-]{8,128}\z", RegexOptions.CultureInvariant)]
    private static partial Regex Nonce();

    /// <summary>1–80 Unicode scalar values (never UTF-16 units), none a control character.</summary>
    private static bool Title(string s)
    {
        var scalars = s.EnumerateRunes().Count();
        return scalars is >= 1 and <= 80 && !s.Any(IsControl);
    }

    /// <summary>A path that starts <c>/</c> but not <c>//</c>, or an absolute <c>https://</c> URL.</summary>
    private static bool TokenEndpoint(string s) =>
        ((s.StartsWith('/') && !s.StartsWith("//", StringComparison.Ordinal)) || s.StartsWith("https://", StringComparison.Ordinal)) && UrlText(s);

    /// <summary>At most 2048 UTF-8 bytes, with no control character or whitespace.</summary>
    private static bool UrlText(string s) => Encoding.UTF8.GetByteCount(s) <= 2048 && !s.Any(c => IsControl(c) || IsSpace(c));

    /// <summary>Unicode category Cc.</summary>
    private static bool IsControl(char c) => c <= '\u001f' || (c >= '\u007f' && c <= '\u009f');

    /// <summary>The contract's whitespace set.</summary>
    private static bool IsSpace(char c) =>
        c is ' ' or '\u0085' or '\u00a0' or '\u1680' or '\u2028' or '\u2029' or '\u202f' or '\u205f' or '\u3000'
        || (c >= '\u2000' && c <= '\u200a');
}
