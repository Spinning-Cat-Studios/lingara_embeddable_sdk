using System.Collections.Generic;
using System.Linq;
using Lingara.Embed.Json;

namespace Lingara.Embed.Dialogues
{
    /// <summary>
    /// A built procedural dialogue (ADR 3.11.26j D8): immutable, always <c>dialogue.v2</c>, <c>kind: procedural</c>.
    /// <see cref="ToCanonicalJson"/> is what <c>DialogueLoad</c> sends, held to the Rust builder's bytes by AC13.
    /// </summary>
    public sealed class ProceduralDialogue
    {
        private readonly ProceduralBuilder b;

        internal ProceduralDialogue(ProceduralBuilder builder) => b = builder;

        public string Id => b.Cast.Id;

        public IReadOnlyList<DialogueNpc> Npcs => b.Cast.Npcs;

        /// <summary>JCS over the normal form: no whitespace, no trailing newline.</summary>
        public string ToCanonicalJson() => CanonicalJson.Write(Tree());

        private JsonValue Tree()
        {
            var cast = b.Cast;
            var root = Obj(("schema", Str("dialogue.v2")), ("kind", Str("procedural")), ("id", Str(cast.Id)),
                ("pedagogy", Str(cast.PedagogyValue == Pedagogy.None ? "none" : "subtle")),
                ("npcs", Arr(cast.Npcs.Select(CanonicalJson.NpcTree))), ("facts", Arr(cast.Facts.Select(CanonicalJson.FactTree))),
                ("endings", Arr(cast.Endings.Select(CanonicalJson.EndingTree))), ("procedural", Spec()));
            Opt(root, "title", cast.TitleText);
            return root;
        }

        private JsonValue Spec()
        {
            var (planDepth, sketchDepth, maxNodes) = b.HorizonValue;
            var spec = Obj(("premise", Str(b.Premise)), ("disclosable", Arr(b.DisclosableKeys.Select(Str))),
                ("topics", Arr(b.Topics.Select(TopicTree))),
                ("atoms", Arr(b.AtomList.Select(a => Prefer(Obj(("atom", Str(GraphWords.AtomKinds.Name(a.Atom)))), a.Prefer)))),
                ("horizon", Obj(("plan_depth", Num(planDepth)), ("sketch_depth", Num(sketchDepth)), ("max_nodes", Num(maxNodes)))),
                ("termination", Termination(b.Rules)));
            NonEmpty(spec, "forbidden", b.Forbidden.Select(f => Ids(Obj(("about", Str(f.About))), "npcs", f.Npcs)));
            NonEmpty(spec, "goals", b.Goals.Select(GoalTree));
            return spec;
        }

        private static JsonValue TopicTree(ProceduralTopic t)
        {
            var o = Ids(Obj(("id", Str(t.Id)), ("about", Str(t.About))), "npcs", t.Npcs);
            Opt(o, "when", t.When);
            NonEmpty(o, "discloses", t.Discloses.Select(Str));
            return o;
        }

        private static JsonValue GoalTree(ProceduralGoal g)
        {
            var o = Obj(("id", Str(g.Id)), ("describe", Str(g.Describe)));
            Opt(o, "predicate", g.Predicate);
            Opt(o, "when", g.When);
            return o;
        }

        private static JsonValue Termination(TerminationBuilder t)
        {
            var o = Obj(("max_turns", Num(t.MaxTurns)), ("max_beats", Num(t.MaxBeats)),
                ("stall", Obj(("turns", Num(t.StallRule.Turns)), ("ending", Str(t.StallRule.Ending)))), ("on_limit", Str(t.OnLimitEnding)));
            if (t.SuccessRule is (string goal, string ending))
            {
                Add(o, "success", Obj(("goal", Str(goal)), ("ending", Str(ending))));
            }

            return o;
        }

        private static JsonValue Prefer(JsonValue atom, double? prefer)
        {
            if (prefer.HasValue)
            {
                Add(atom, "prefer", Num(prefer.Value));
            }

            return atom;
        }

        private static JsonValue Ids(JsonValue o, string key, IReadOnlyList<string>? ids)
        {
            if (ids != null)
            {
                Add(o, key, Arr(ids.Select(Str)));
            }

            return o;
        }

        private static void NonEmpty(JsonValue o, string key, IEnumerable<JsonValue> items)
        {
            var array = Arr(items);
            if (array.Items.Count > 0)
            {
                Add(o, key, array);
            }
        }

        private static void Opt(JsonValue o, string key, string? value)
        {
            if (value != null)
            {
                Add(o, key, Str(value));
            }
        }

        private static void Add(JsonValue o, string key, JsonValue value) => o.Members.Add(new KeyValuePair<string, JsonValue>(key, value));

        private static JsonValue Str(string s) => new JsonValue(JsonKind.String, s);

        private static JsonValue Num(double x) => new JsonValue(JsonKind.Number, x.ToString("R", System.Globalization.CultureInfo.InvariantCulture));

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
    }

    internal sealed record ProceduralTopic(string Id, string About, IReadOnlyList<string>? Npcs, string? When, IReadOnlyList<string> Discloses);

    internal sealed record ProceduralForbidden(string About, IReadOnlyList<string>? Npcs);

    internal sealed record ProceduralGoal(string Id, string Describe, string? Predicate, string? When);
}
