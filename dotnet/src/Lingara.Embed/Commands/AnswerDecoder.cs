using System;
using Lingara.Embed.Json;
using Lingara.Embed.Native;

namespace Lingara.Embed.Commands
{
    /// <summary>
    /// A command's answer (ADR 1.10.26aa D3): <c>OK</c> carries K1's
    /// <c>CommandOutcome</c>; <c>REFUSED</c> carries its <c>EmbedError</c> and
    /// becomes an <see cref="EmbedException"/>.
    /// </summary>
    internal static class AnswerDecoder
    {
        /// <summary>The outcome object, or the core's refusal thrown.</summary>
        public static JsonValue Outcome(int status, ReadOnlySpan<byte> json)
        {
            var value = Parse(json);
            if (status == NativeStatus.Refused)
            {
                throw new EmbedException(TypeDecoders.Error(value));
            }

            return value;
        }

        public static long Id(JsonValue outcome) => Expect(outcome, "id").Long("id");

        public static PracticeSnapshot? Practice(JsonValue outcome) => TypeDecoders.OptPractice(Expect(outcome, "practice")["snapshot"]);

        public static DialogueSnapshot? Dialogue(JsonValue outcome) => TypeDecoders.OptDialogue(Expect(outcome, "dialogue")["snapshot"]);

        public static void Done(JsonValue outcome) => Expect(outcome, "done");

        private static JsonValue Expect(JsonValue outcome, string type)
        {
            var got = outcome.Str("type");
            return got == type ? outcome : throw new JsonFormatException($"expected a `{type}` outcome, got `{got}`");
        }

        private static JsonValue Parse(ReadOnlySpan<byte> json) =>
            JsonReader.TryParse(json, out var value, out var error) ? value : throw new JsonFormatException(error);
    }
}
