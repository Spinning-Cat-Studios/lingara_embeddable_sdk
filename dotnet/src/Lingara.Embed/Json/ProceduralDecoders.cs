using System.Globalization;

namespace Lingara.Embed.Json
{
    /// <summary>A procedural run's types out of K1's JSON (ADR 3.11.26j D9, D13), beside <see cref="TypeDecoders"/>.</summary>
    internal static class ProceduralDecoders
    {
        public static ProceduralSnapshot Snapshot(JsonValue v) => new ProceduralSnapshot
        {
            Frontier = v.Str("frontier"),
            Progress = Progress(v.Obj("progress")),
        };

        public static ProceduralProgress Progress(JsonValue v) => new ProceduralProgress
        {
            Visited = TypeDecoders.List(v["visited"], b => new VisitedBeat { Atom = GraphWords.AtomKinds.Parse(b.Str("atom")), Topic = b.OptStr("topic") }),
            Trace = TypeDecoders.List(v["trace"], Traced),
            Plans = Int(v, "plans"),
            NextPrefix = Int(v, "next_prefix"),
            FrontierSeq = Int(v, "frontier_seq"),
            Beats = Int(v, "beats"),
            Turns = Int(v, "turns"),
            SinceCrossing = Int(v, "since_crossing"),
        };

        public static TracedBeat Traced(JsonValue v) => new TracedBeat
        {
            Id = v.Str("id"),
            Atom = v["atom"].IsNull ? (AtomKind?)null : GraphWords.AtomKinds.Parse(v.Str("atom")),
            Topic = v.OptStr("topic"),
            Goal = v.OptStr("goal"),
            Ending = v.OptStr("ending"),
            LeftByEdge = v.OptStr("left_by_edge"),
            Via = GraphWords.CrossedVias.Parse(v.Str("via")),
            EnteredTurn = Int(v, "entered_turn"),
            LeftTurn = v["left_turn"].IsNull ? (int?)null : Int(v, "left_turn"),
            PlanSeq = Int(v, "plan_seq"),
        };

        /// <summary>A counter, absent read as 0 as the core's <c>serde(default)</c> does.</summary>
        private static int Int(JsonValue v, string key) =>
            v[key].IsNull ? 0 : int.Parse(v[key].Text ?? "0", NumberStyles.Integer, CultureInfo.InvariantCulture);
    }
}
