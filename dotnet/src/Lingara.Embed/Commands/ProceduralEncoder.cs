using Lingara.Embed.Json;

namespace Lingara.Embed.Commands
{
    /// <summary>A procedural run's saved counters and trace in K1's JSON (ADR 3.11.26j D9), for <c>DialogueLoad</c>'s session.</summary>
    internal static class ProceduralEncoder
    {
        public static JsonWriter Progress(JsonWriter w, ProceduralProgress p)
        {
            w.BeginObject().Name("visited").BeginArray();
            foreach (var beat in p.Visited)
            {
                w.BeginObject().Prop("atom", GraphWords.AtomKinds.Name(beat.Atom)).Prop("topic", beat.Topic).EndObject();
            }

            w.EndArray().Name("trace").BeginArray();
            foreach (var beat in p.Trace)
            {
                Traced(w, beat);
            }

            return w.EndArray()
                .Prop("plans", p.Plans).Prop("next_prefix", p.NextPrefix).Prop("frontier_seq", p.FrontierSeq)
                .Prop("beats", p.Beats).Prop("turns", p.Turns).Prop("since_crossing", p.SinceCrossing)
                .EndObject();
        }

        private static void Traced(JsonWriter w, TracedBeat beat)
        {
            w.BeginObject().Prop("id", beat.Id).Prop("atom", beat.Atom.HasValue ? GraphWords.AtomKinds.Name(beat.Atom.Value) : null)
                .Prop("topic", beat.Topic).Prop("goal", beat.Goal).Prop("ending", beat.Ending).Prop("left_by_edge", beat.LeftByEdge)
                .Prop("via", GraphWords.CrossedVias.Name(beat.Via)).Prop("entered_turn", beat.EnteredTurn).Name("left_turn");
            if (beat.LeftTurn.HasValue)
            {
                w.Number(beat.LeftTurn.Value);
            }
            else
            {
                w.Null();
            }

            w.Prop("plan_seq", beat.PlanSeq).EndObject();
        }
    }
}
