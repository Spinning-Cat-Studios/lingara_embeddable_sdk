using System;
using System.Globalization;
using System.Text;
using Lingara.Embed.Dialogues;
using Lingara.Embed.Json;
using Xunit;

namespace Lingara.Embed.Tests
{
    /// <summary>The C# canonical writer (ADR 28.10.26g D2) against the shared fixtures.</summary>
    public class CanonicalJsonTests
    {
        private static string Quoted(string text)
        {
            var sb = new StringBuilder();
            CanonicalJson.Quote(sb, text);
            return sb.ToString();
        }

        // 28.10.26g AC6
        [Fact]
        public void TheCorpusIntegralsSurrogatesAndTheComparisonForm()
        {
            var strings = DialogueBuilderTests.ParsedVector("canonical-strings.json");
            Assert.NotEmpty(strings["strings"].Items);
            foreach (var pair in strings["strings"].Items)
            {
                Assert.Equal(pair.Str("canonical"), Quoted(pair.Str("input")));
            }

            foreach (var pair in strings["keys"].Items)
            {
                Assert.Equal(pair.Str("canonical"), CanonicalJson.Write(pair["input"]));
            }

            var numbers = DialogueBuilderTests.ParsedVector("canonical-numbers.json");
            Assert.NotEmpty(numbers["builder"].Items);
            foreach (var pair in numbers["builder"].Items)
            {
                var x = double.Parse(pair.Str("input"), NumberStyles.Float, CultureInfo.InvariantCulture);
                Assert.Equal(pair.Str("canonical"), CanonicalJson.Number(x));
            }

            // An integral double has no fraction; -0 is 0.
            Assert.Equal("2", CanonicalJson.Number(2.0));
            Assert.Equal("0", CanonicalJson.Number(-0.0));

            // A lone surrogate cannot be UTF-8; a pair is written raw.
            Assert.Throws<ArgumentException>(() => Quoted("a\uD83D"));
            Assert.Throws<ArgumentException>(() => Quoted("\uDE00b"));
            Assert.Equal("\"😀\"", Quoted("😀"));

            // The comparison form of the canonical two-keepers is the comparison vector.
            var canonical = DialogueBuilderTests.ParsedVector("two-keepers.canonical.json");
            Assert.True(canonical.Has("x-layout"));
            Assert.Equal(DialogueBuilderTests.DialogueVector("two-keepers.canonical.json"), CanonicalJson.Write(canonical));
            Assert.Equal(DialogueBuilderTests.DialogueVector("two-keepers.comparison.json"), CanonicalJson.ComparisonForm(canonical));
        }
    }
}
