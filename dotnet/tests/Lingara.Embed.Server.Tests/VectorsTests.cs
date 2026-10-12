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

    /// <summary>
    /// 1.10.26af AC13: every <c>snippet.json</c> vector through
    /// <c>WidgetSnippet.Render</c>, and every <c>token-handoff.json</c> vector
    /// through the internal <c>HandoffBody</c>, gives its expected answer.
    /// </summary>
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

        foreach (var c in Repo.Vectors("token-handoff.json").EnumerateArray())
        {
            var input = c.GetProperty("input");
            var got = TokenHandoff.HandoffBody(input.GetProperty("token").GetString()!, input.GetProperty("expires_at").GetString()!);
            Assert.True(c.GetProperty("body").GetString() == got, c.GetProperty("name").GetString());
        }
    }
}
