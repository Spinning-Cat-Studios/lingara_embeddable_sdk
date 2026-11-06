using System;
using System.Collections.Generic;

namespace Lingara.Embed.Dialogues
{
    /// <summary>
    /// Writes a procedural dialogue in C# (ADR 3.11.26j D8), for Unity and Godot C#: always <c>dialogue.v2</c>,
    /// <c>kind: procedural</c>. Nobody writes its checkpoints: the builder draws the box the planner fills (the
    /// premise, what may come up, what may be disclosed, and when the run stops). Atoms are an enum, never free
    /// text. <see cref="Build"/> checks the structure, the two mistakes the planner cannot recover from among them;
    /// the rest of the crate's rules run in <c>DialogueLoad</c> and <c>DialogueValidate</c>.
    /// </summary>
    public sealed class ProceduralBuilder
    {
        public ProceduralBuilder(string id, string premise)
        {
            Cast = new DialogueBuilder(id, string.Empty);
            Premise = premise;
        }

        /// <summary>The id, title, pedagogy, cast, facts and endings: the static builder's own parts.</summary>
        internal DialogueBuilder Cast { get; }

        internal string Premise { get; }

        internal List<string> DisclosableKeys { get; } = new List<string>();

        internal List<ProceduralTopic> Topics { get; } = new List<ProceduralTopic>();

        internal List<ProceduralForbidden> Forbidden { get; } = new List<ProceduralForbidden>();

        internal List<ProceduralGoal> Goals { get; } = new List<ProceduralGoal>();

        internal List<(AtomKind Atom, double? Prefer)> AtomList { get; } = new List<(AtomKind Atom, double? Prefer)>();

        internal (int PlanDepth, int SketchDepth, int MaxNodes) HorizonValue { get; private set; } = (2, 1, 10);

        internal TerminationBuilder Rules { get; private set; } = new TerminationBuilder(1, 1);

        public ProceduralBuilder Title(string title) => With(() => Cast.Title(title));

        public ProceduralBuilder Pedagogy(Lingara.Embed.Pedagogy pedagogy) => With(() => Cast.Pedagogy(pedagogy));

        /// <summary>A member of the cast; <paramref name="profile"/> sets its persona, languages and disposition.</summary>
        public ProceduralBuilder Npc(string id, string name, Action<NpcBuilder>? profile = null) => With(() => Cast.Npc(id, name, profile));

        /// <summary>A fact; <paramref name="defaultValue"/> is a bool, a number or a string, matching <paramref name="type"/>.</summary>
        public ProceduralBuilder Fact(string key, FactType type, object? defaultValue = null) => With(() => Cast.Fact(key, type, defaultValue));

        /// <summary>A named ending; a procedural run ends only on its declared endings, plus <c>abandoned</c>.</summary>
        public ProceduralBuilder Ending(string id, string? label = null) => With(() => Cast.Ending(id, label));

        /// <summary>The fact keys a Reveal beat may disclose: the knowledge pool.</summary>
        public ProceduralBuilder Disclosable(params string[] keys) => With(() => DisclosableKeys.AddRange(keys));

        /// <summary>Something that may come up; <paramref name="spec"/> sets with whom, when, and what it discloses.</summary>
        public ProceduralBuilder Topic(string id, string about, Action<TopicBuilder>? spec = null)
        {
            var topic = new TopicBuilder();
            spec?.Invoke(topic);
            Topics.Add(new ProceduralTopic(id, about, topic.NpcIds, topic.WhenText, topic.Disclosed.ToArray()));
            return this;
        }

        /// <summary>A subject never to raise: planner guidance, not a guarantee. No <paramref name="npcs"/> is every cast member.</summary>
        public ProceduralBuilder Forbid(string about, params string[] npcs) =>
            With(() => Forbidden.Add(new ProceduralForbidden(about, npcs.Length == 0 ? null : npcs)));

        /// <summary>A named success target: <paramref name="predicate"/> the game decides, <paramref name="when"/> the judge does.</summary>
        public ProceduralBuilder Goal(string id, string describe, string? predicate = null, string? when = null) =>
            With(() => Goals.Add(new ProceduralGoal(id, describe, predicate, when)));

        /// <summary>The atoms the planner may compose, in order; <c>Open</c> and <c>Close</c> are required.</summary>
        public ProceduralBuilder Atoms(params AtomKind[] atoms) => With(() => Array.ForEach(atoms, a => AtomList.Add((a, null))));

        /// <summary>A planner hint, 0–1, on an atom already listed.</summary>
        public ProceduralBuilder Prefer(AtomKind atom, double prefer) => With(() =>
        {
            var at = AtomList.FindIndex(a => a.Atom == atom);
            if (at >= 0)
            {
                AtomList[at] = (atom, prefer);
            }
        });

        public ProceduralBuilder Horizon(int planDepth, int sketchDepth, int maxNodes) => With(() => HorizonValue = (planDepth, sketchDepth, maxNodes));

        /// <summary>When a run stops: both limits are required; <paramref name="rules"/> sets success, stall and on-limit.</summary>
        public ProceduralBuilder Termination(int maxTurns, int maxBeats, Action<TerminationBuilder>? rules = null)
        {
            Rules = new TerminationBuilder(maxTurns, maxBeats);
            rules?.Invoke(Rules);
            return this;
        }

        /// <summary>The document, once the structural checks pass.</summary>
        /// <exception cref="ArgumentException">A structural fault, named by its JSON path.</exception>
        public ProceduralDialogue Build()
        {
            ProceduralChecks.Run(this);
            return new ProceduralDialogue(this);
        }

        private ProceduralBuilder With(Action change)
        {
            change();
            return this;
        }
    }

    /// <summary>A topic's audience, condition and disclosures.</summary>
    public sealed class TopicBuilder
    {
        internal string? WhenText { get; private set; }

        internal IReadOnlyList<string>? NpcIds { get; private set; }

        internal List<string> Disclosed { get; } = new List<string>();

        /// <summary>A K1 predicate over facts; the topic is open only while it holds.</summary>
        public TopicBuilder OpenWhen(string predicate)
        {
            WhenText = predicate;
            return this;
        }

        /// <summary>The cast ids that may raise it; never called, every cast member.</summary>
        public TopicBuilder Npcs(params string[] ids)
        {
            NpcIds = ids;
            return this;
        }

        /// <summary>The facts a Reveal beat on this topic discloses: within the pool.</summary>
        public TopicBuilder Discloses(params string[] keys)
        {
            Disclosed.AddRange(keys);
            return this;
        }
    }

    /// <summary>The rules that end a run, past its two limits.</summary>
    public sealed class TerminationBuilder
    {
        internal TerminationBuilder(int maxTurns, int maxBeats)
        {
            MaxTurns = maxTurns;
            MaxBeats = maxBeats;
        }

        internal int MaxTurns { get; }

        internal int MaxBeats { get; }

        internal (string Goal, string Ending)? SuccessRule { get; private set; }

        internal (int Turns, string Ending) StallRule { get; private set; } = (6, DialogueOutcome.Abandoned);

        internal string OnLimitEnding { get; private set; } = DialogueOutcome.Abandoned;

        /// <summary>When <paramref name="goal"/>'s predicate holds, the run ends on <paramref name="ending"/>.</summary>
        public TerminationBuilder Success(string goal, string ending)
        {
            SuccessRule = (goal, ending);
            return this;
        }

        /// <summary><paramref name="turns"/> turns with no crossing end the run on <paramref name="ending"/>.</summary>
        public TerminationBuilder Stall(int turns, string ending)
        {
            StallRule = (turns, ending);
            return this;
        }

        /// <summary>What either limit ends on; never called, <c>abandoned</c>.</summary>
        public TerminationBuilder OnLimit(string ending)
        {
            OnLimitEnding = ending;
            return this;
        }
    }
}
