using System;
using System.Collections.Generic;

namespace Lingara.Embed.Dialogues
{
    /// <summary>
    /// Writes a <c>dialogue.v1</c> graph in C# (ADR 28.10.26g D1), for Unity and Godot C# alike.
    /// Parts keep the order they are added in: edge order is meaningful, because the core
    /// auto-crosses the first open edge with no <c>when</c> and the judge takes the first match.
    /// A predicate is K1's text (<c>"trust >= 0.4"</c>), never a lambda.
    /// <see cref="Build"/> checks the structure; the predicate grammar, the length bounds and
    /// the byte cap are the core's, which <c>DialogueLoad</c> and <c>DialogueValidate</c> run.
    /// </summary>
    public sealed class DialogueBuilder
    {
        public DialogueBuilder(string id, string start)
        {
            Id = id;
            Start = start;
        }

        internal string Id { get; }

        internal string Start { get; }

        internal string? TitleText { get; private set; }

        internal Lingara.Embed.Pedagogy PedagogyValue { get; private set; } = Lingara.Embed.Pedagogy.Subtle;

        internal List<DialogueNpc> Npcs { get; } = new List<DialogueNpc>();

        internal List<DialogueFact> Facts { get; } = new List<DialogueFact>();

        internal List<DialogueCheckpoint> Checkpoints { get; } = new List<DialogueCheckpoint>();

        public DialogueBuilder Title(string title)
        {
            TitleText = title;
            return this;
        }

        public DialogueBuilder Pedagogy(Lingara.Embed.Pedagogy pedagogy)
        {
            PedagogyValue = pedagogy;
            return this;
        }

        /// <summary>A member of the cast; <paramref name="profile"/> sets its persona and languages.</summary>
        public DialogueBuilder Npc(string id, string name, Action<NpcBuilder>? profile = null)
        {
            var npc = new NpcBuilder();
            profile?.Invoke(npc);
            Npcs.Add(new DialogueNpc(id, name, npc.PersonaText, npc.Languages.Count == 0 ? null : npc.Languages.ToArray()));
            return this;
        }

        /// <summary>A fact; <paramref name="defaultValue"/> is a bool, a number or a string, matching <paramref name="type"/>.</summary>
        public DialogueBuilder Fact(string key, FactType type, object? defaultValue = null)
        {
            var value = defaultValue switch
            {
                int i => (double)i,
                long l => (double)l,
                float f => (double)f,
                _ => defaultValue,
            };
            Facts.Add(new DialogueFact(key, type, value));
            return this;
        }

        public DialogueBuilder Checkpoint(string id, string goal, Action<CheckpointBuilder>? edges = null)
        {
            var checkpoint = new CheckpointBuilder();
            edges?.Invoke(checkpoint);
            Checkpoints.Add(new DialogueCheckpoint(id, goal, checkpoint.Disclosed.ToArray(), checkpoint.Edges.ToArray()));
            return this;
        }

        /// <summary>The dialogue, once D5's structural checks pass.</summary>
        /// <exception cref="ArgumentException">A structural fault, named by its JSON path.</exception>
        public Dialogue Build()
        {
            StructuralChecks.Run(this);
            return new Dialogue(this);
        }
    }

    /// <summary>One NPC's persona and N1 language profile.</summary>
    public sealed class NpcBuilder
    {
        internal string? PersonaText { get; private set; }

        internal List<NpcLanguage> Languages { get; } = new List<NpcLanguage>();

        public NpcBuilder Persona(string persona)
        {
            PersonaText = persona;
            return this;
        }

        /// <summary>A language at speaking and comprehension levels 1–9 (<see cref="Level"/> maps labels).</summary>
        public NpcBuilder Speaks(string code, byte speaking, byte comprehension)
        {
            Languages.Add(new NpcLanguage { Code = code, Speaking = speaking, Comprehension = comprehension });
            return this;
        }
    }

    /// <summary>A checkpoint's disclosed facts and its edges, in order.</summary>
    public sealed class CheckpointBuilder
    {
        internal List<string> Disclosed { get; } = new List<string>();

        internal List<DialogueEdge> Edges { get; } = new List<DialogueEdge>();

        /// <summary>A declared fact key the NPC may voice here.</summary>
        public CheckpointBuilder Discloses(string key)
        {
            Disclosed.Add(key);
            return this;
        }

        /// <summary>An edge to another checkpoint.</summary>
        public CheckpointBuilder Edge(string id, string to, string? predicate = null, string? when = null)
        {
            Edges.Add(new DialogueEdge(id, to, predicate, when));
            return this;
        }

        /// <summary>An edge that ends the story.</summary>
        public CheckpointBuilder Edge(string id, Outcome to, string? predicate = null, string? when = null) =>
            Edge(id, to == Outcome.Success ? "success" : to == Outcome.Failure ? "failure" : "abandoned", predicate, when);
    }
}
