using System.Linq;
using System.Text;
using Lingara.Embed.Json;
using Xunit;

namespace Lingara.Embed.Tests
{
    /// <summary>The package's own JSON reader and writer (ADR 1.10.26aa D8), against a corpus of the awkward cases.</summary>
    public class JsonReaderTests
    {
        private static readonly string[] Corpus =
        {
            "{\"type\":\"npc_turn\",\"update\":{\"type\":\"fragment\",\"text\":\"Ni\"}}",
            "{\"emoji\":\"\\ud83d\\ude00 and 😀\",\"cjk\":\"你好\"}",
            "{\"ctl\":\"\\u0000\\u0001\\u001f\",\"short\":\"\\\"\\\\\\/\\b\\f\\n\\r\\t\"}",
            "[1,-2,3.5,-0.25e+3,1E-2,0,9223372036854775807]",
            "{\"t\":true,\"f\":false,\"n\":null,\"e\":{},\"a\":[]}",
            "  {\"spaced\" : [ 1 , 2 ] }  ",
            string.Concat(Enumerable.Repeat("[", 64)) + "\"deep\"" + string.Concat(Enumerable.Repeat("]", 64)),
        };

        private static bool Same(JsonValue a, JsonValue b)
        {
            if (a.Kind != b.Kind || a.Text != b.Text || a.Items.Count != b.Items.Count || a.Members.Count != b.Members.Count)
            {
                return false;
            }

            return a.Items.Zip(b.Items).All(x => Same(x.First, x.Second))
                && a.Members.Zip(b.Members).All(x => x.First.Key == x.Second.Key && Same(x.First.Value, x.Second.Value));
        }

        // 1.10.26aa AC21
        [Fact]
        public void TheCorpusRoundTripsAndBadInputIsRefused()
        {
            foreach (var text in Corpus)
            {
                Assert.True(JsonReader.TryParse(Encoding.UTF8.GetBytes(text), out var read, out var error), $"{text}: {error}");
                var written = new JsonWriter().Value(read).ToUtf8();
                Assert.True(JsonReader.TryParse(written, out var again, out error), error);
                Assert.True(Same(read, again), $"{text} did not round-trip: {Encoding.UTF8.GetString(written)}");
            }

            Assert.True(JsonReader.TryParse(Encoding.UTF8.GetBytes(Corpus[1]), out var emoji, out _));
            Assert.Equal("😀 and 😀", emoji["emoji"].AsString());
            Assert.True(JsonReader.TryParse(Encoding.UTF8.GetBytes(Corpus[2]), out var escapes, out _));
            Assert.Equal("\u0000\u0001\u001f", escapes["ctl"].AsString());
            Assert.Equal("\"\\/\b\f\n\r\t", escapes["short"].AsString());

            var bad = new[]
            {
                Encoding.UTF8.GetBytes("{\"type\":\"np"),
                Encoding.UTF8.GetBytes("{\"a\":[1,2"),
                Encoding.UTF8.GetBytes("{\"a\":1,}"),
                Encoding.UTF8.GetBytes("{\"a\":\"\\x\"}"),
                Encoding.UTF8.GetBytes("{\"a\":\"\\u12\"}"),
                Encoding.UTF8.GetBytes("{\"a\":\"line\nbreak\"}"),
                Encoding.UTF8.GetBytes("{} {}"),
                new byte[] { (byte)'"', 0xC3, 0x28, (byte)'"' },
                new byte[] { (byte)'"', 0xFF, (byte)'"' },
                Encoding.UTF8.GetBytes(string.Concat(Enumerable.Repeat("[", 200))),
            };
            foreach (var input in bad)
            {
                Assert.False(JsonReader.TryParse(input, out _, out var error), Encoding.UTF8.GetString(input));
                Assert.StartsWith("invalid JSON at byte", error, System.StringComparison.Ordinal);
            }
        }
    }
}
