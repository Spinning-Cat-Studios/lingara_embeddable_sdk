using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text;
using Lingara.Embed.Json;

namespace Lingara.Embed.Dialogues
{
    /// <summary>
    /// The IR's canonical form (ADR 28.10.26g D2), written by hand: RFC 8785 (JCS) over the
    /// normal form, for what a builder holds. Keys sort by UTF-16 code unit (ordinal), which
    /// is JCS's order; strings escape only <c>"</c>, <c>\</c> and U+0000–U+001F; numbers are
    /// the builders' range only (an integer to 2^53 − 1, or up to four decimal places in
    /// <c>1e-4 ≤ |x| &lt; 1e9</c>), where .NET's shortest round-trip form is the plain decimal.
    /// </summary>
    internal static class CanonicalJson
    {
        /// <summary>The largest integer magnitude a builder writes (D2 rule 3).</summary>
        public const double MaxSafeInteger = 9007199254740991d;

        public static string Write(JsonValue value)
        {
            var sb = new StringBuilder();
            Value(sb, value);
            return sb.ToString();
        }

        /// <summary>The canonical text with every <c>x-</c> member removed, at every depth.</summary>
        public static string ComparisonForm(JsonValue value) => Write(Strip(value));

        /// <summary>A builder number's canonical text, or null outside the builders' range.</summary>
        public static string? Number(double x)
        {
            if (double.IsNaN(x) || double.IsInfinity(x))
            {
                return null;
            }

            if (x == Math.Floor(x))
            {
                // (long)-0.0 is 0, so -0 writes as 0.
                return Math.Abs(x) <= MaxSafeInteger ? ((long)x).ToString(CultureInfo.InvariantCulture) : null;
            }

            var magnitude = Math.Abs(x);
            var fourPlaces = Math.Round(x * 1e4) / 1e4 == x;
            return magnitude >= 1e-4 && magnitude < 1e9 && fourPlaces ? x.ToString("R", CultureInfo.InvariantCulture) : null;
        }

        /// <summary>D2 rule 5 over UTF-16; a lone surrogate cannot be UTF-8 and is refused.</summary>
        /// <exception cref="ArgumentException">The text holds a lone surrogate.</exception>
        public static void Quote(StringBuilder sb, string text)
        {
            sb.Append('"');
            for (var i = 0; i < text.Length; i++)
            {
                var c = text[i];
                if (char.IsHighSurrogate(c) && i + 1 < text.Length && char.IsLowSurrogate(text[i + 1]))
                {
                    sb.Append(c).Append(text[++i]);
                }
                else if (char.IsSurrogate(c))
                {
                    throw new ArgumentException($"a lone surrogate U+{(int)c:X4} at {i} cannot be written as UTF-8", nameof(text));
                }
                else
                {
                    Escape(sb, c);
                }
            }

            sb.Append('"');
        }

        private static void Escape(StringBuilder sb, char c)
        {
            switch (c)
            {
                case '"': sb.Append("\\\""); break;
                case '\\': sb.Append("\\\\"); break;
                case '\b': sb.Append("\\b"); break;
                case '\t': sb.Append("\\t"); break;
                case '\n': sb.Append("\\n"); break;
                case '\f': sb.Append("\\f"); break;
                case '\r': sb.Append("\\r"); break;
                default:
                    if (c < 0x20)
                    {
                        sb.Append("\\u").Append(((int)c).ToString("x4", CultureInfo.InvariantCulture));
                    }
                    else
                    {
                        sb.Append(c);
                    }

                    break;
            }
        }

        private static void Value(StringBuilder sb, JsonValue value)
        {
            switch (value.Kind)
            {
                case JsonKind.Null: sb.Append("null"); break;
                case JsonKind.False: sb.Append("false"); break;
                case JsonKind.True: sb.Append("true"); break;
                case JsonKind.String: Quote(sb, value.Text ?? string.Empty); break;
                case JsonKind.Number: sb.Append(ParsedNumber(value.Text ?? "0")); break;
                case JsonKind.Array: Array(sb, value.Items); break;
                default: Object(sb, value.Members); break;
            }
        }

        private static string ParsedNumber(string literal)
        {
            var x = double.Parse(literal, NumberStyles.Float, CultureInfo.InvariantCulture);
            return Number(x) ?? throw new ArgumentException($"{literal} is outside the builders' number range", nameof(literal));
        }

        private static void Array(StringBuilder sb, List<JsonValue> items)
        {
            sb.Append('[');
            for (var i = 0; i < items.Count; i++)
            {
                if (i > 0)
                {
                    sb.Append(',');
                }

                Value(sb, items[i]);
            }

            sb.Append(']');
        }

        private static void Object(StringBuilder sb, List<KeyValuePair<string, JsonValue>> members)
        {
            sb.Append('{');
            var first = true;
            foreach (var member in members.OrderBy(m => m.Key, StringComparer.Ordinal))
            {
                if (!first)
                {
                    sb.Append(',');
                }

                first = false;
                Quote(sb, member.Key);
                sb.Append(':');
                Value(sb, member.Value);
            }

            sb.Append('}');
        }

        private static JsonValue Strip(JsonValue value)
        {
            if (value.Kind == JsonKind.Array)
            {
                var array = new JsonValue(JsonKind.Array);
                array.Items.AddRange(value.Items.Select(Strip));
                return array;
            }

            if (value.Kind != JsonKind.Object)
            {
                return value;
            }

            var obj = new JsonValue(JsonKind.Object);
            foreach (var member in value.Members.Where(m => !m.Key.StartsWith("x-", StringComparison.Ordinal)))
            {
                obj.Members.Add(new KeyValuePair<string, JsonValue>(member.Key, Strip(member.Value)));
            }

            return obj;
        }

        /// <summary>The dialogue in normal form (D2 rule 1): the always-written members, empty or not.</summary>
        public static JsonValue Tree(Dialogue d)
        {
            var root = Obj(("schema", Str("dialogue.v1")), ("id", Str(d.Id)), ("pedagogy", Str(d.Pedagogy == Pedagogy.None ? "none" : "subtle")),
                ("npcs", Arr(d.Npcs.Select(NpcTree))), ("facts", Arr(d.Facts.Select(FactTree))), ("start", Str(d.Start)),
                ("checkpoints", Arr(d.Checkpoints.Select(CheckpointTree))));
            Opt(root, "title", d.Title);
            return root;
        }

        private static JsonValue NpcTree(DialogueNpc npc)
        {
            var o = Obj(("id", Str(npc.Id)), ("name", Str(npc.Name)));
            Opt(o, "persona", npc.Persona);
            if (npc.Languages != null)
            {
                var languages = npc.Languages.Select(l => Obj(("code", Str(l.Code)), ("speaking", Num(l.Speaking)), ("comprehension", Num(l.Comprehension))));
                o.Members.Add(new KeyValuePair<string, JsonValue>("languages", Arr(languages)));
            }

            return o;
        }

        private static JsonValue FactTree(DialogueFact fact)
        {
            var type = fact.Type == FactType.Number ? "number" : fact.Type == FactType.Boolean ? "boolean" : "string";
            var o = Obj(("key", Str(fact.Key)), ("type", Str(type)));
            var value = fact.Default switch
            {
                bool b => new JsonValue(b ? JsonKind.True : JsonKind.False),
                double x => Num(x),
                string s => Str(s),
                _ => null,
            };
            if (value != null)
            {
                o.Members.Add(new KeyValuePair<string, JsonValue>("default", value));
            }

            return o;
        }

        private static JsonValue CheckpointTree(DialogueCheckpoint c)
        {
            var edges = c.Edges.Select(e =>
            {
                var o = Obj(("id", Str(e.Id)), ("to", Str(e.To)));
                Opt(o, "predicate", e.Predicate);
                Opt(o, "when", e.When);
                return o;
            });
            return Obj(("id", Str(c.Id)), ("goal", Str(c.Goal)), ("may_disclose", Arr(c.MayDisclose.Select(Str))), ("edges", Arr(edges)));
        }

        private static JsonValue Str(string s) => new JsonValue(JsonKind.String, s);

        private static JsonValue Num(double x) => new JsonValue(JsonKind.Number, x.ToString("R", CultureInfo.InvariantCulture));

        private static JsonValue Arr(IEnumerable<JsonValue> items)
        {
            var a = new JsonValue(JsonKind.Array);
            a.Items.AddRange(items);
            return a;
        }

        private static JsonValue Obj(params (string Key, JsonValue Value)[] members)
        {
            var o = new JsonValue(JsonKind.Object);
            o.Members.AddRange(members.Select(m => new KeyValuePair<string, JsonValue>(m.Key, m.Value)));
            return o;
        }

        private static void Opt(JsonValue o, string key, string? value)
        {
            if (value != null)
            {
                o.Members.Add(new KeyValuePair<string, JsonValue>(key, Str(value)));
            }
        }
    }
}
