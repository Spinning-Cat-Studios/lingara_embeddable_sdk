using System;
using System.Collections.Generic;
using System.Globalization;

namespace Lingara.Embed
{
    /// <summary>How much the NPC adapts its speech to the learner (N1): per dialogue.</summary>
    public enum Pedagogy
    {
        Unknown = -1,

        /// <summary>The speech level is the lower of the NPC's speaking level and the learner's.</summary>
        Subtle,

        /// <summary>The NPC's own level decides.</summary>
        None,
    }

    /// <summary>How a graph dialogue reached a checkpoint (ADR 28.10.26f D6).</summary>
    public enum CrossedVia
    {
        Unknown = -1,
        Start,
        Auto,
        Judge,
    }

    /// <summary>The reserved names a graph dialogue's story ends on.</summary>
    public enum DialogueOutcome
    {
        Unknown = -1,
        Success,
        Failure,
        Abandoned,
    }

    /// <summary>One of an NPC's languages (N1): levels 1–9.</summary>
    public sealed class NpcLanguage
    {
        public string Code { get; init; } = string.Empty;

        public byte Speaking { get; init; }

        public byte Comprehension { get; init; }
    }

    /// <summary>A fact's value: a boolean, a number or a string, as the dialogue declared it.</summary>
    public abstract class FactValue
    {
        private FactValue()
        {
        }

        public static FactValue Of(bool value) => new Flag { Value = value };

        public static FactValue Of(double value) => new Number { Value = value };

        public static FactValue Of(string value) => new Text { Value = value };

        /// <summary>A <c>boolean</c> fact's value (CA1716 reserves <c>Boolean</c>).</summary>
        public sealed class Flag : FactValue
        {
            public bool Value { get; init; }
        }

        public sealed class Number : FactValue
        {
            public double Value { get; init; }

            /// <summary>K1's JSON for it: a whole number as an integer, so <c>0</c> stays <c>0</c>.</summary>
            internal string Literal => Value == Math.Floor(Value) && Math.Abs(Value) < 9e15
                ? ((long)Value).ToString(CultureInfo.InvariantCulture)
                : Value.ToString("R", CultureInfo.InvariantCulture);
        }

        public sealed class Text : FactValue
        {
            public string Value { get; init; } = string.Empty;
        }
    }

    /// <summary>
    /// One run of a <c>dialogue.v1</c> graph (ADR 28.10.26f D6): which cast member speaks, the
    /// learner's settings, and, to resume a saved story, its checkpoint, facts and window.
    /// </summary>
    public sealed class GraphSession
    {
        /// <summary>A cast member's id; may be null only when the cast has one.</summary>
        public string? Npc { get; init; }

        public string SourceLang { get; init; } = string.Empty;

        public string TargetLang { get; init; } = string.Empty;

        public byte Level { get; init; }

        public string? Scene { get; init; }

        public IReadOnlyDictionary<string, FactValue> Facts { get; init; } = new Dictionary<string, FactValue>();

        /// <summary>Resume here instead of at the dialogue's <c>start</c>.</summary>
        public string? Checkpoint { get; init; }

        public IReadOnlyList<WindowEntry> Window { get; init; } = Array.Empty<WindowEntry>();
    }

    /// <summary>Where a graph dialogue comes from: its IR as text, or a dialogue saved on Lingara (ADR 28.10.26i).</summary>
    public abstract class DialogueSource
    {
        private DialogueSource() { }

        /// <summary>A <c>dialogue.v1</c> document, as text.</summary>
        public static DialogueSource Ir(string irJson) => new IrSource(irJson);

        /// <summary>The active revision of a saved dialogue, by its store id (a UUID); <c>DialogueLoaded</c> follows once it is fetched.</summary>
        public static DialogueSource Stored(string dialogueId) => new StoredSource(dialogueId);

        public sealed class IrSource : DialogueSource
        {
            public IrSource(string irJson) => IrJson = irJson;

            public string IrJson { get; }
        }

        public sealed class StoredSource : DialogueSource
        {
            public StoredSource(string dialogueId) => DialogueId = dialogueId;

            public string DialogueId { get; }
        }
    }

    /// <summary>A graph dialogue's story, for the game to save: only the facts it set.</summary>
    public sealed class GraphSnapshot
    {
        /// <summary>The dialogue's own <c>id</c>.</summary>
        public string Id { get; init; } = string.Empty;

        public string Npc { get; init; } = string.Empty;

        public string Checkpoint { get; init; } = string.Empty;

        public IReadOnlyDictionary<string, FactValue> Facts { get; init; } = new Dictionary<string, FactValue>();

        public DialogueOutcome? Outcome { get; init; }
    }

    internal static class GraphWords
    {
        public static readonly WordTable<Pedagogy> Pedagogies = new WordTable<Pedagogy>(
            Pedagogy.Unknown, W("subtle", Pedagogy.Subtle), W("none", Pedagogy.None));

        public static readonly WordTable<CrossedVia> CrossedVias = new WordTable<CrossedVia>(
            CrossedVia.Unknown, W("start", CrossedVia.Start), W("auto", CrossedVia.Auto), W("judge", CrossedVia.Judge));

        public static readonly WordTable<DialogueOutcome> Outcomes = new WordTable<DialogueOutcome>(
            DialogueOutcome.Unknown, W("success", DialogueOutcome.Success), W("failure", DialogueOutcome.Failure), W("abandoned", DialogueOutcome.Abandoned));

        private static KeyValuePair<string, T> W<T>(string word, T value) => new KeyValuePair<string, T>(word, value);
    }
}
