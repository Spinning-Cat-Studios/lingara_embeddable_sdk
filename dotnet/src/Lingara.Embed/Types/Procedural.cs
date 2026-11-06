using System;
using System.Collections.Generic;

namespace Lingara.Embed
{
    /// <summary>What became of a procedural run's plan (ADR 3.11.26j D9).</summary>
    public enum PlanOutcome
    {
        Unknown = -1,

        /// <summary>Its fragment is the run's new frontier.</summary>
        Accepted,

        /// <summary>The run kept the frontier it had; see <see cref="PlanKeptReason"/>.</summary>
        Kept,
    }

    /// <summary>Why a run kept its frontier (D5).</summary>
    public enum PlanKeptReason
    {
        Unknown = -1,

        /// <summary>The run had moved on while the plan was in flight.</summary>
        Stale,

        /// <summary>The fragment failed the core's own check, or the server's.</summary>
        Rejected,

        /// <summary>The planner was dark, refused, or could not be reached.</summary>
        Unavailable,

        /// <summary>No answer within the core's wait, or the server's.</summary>
        Timeout,

        /// <summary>The planner's answer could not be read.</summary>
        Unparseable,
    }

    /// <summary>The five beat templates a procedural dialogue composes (ADR 3.11.26g §5).</summary>
    public enum AtomKind
    {
        Unknown = -1,
        Open,
        Elicit,
        Reveal,
        Practise,
        Close,
    }

    /// <summary>A beat a procedural run crossed, as the planner reads it.</summary>
    public sealed class VisitedBeat
    {
        public AtomKind Atom { get; init; }

        public string? Topic { get; init; }
    }

    /// <summary>One beat of a procedural run's whole path (ADR 3.11.26j D13): ids and counters, never text.</summary>
    public sealed class TracedBeat
    {
        /// <summary>The beat id, never <c>‹beat›_again</c>.</summary>
        public string Id { get; init; } = string.Empty;

        public AtomKind? Atom { get; init; }

        public string? Topic { get; init; }

        public string? Goal { get; init; }

        public string? Ending { get; init; }

        /// <summary>The edge that left the beat; null while it is current, or when a rule with no edge ended the run.</summary>
        public string? LeftByEdge { get; init; }

        /// <summary>How the beat was entered.</summary>
        public CrossedVia Via { get; init; }

        public int EnteredTurn { get; init; }

        /// <summary>The turn count when it was left; null while it is current.</summary>
        public int? LeftTurn { get; init; }

        /// <summary>The accepted frontier it was entered under, 1 for the first.</summary>
        public int PlanSeq { get; init; }
    }

    /// <summary>A procedural run's counters and trace (ADR 3.11.26j D9), saved beside its frontier.</summary>
    public sealed class ProceduralProgress
    {
        public IReadOnlyList<VisitedBeat> Visited { get; init; } = Array.Empty<VisitedBeat>();

        public IReadOnlyList<TracedBeat> Trace { get; init; } = Array.Empty<TracedBeat>();

        public int Plans { get; init; }

        public int NextPrefix { get; init; }

        public int FrontierSeq { get; init; }

        public int Beats { get; init; }

        public int Turns { get; init; }

        public int SinceCrossing { get; init; }
    }

    /// <summary>
    /// A procedural run's half of a snapshot: passed back as <see cref="GraphSession.Frontier"/> and
    /// <see cref="GraphSession.Progress"/>, with the snapshot's checkpoint, facts and window, it restores the run.
    /// </summary>
    public sealed class ProceduralSnapshot
    {
        /// <summary>The frontier the run walks, a <c>PlanFragment</c> as JSON text.</summary>
        public string Frontier { get; init; } = string.Empty;

        public ProceduralProgress Progress { get; init; } = new ProceduralProgress();
    }
}
