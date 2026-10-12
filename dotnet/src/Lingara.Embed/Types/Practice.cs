using System;
using System.Collections.Generic;

namespace Lingara.Embed
{
    /// <summary>Where a practice session's plan comes from (K1 D7).</summary>
    public abstract class PracticeSource
    {
        private PracticeSource()
        {
        }

        /// <summary>A plan generated for a topic.</summary>
        public sealed class Request : PracticeSource
        {
            public string Topic { get; init; } = string.Empty;

            public string SourceLang { get; init; } = string.Empty;

            public string TargetLang { get; init; } = string.Empty;

            public byte Level { get; init; }

            public IReadOnlyList<string> Tags { get; init; } = Array.Empty<string>();
        }

        /// <summary>A plan created from a scene's context.</summary>
        public sealed class Create : PracticeSource
        {
            public string Context { get; init; } = string.Empty;

            public string SourceLang { get; init; } = string.Empty;

            public string TargetLang { get; init; } = string.Empty;

            public byte Level { get; init; }
        }

        /// <summary>A plan the game holds. A demo plan must be started with <c>Report = false</c>.</summary>
        public sealed class Plan : PracticeSource
        {
            public string PlanId { get; init; } = string.Empty;

            public bool Report { get; init; } = true;
        }
    }

    public sealed class PracticeSnapshot
    {
        public PracticePhase Phase { get; init; }

        public PlanSummary? Plan { get; init; }

        public uint SetIndex { get; init; }

        public uint SetCount { get; init; }

        public uint QuestionIndex { get; init; }

        public uint QuestionCount { get; init; }

        public PracticeQuestion? Question { get; init; }

        public AnswerResult? LastResult { get; init; }

        public EmbedError? Failure { get; init; }
    }

    public sealed class PlanSummary
    {
        public string Id { get; init; } = string.Empty;

        public string? Title { get; init; }

        public byte Level { get; init; }

        public string SourceLang { get; init; } = string.Empty;

        public string TargetLang { get; init; } = string.Empty;

        public IReadOnlyList<PlanWord> Vocabulary { get; init; } = Array.Empty<PlanWord>();
    }

    public sealed class PlanWord
    {
        public string Word { get; init; } = string.Empty;

        public string Translation { get; init; } = string.Empty;

        public string? Pronunciation { get; init; }
    }

    public sealed class PracticeQuestion
    {
        /// <summary>The plan's free <c>type</c> string.</summary>
        public string Kind { get; init; } = string.Empty;

        public string Prompt { get; init; } = string.Empty;

        /// <summary>Present on a multiple-choice question: answer with <c>PracticeChoose</c>.</summary>
        public IReadOnlyList<string>? Options { get; init; }

        public string? Hint { get; init; }

        /// <summary>Graded by the learner after seeing the model answer (K1 D7).</summary>
        public bool SelfGraded { get; init; }
    }

    /// <summary>The last answer's result. <see cref="Outcome"/> is null while a self-graded question waits.</summary>
    public sealed class AnswerResult
    {
        public PracticeOutcome? Outcome { get; init; }

        public string Expected { get; init; } = string.Empty;

        public string Explanation { get; init; } = string.Empty;
    }
}
