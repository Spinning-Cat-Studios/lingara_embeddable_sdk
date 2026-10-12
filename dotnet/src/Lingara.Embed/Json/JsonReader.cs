using System;
using System.Collections.Generic;
using System.Globalization;
using System.Text;

namespace Lingara.Embed.Json
{
    internal enum JsonKind
    {
        Null,
        False,
        True,
        Number,
        String,
        Array,
        Object,
    }

    /// <summary>
    /// One parsed JSON value. K1's messages are shallow and small (ADR 1.10.26aa D8),
    /// so the reader builds this little tree in one forward pass over the UTF-8
    /// bytes, and the decoders read it by key. Numbers keep their literal text.
    /// </summary>
    internal sealed class JsonValue
    {
        public static readonly JsonValue NullValue = new JsonValue(JsonKind.Null);

        public JsonValue(JsonKind kind, string? text = null)
        {
            Kind = kind;
            Text = text;
        }

        public JsonKind Kind { get; }

        /// <summary>A string's value, or a number's literal.</summary>
        public string? Text { get; }

        public List<JsonValue> Items { get; } = new List<JsonValue>();

        public List<KeyValuePair<string, JsonValue>> Members { get; } = new List<KeyValuePair<string, JsonValue>>();

        public bool IsNull => Kind == JsonKind.Null;

        /// <summary>The member named <paramref name="key"/>, or JSON null when absent.</summary>
        public JsonValue this[string key]
        {
            get
            {
                foreach (var member in Members)
                {
                    if (member.Key == key)
                    {
                        return member.Value;
                    }
                }

                return NullValue;
            }
        }

        public bool Has(string key)
        {
            foreach (var member in Members)
            {
                if (member.Key == key)
                {
                    return true;
                }
            }

            return false;
        }

        public string? AsString() => Kind == JsonKind.String ? Text : null;

        public string Str(string key) => this[key].AsString() ?? throw Missing(key, "a string");

        public string? OptStr(string key) => this[key].AsString();

        public long Long(string key) => this[key].Kind == JsonKind.Number
            ? long.Parse(this[key].Text!, NumberStyles.Integer, CultureInfo.InvariantCulture)
            : throw Missing(key, "an integer");

        public long? OptLong(string key) => this[key].IsNull ? (long?)null : Long(key);

        public bool Bool(string key) => this[key].Kind switch
        {
            JsonKind.True => true,
            JsonKind.False => false,
            _ => throw Missing(key, "a boolean"),
        };

        public JsonValue Obj(string key) => this[key].Kind == JsonKind.Object ? this[key] : throw Missing(key, "an object");

        private static JsonFormatException Missing(string key, string what) => new JsonFormatException($"`{key}` is not {what}");
    }

    /// <summary>A message that is not the JSON or the shape a decoder expects.</summary>
    internal sealed class JsonFormatException : Exception
    {
        public JsonFormatException(string message)
            : base(message)
        {
        }

        public JsonFormatException()
        {
        }

        public JsonFormatException(string message, Exception inner)
            : base(message, inner)
        {
        }
    }

    /// <summary>
    /// A forward-only UTF-8 JSON reader (D8): no dependency, no reflection. Invalid
    /// or truncated input is refused with a message, never a runtime exception.
    /// </summary>
    internal static class JsonReader
    {
        public const int MaxDepth = 128;

        public static bool TryParse(ReadOnlySpan<byte> utf8, out JsonValue value, out string error)
        {
            try
            {
                value = Parse(utf8);
                error = string.Empty;
                return true;
            }
            catch (JsonFormatException e)
            {
                value = JsonValue.NullValue;
                error = e.Message;
                return false;
            }
        }

        /// <exception cref="JsonFormatException">The input is not one JSON value.</exception>
        public static JsonValue Parse(ReadOnlySpan<byte> utf8)
        {
            var parser = new Parser(utf8);
            var value = parser.Value(0);
            parser.SkipSpace();
            if (!parser.AtEnd)
            {
                throw parser.Fail("trailing bytes after the value");
            }

            return value;
        }

        private ref struct Parser
        {
            private static readonly Encoding Strict = new UTF8Encoding(false, true);
            private readonly ReadOnlySpan<byte> s;
            private int pos;

            public Parser(ReadOnlySpan<byte> s)
            {
                this.s = s;
                pos = 0;
            }

            public bool AtEnd => pos >= s.Length;

            public JsonFormatException Fail(string what) => new JsonFormatException($"invalid JSON at byte {pos}: {what}");

            public void SkipSpace()
            {
                while (pos < s.Length && (s[pos] == ' ' || s[pos] == '\t' || s[pos] == '\n' || s[pos] == '\r'))
                {
                    pos++;
                }
            }

            public JsonValue Value(int depth)
            {
                if (depth > MaxDepth)
                {
                    throw Fail($"nested deeper than {MaxDepth}");
                }

                SkipSpace();
                if (AtEnd)
                {
                    throw Fail("truncated");
                }

                return s[pos] switch
                {
                    (byte)'{' => Object(depth),
                    (byte)'[' => Array(depth),
                    (byte)'"' => new JsonValue(JsonKind.String, String()),
                    (byte)'t' => Literal("true", JsonKind.True),
                    (byte)'f' => Literal("false", JsonKind.False),
                    (byte)'n' => Literal("null", JsonKind.Null),
                    _ => Number(),
                };
            }

            private JsonValue Literal(string word, JsonKind kind)
            {
                for (var i = 0; i < word.Length; i++)
                {
                    if (pos + i >= s.Length || s[pos + i] != word[i])
                    {
                        throw Fail($"expected `{word}`");
                    }
                }

                pos += word.Length;
                return kind == JsonKind.Null ? JsonValue.NullValue : new JsonValue(kind);
            }

            private void Expect(byte b)
            {
                SkipSpace();
                if (AtEnd || s[pos] != b)
                {
                    throw Fail(AtEnd ? "truncated" : $"expected `{(char)b}`");
                }

                pos++;
            }

            /// <summary>True when the next byte closes the container, consuming it.</summary>
            private bool Closes(byte close)
            {
                SkipSpace();
                if (!AtEnd && s[pos] == close)
                {
                    pos++;
                    return true;
                }

                return false;
            }

            private JsonValue Object(int depth)
            {
                pos++;
                var obj = new JsonValue(JsonKind.Object);
                if (Closes((byte)'}'))
                {
                    return obj;
                }

                do
                {
                    SkipSpace();
                    if (AtEnd || s[pos] != '"')
                    {
                        throw Fail(AtEnd ? "truncated" : "expected a member name");
                    }

                    var key = String();
                    Expect((byte)':');
                    obj.Members.Add(new KeyValuePair<string, JsonValue>(key, Value(depth + 1)));
                }
                while (!Closes((byte)'}') && Comma());

                return obj;
            }

            private JsonValue Array(int depth)
            {
                pos++;
                var array = new JsonValue(JsonKind.Array);
                if (Closes((byte)']'))
                {
                    return array;
                }

                do
                {
                    array.Items.Add(Value(depth + 1));
                }
                while (!Closes((byte)']') && Comma());

                return array;
            }

            private bool Comma()
            {
                Expect((byte)',');
                return true;
            }

            private string String()
            {
                pos++;
                var sb = new StringBuilder();
                var run = pos;
                while (true)
                {
                    if (AtEnd)
                    {
                        throw Fail("truncated string");
                    }

                    var b = s[pos];
                    if (b == '"' || b == '\\')
                    {
                        sb.Append(Decode(s.Slice(run, pos - run)));
                        pos++;
                        if (b == '"')
                        {
                            return sb.ToString();
                        }

                        Escape(sb);
                        run = pos;
                    }
                    else if (b < 0x20)
                    {
                        throw Fail("an unescaped control character in a string");
                    }
                    else
                    {
                        pos++;
                    }
                }
            }

            private string Decode(ReadOnlySpan<byte> bytes)
            {
                try
                {
                    return Strict.GetString(bytes);
                }
                catch (DecoderFallbackException)
                {
                    throw Fail("invalid UTF-8");
                }
            }

            private void Escape(StringBuilder sb)
            {
                if (AtEnd)
                {
                    throw Fail("truncated escape");
                }

                var e = s[pos++];
                if (e == 'u')
                {
                    sb.Append(Hex4());
                    return;
                }

                var at = ShortEscapes.IndexOf((char)e);
                sb.Append(at >= 0 ? ShortValues[at] : throw Fail("an unknown escape"));
            }

            // The escapes with one letter, and what each stands for, in the same order.
            private const string ShortEscapes = "\"\\/bfnrt";
            private const string ShortValues = "\"\\/\b\f\n\r\t";

            private char Hex4()
            {
                if (pos + 4 > s.Length)
                {
                    throw Fail("truncated \\u escape");
                }

                var value = 0;
                for (var i = 0; i < 4; i++)
                {
                    var digit = HexDigit(s[pos + i]);
                    value = (value << 4) | digit;
                }

                pos += 4;
                return (char)value;
            }

            private int HexDigit(byte c)
            {
                if (c >= '0' && c <= '9')
                {
                    return c - '0';
                }

                var lower = c | 0x20;
                return lower >= 'a' && lower <= 'f' ? lower - 'a' + 10 : throw Fail("a bad hex digit");
            }

            private JsonValue Number()
            {
                var start = pos;
                if (s[pos] == '-')
                {
                    pos++;
                }

                Digits();
                if (!AtEnd && s[pos] == '.')
                {
                    pos++;
                    Digits();
                }

                if (!AtEnd && (s[pos] | 0x20) == 'e')
                {
                    pos++;
                    if (!AtEnd && (s[pos] == '+' || s[pos] == '-'))
                    {
                        pos++;
                    }

                    Digits();
                }

                return new JsonValue(JsonKind.Number, Encoding.ASCII.GetString(s.Slice(start, pos - start)));
            }

            private void Digits()
            {
                var start = pos;
                while (!AtEnd && s[pos] >= '0' && s[pos] <= '9')
                {
                    pos++;
                }

                if (pos == start)
                {
                    throw Fail(AtEnd ? "truncated number" : "expected a value");
                }
            }
        }
    }
}
