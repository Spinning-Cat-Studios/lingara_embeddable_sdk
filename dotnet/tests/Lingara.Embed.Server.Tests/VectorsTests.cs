using System.Linq;
using System.Text.Json;
using Xunit;

namespace Lingara.Embed.Server.Tests;

public class VectorsTests
{
    private static string? Get(JsonElement o, string key) => o.TryGetProperty(key, out var v) ? v.GetString() : null;

    private static SnippetOptions Options(JsonElement o) => new()
    {
        PlanId = Get(o, "planId") ?? string.Empty,
        TokenEndpoint = Get(o, "tokenEndpoint") ?? string.Empty,
        Surface = Get(o, "surface"),
        Theme = Get(o, "theme"),
        Locale = Get(o, "locale"),
        LingaraOrigin = Get(o, "lingaraOrigin"),
        Title = Get(o, "title"),
        Id = Get(o, "id"),
        Nonce = Get(o, "nonce"),
        ElementSrc = Get(o, "elementSrc"),
    };

    /// <summary><c>input.update_available</c> decoded to the typed value, never forwarded as a raw map.</summary>
    private static UpdateAvailable? Notice(JsonElement input) =>
        input.TryGetProperty("update_available", out var n)
            ? new UpdateAvailable(Get(n, "latest"), n.GetProperty("deprecated").GetBoolean(), Get(n, "message"))
            : null;

    /// <summary>
    /// 1.10.26af AC13: every <c>snippet.json</c> vector through
    /// <c>WidgetSnippet.Render</c>, and every <c>token-handoff.json</c> vector
    /// through the internal <c>HandoffBody</c>, gives its expected answer;
    /// since 31.10.26f the three <c>update_available</c> vectors too.
    /// </summary>
    // 31.10.26f AC4
    [Fact]
    public void EverySnippetAndHandoffVector()
    {
        var cases = Repo.Vectors("snippet.json").GetProperty("cases").EnumerateArray().ToList();
        Assert.True(cases.Count >= 40, "the snippet vectors are all here");
        foreach (var c in cases)
        {
            var name = c.GetProperty("name").GetString();
            var expect = c.GetProperty("expect");
            if (expect.TryGetProperty("html", out var html))
            {
                var want = html.GetString()!.Replace("{{version}}", Repo.Version, System.StringComparison.Ordinal);
                Assert.True(want == WidgetSnippet.Render(Options(c.GetProperty("options"))), name);
            }
            else
            {
                var refused = Assert.Throws<SnippetException>(() => WidgetSnippet.Render(Options(c.GetProperty("options"))));
                Assert.True(expect.GetProperty("refused").GetString() == refused.Field, $"{name}: refused {refused.Field}");
            }
        }

        var handoffs = Repo.Vectors("token-handoff.json").EnumerateArray().ToList();
        Assert.True(handoffs.Count(c => c.GetProperty("input").TryGetProperty("update_available", out _)) >= 3, "the update_available vectors are all here");
        foreach (var c in handoffs)
        {
            var input = c.GetProperty("input");
            var got = TokenHandoff.HandoffBody(input.GetProperty("token").GetString()!, input.GetProperty("expires_at").GetString()!, Notice(input));
            Assert.True(c.GetProperty("body").GetString() == got, $"{c.GetProperty("name").GetString()}: {got}");
        }
    }

    /// <summary>
    /// 31.10.26f D1: the mint's <c>update_available</c> is read leniently: a
    /// well-formed notice is kept, and a malformed one is dropped while the
    /// token it rode is kept.
    /// </summary>
    [Fact]
    public void AMalformedNoticeIsDroppedAndTheTokenKept()
    {
        const string Answer = "{\"token\":\"lgr_et_Ab3dEf6hIj9lMn2pQr5tUv8xYz1bCd4fGh7jKl0nPq3\",\"expires_at\":\"2026-10-01T09:27:44Z\","
            + "\"expires_in\":900,\"subject\":\"lgr_sub_x\",\"scopes\":[\"embed:play\"],\"account_linked\":false";
        static MintedToken Mint(string notice) => MintedToken.FromAnswer(System.Text.Encoding.UTF8.GetBytes(Answer + notice + "}"));

        Assert.Equal(new UpdateAvailable(null, true, "Withdrawn"), Mint(",\"update_available\":{\"latest\":null,\"deprecated\":true,\"message\":\"Withdrawn\",\"extra\":1}").UpdateAvailable);
        Assert.Equal(new UpdateAvailable(null, false, new string('x', 280)), Mint(",\"update_available\":{\"latest\":null,\"deprecated\":false,\"message\":\"" + new string('x', 280) + "\"}").UpdateAvailable);
        Assert.Null(Mint(string.Empty).UpdateAvailable);
        string[] malformed =
        [
            "null",
            "[]",
            "\"1.2.0\"",
            "{\"latest\":\"1.2.0\",\"message\":null}",
            "{\"latest\":\"1.2.0\",\"deprecated\":\"yes\",\"message\":null}",
            "{\"latest\":1,\"deprecated\":false,\"message\":null}",
            "{\"latest\":\"1.2.0\",\"deprecated\":false,\"message\":{}}",
            "{\"latest\":\"1.2.0\",\"deprecated\":false,\"message\":\"" + new string('x', 281) + "\"}",
        ];
        foreach (var notice in malformed)
        {
            var minted = Mint(",\"update_available\":" + notice);
            Assert.True(minted.UpdateAvailable is null, notice);
            Assert.StartsWith("lgr_et_", minted.ExposeToken(), System.StringComparison.Ordinal);
            Assert.DoesNotContain("lgr_et_", minted.ToString(), System.StringComparison.Ordinal);
        }
    }

    /// <summary>31.10.26f D1: the header read case-insensitively by the caller's lookup, trimmed; blank is none.</summary>
    [Fact]
    public void EmbedVersionFromTrimsAndDropsBlank()
    {
        static System.Func<string, string?> Headers(string? value) => name =>
            string.Equals(name, "lingara-embed-version", System.StringComparison.OrdinalIgnoreCase) ? value : null;

        Assert.Equal("2.0.1", EmbedServer.EmbedVersionFrom(Headers("  2.0.1 ")));
        Assert.Null(EmbedServer.EmbedVersionFrom(Headers("   ")));
        Assert.Null(EmbedServer.EmbedVersionFrom(Headers(null)));
    }
}
