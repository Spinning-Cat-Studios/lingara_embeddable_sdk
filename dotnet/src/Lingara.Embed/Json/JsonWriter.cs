using System.Collections.Generic;
using System.Globalization;
using System.Text;

namespace Lingara.Embed.Json
{
    /// <summary>
    /// A forward-only JSON writer (ADR 1.10.26aa D8): the commands' bytes, in
    /// K1's shape, with no dependency and no reflection. Commas are placed by a
    /// per-container flag, so callers write members in order and nothing else.
    /// </summary>
    internal sealed class JsonWriter
    {
        private readonly StringBuilder sb = new StringBuilder();
        private readonly Stack<bool> first = new Stack<bool>();
        private bool afterName;

        public JsonWriter BeginObject() => Open('{');

        public JsonWriter EndObject() => Close('}');

        public JsonWriter BeginArray() => Open('[');

        public JsonWriter EndArray() => Close(']');

        public JsonWriter Name(string name)
        {
            Separate();
            Quote(name);
            sb.Append(':');
            afterName = true;
            return this;
        }

        public JsonWriter String(string? value)
        {
            Separate();
            if (value == null)
            {
                sb.Append("null");
            }
            else
            {
                Quote(value);
            }

            return this;
        }

        public JsonWriter Number(long value)
        {
            Separate();
            sb.Append(value.ToString(CultureInfo.InvariantCulture));
            return this;
        }

        public JsonWriter Bool(bool value)
        {
            Separate();
            sb.Append(value ? "true" : "false");
            return this;
        }

        public JsonWriter Null()
        {
            Separate();
            sb.Append("null");
            return this;
        }

        /// <summary>A number literal as read, for round-tripping a parsed value.</summary>
        public JsonWriter Literal(string text)
        {
            Separate();
            sb.Append(text);
            return this;
        }

        public JsonWriter Prop(string name, string? value) => Name(name).String(value);

        public JsonWriter Prop(string name, long value) => Name(name).Number(value);

        public JsonWriter Prop(string name, bool value) => Name(name).Bool(value);

        /// <summary>Writes a parsed value back out.</summary>
        public JsonWriter Value(JsonValue value)
        {
            switch (value.Kind)
            {
                case JsonKind.Object:
                    BeginObject();
                    foreach (var member in value.Members)
                    {
                        Name(member.Key).Value(member.Value);
                    }

                    return EndObject();
                case JsonKind.Array:
                    BeginArray();
                    foreach (var item in value.Items)
                    {
                        Value(item);
                    }

                    return EndArray();
                case JsonKind.String: return String(value.Text);
                case JsonKind.Number: return Literal(value.Text!);
                case JsonKind.True: return Bool(true);
                case JsonKind.False: return Bool(false);
                default: return Null();
            }
        }

        public byte[] ToUtf8() => Encoding.UTF8.GetBytes(sb.ToString());

        public override string ToString() => sb.ToString();

        private JsonWriter Open(char c)
        {
            Separate();
            sb.Append(c);
            first.Push(true);
            return this;
        }

        private JsonWriter Close(char c)
        {
            first.Pop();
            sb.Append(c);
            return this;
        }

        /// <summary>A comma before every value but a container's first, and none after a name.</summary>
        private void Separate()
        {
            if (afterName)
            {
                afterName = false;
                return;
            }

            if (first.Count == 0)
            {
                return;
            }

            if (first.Peek())
            {
                first.Pop();
                first.Push(false);
            }
            else
            {
                sb.Append(',');
            }
        }

        private void Quote(string value)
        {
            sb.Append('"');
            foreach (var c in value)
            {
                switch (c)
                {
                    case '"': sb.Append("\\\""); break;
                    case '\\': sb.Append("\\\\"); break;
                    case '\n': sb.Append("\\n"); break;
                    case '\r': sb.Append("\\r"); break;
                    case '\t': sb.Append("\\t"); break;
                    case '\b': sb.Append("\\b"); break;
                    case '\f': sb.Append("\\f"); break;
                    default: Plain(c); break;
                }
            }

            sb.Append('"');
        }

        private void Plain(char c)
        {
            if (c < 0x20)
            {
                sb.Append("\\u").Append(((int)c).ToString("x4", CultureInfo.InvariantCulture));
            }
            else
            {
                sb.Append(c);
            }
        }
    }
}
