using System.Collections.Generic;

namespace Lingara.Embed.Dialogues
{
    /// <summary>
    /// Shorthands for the three v1 ends. In <c>dialogue.v2</c> <c>success</c> and <c>failure</c> are ordinary
    /// ending ids (ADR 3.11.26g §11): a dialogue that declares <see cref="DialogueBuilder.Ending"/>s names its own.
    /// </summary>
    public enum Outcome
    {
        Success,
        Failure,
        Abandoned,
    }

    /// <summary>The three types a fact may be declared with.</summary>
    public enum FactType
    {
#pragma warning disable CA1720 // the IR's own words for the types
        String,
        Number,
        Boolean,
#pragma warning restore CA1720
    }

    /// <summary>
    /// A built <c>dialogue.v1</c> or <c>dialogue.v2</c> graph (ADR 28.10.26g D1, ADR 3.11.26g §11.5): immutable, its parts in the order
    /// the author added them. <see cref="ToCanonicalJson"/> is what <c>DialogueLoad</c> sends.
    /// </summary>
    public sealed class Dialogue
    {
        internal Dialogue(DialogueBuilder b)
        {
            Id = b.Id;
            Title = b.TitleText;
            Pedagogy = b.PedagogyValue;
            Npcs = b.Npcs.ToArray();
            Facts = b.Facts.ToArray();
            Endings = b.DeclaredEndings();
            Start = b.Start;
            Checkpoints = b.Checkpoints.ToArray();
        }

        public string Id { get; }

        public string? Title { get; }

        public Pedagogy Pedagogy { get; }

        public IReadOnlyList<DialogueNpc> Npcs { get; }

        public IReadOnlyList<DialogueFact> Facts { get; }

        /// <summary>The declared endings: <c>success</c> and <c>failure</c> when the builder declared none.</summary>
        public IReadOnlyList<DialogueEnding> Endings { get; }

        public string Start { get; }

        public IReadOnlyList<DialogueCheckpoint> Checkpoints { get; }

        /// <summary>JCS over the normal form: no whitespace, no trailing newline (D2).</summary>
        public string ToCanonicalJson() => CanonicalJson.Write(CanonicalJson.Tree(this));
    }

    /// <summary>A way the story can end (ADR 3.11.26g §11.1); <see cref="Label"/> is what a person reads.</summary>
    public sealed class DialogueEnding
    {
        internal DialogueEnding(string id, string? label)
        {
            Id = id;
            Label = label;
        }

        public string Id { get; }

        public string? Label { get; }
    }

    /// <summary>A member of the cast; <see cref="Languages"/> null is N1's native speaker.</summary>
    public sealed class DialogueNpc
    {
        internal DialogueNpc(string id, string name, string? persona, IReadOnlyList<NpcLanguage>? languages)
        {
            Id = id;
            Name = name;
            Persona = persona;
            Languages = languages;
        }

        public string Id { get; }

        public string Name { get; }

        public string? Persona { get; }

        public IReadOnlyList<NpcLanguage>? Languages { get; }
    }

    /// <summary>A fact the game tracks; <see cref="Default"/> is a bool, a double or a string.</summary>
    public sealed class DialogueFact
    {
        internal DialogueFact(string key, FactType type, object? defaultValue)
        {
            Key = key;
            Type = type;
            Default = defaultValue;
        }

        public string Key { get; }

        public FactType Type { get; }

        public object? Default { get; }
    }

    /// <summary>A beat of the story.</summary>
    public sealed class DialogueCheckpoint
    {
        internal DialogueCheckpoint(string id, string goal, IReadOnlyList<string> mayDisclose, IReadOnlyList<DialogueEdge> edges)
        {
            Id = id;
            Goal = goal;
            MayDisclose = mayDisclose;
            Edges = edges;
        }

        public string Id { get; }

        public string Goal { get; }

        public IReadOnlyList<string> MayDisclose { get; }

        public IReadOnlyList<DialogueEdge> Edges { get; }
    }

    /// <summary>A way out of a checkpoint: <see cref="To"/> is a checkpoint id, an ending id or <c>abandoned</c>.</summary>
    public sealed class DialogueEdge
    {
        internal DialogueEdge(string id, string to, string? predicate, string? when)
        {
            Id = id;
            To = to;
            Predicate = predicate;
            When = when;
        }

        public string Id { get; }

        public string To { get; }

        /// <summary>The game condition, in K1's predicate language (<c>trust &gt;= 0.4</c>).</summary>
        public string? Predicate { get; }

        /// <summary>The conversational condition the server's judge reads.</summary>
        public string? When { get; }
    }
}
