using System;
using System.Collections.Generic;

namespace Lingara.Embed.Dialogues
{
    /// <summary>
    /// Writes a dialogue graph in C# (ADR 28.10.26g D1), for Unity and Godot C# alike: <c>dialogue.v1</c>,
    /// or <c>dialogue.v2</c> once it declares an <see cref="Ending"/> v1 cannot say (ADR 3.11.26g §11.5).
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

        internal List<DialogueEnding> Endings { get; } = new List<DialogueEnding>();

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
            Npcs.Add(npc.Build(id, name));
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

        /// <summary>
        /// A named ending (ADR 3.11.26g §11). Once one is declared the dialogue ends only on the endings it
        /// declares, plus <c>abandoned</c>; an edge names one by id.
        /// </summary>
        public DialogueBuilder Ending(string id, string? label = null)
        {
            Endings.Add(new DialogueEnding(id, label));
            return this;
        }

        /// <summary>The declared endings, or v1's <c>success</c> and <c>failure</c> when none was declared.</summary>
        internal IReadOnlyList<DialogueEnding> DeclaredEndings() =>
            Endings.Count > 0 ? Endings.ToArray() : new[] { new DialogueEnding("success", null), new DialogueEnding("failure", null) };

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

        /// <summary>
        /// How the NPC steers (ADR 3.11.26j D8), each 0–1: how readily it follows the player off its goal, and how
        /// hard it drives toward it. One call makes a static dialogue <c>dialogue.v2</c>, which a v1-only kit cannot read.
        /// </summary>
        public NpcBuilder Disposition(double suggestibility, double initiative)
        {
            DispositionValue = new NpcDisposition(suggestibility, initiative);
            return this;
        }

        internal NpcDisposition? DispositionValue { get; private set; }

        internal DialogueNpc Build(string id, string name) =>
            new DialogueNpc(id, name, PersonaText, Languages.Count == 0 ? null : Languages.ToArray(), DispositionValue);
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

        /// <summary>An edge to another checkpoint, or to a declared ending by its id.</summary>
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
