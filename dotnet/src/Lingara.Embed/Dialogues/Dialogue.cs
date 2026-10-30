using System.Collections.Generic;

namespace Lingara.Embed.Dialogues
{
    /// <summary>Where an edge ends the story instead of naming a checkpoint.</summary>
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
    /// A built <c>dialogue.v1</c> graph (ADR 28.10.26g D1): immutable, its parts in the order
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
            Start = b.Start;
            Checkpoints = b.Checkpoints.ToArray();
        }

        public string Id { get; }

        public string? Title { get; }

        public Pedagogy Pedagogy { get; }

        public IReadOnlyList<DialogueNpc> Npcs { get; }

        public IReadOnlyList<DialogueFact> Facts { get; }

        public string Start { get; }

        public IReadOnlyList<DialogueCheckpoint> Checkpoints { get; }

        /// <summary>JCS over the normal form: no whitespace, no trailing newline (D2).</summary>
        public string ToCanonicalJson() => CanonicalJson.Write(CanonicalJson.Tree(this));
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

    /// <summary>A way out of a checkpoint: <see cref="To"/> is a checkpoint id or an outcome's name.</summary>
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
