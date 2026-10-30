using System.Collections.Generic;

namespace Lingara.Embed
{
    /// <summary>K1's closed error code. <c>Unknown</c> is a kind a newer core added.</summary>
    public enum ErrorKind
    {
        Unknown = -1,
        Config,
        InvalidInput,
        TokenUnavailable,
        TokenRejected,
        Forbidden,
        Billing,
        NotFound,
        RateLimited,
        Unavailable,
        Network,
        Busy,
        InvalidState,
        UnknownId,
        Closed,
        Internal,
    }

    public enum TokenReason
    {
        Unknown = -1,
        Initial,
        Expiring,
        Rejected,
    }

    public enum LogLevel
    {
        Unknown = -1,
        Error,
        Warn,
        Info,
        Debug,
        Trace,
    }

    public enum CloseReason
    {
        Unknown = -1,
        Requested,
        Internal,
    }

    public enum PracticePhase
    {
        Unknown = -1,
        Requesting,
        Generating,
        Creating,
        Loading,
        Unanswered,
        AwaitingSelfGrade,
        Reviewed,
        SetComplete,
        Reporting,
        Reported,
        ReportFailed,
        Failed,
        Ended,
    }

    public enum PracticeOutcome
    {
        Unknown = -1,
        Correct,
        Incorrect,
        Revealed,
        SelfCorrect,
        SelfIncorrect,
        Skipped,
    }

    public enum DialoguePhase
    {
        Unknown = -1,
        Idle,
        Sending,
        Streaming,

        /// <summary>A graph dialogue whose story reached an outcome (ADR 28.10.26f).</summary>
        Ended,

        /// <summary>A stored dialogue still being fetched (ADR 28.10.26i).</summary>
        Loading,
    }

    public enum Speaker
    {
        Unknown = -1,
        Player,
        Npc,
    }

    /// <summary>
    /// Every failure the core reports (K1 D9). <see cref="Message"/> is English, for
    /// logs, never shown to a learner, and never holds a token.
    /// </summary>
    public sealed class EmbedError
    {
        public ErrorKind Kind { get; init; }

        /// <summary>The kind as K1 spells it on the wire, kept even when <see cref="Kind"/> is <c>Unknown</c>.</summary>
        public string KindName { get; init; } = string.Empty;

        public string Message { get; init; } = string.Empty;

        public int? Status { get; init; }

        public string? Code { get; init; }

        public long? RetryAfterMs { get; init; }

        public bool Retryable { get; init; }

        /// <summary>The command this failure answers, when it answers one.</summary>
        public RequestId? Request { get; init; }

        public override string ToString() => $"{KindName}: {Message}";
    }

    /// <summary>
    /// K1's snake_case words for each fieldless enum, as one table per enum: no
    /// reflection, so IL2CPP stripping and NativeAOT both keep them (D8).
    /// </summary>
    internal sealed class WordTable<T>
        where T : struct
    {
        private readonly KeyValuePair<string, T>[] words;
        private readonly T unknown;

        public WordTable(T unknown, params KeyValuePair<string, T>[] words)
        {
            this.unknown = unknown;
            this.words = words;
        }

        public T Parse(string? word)
        {
            foreach (var pair in words)
            {
                if (pair.Key == word)
                {
                    return pair.Value;
                }
            }

            return unknown;
        }

        public string Name(T value)
        {
            foreach (var pair in words)
            {
                if (EqualityComparer<T>.Default.Equals(pair.Value, value))
                {
                    return pair.Key;
                }
            }

            return "unknown";
        }
    }

    internal static class Words
    {
        public static readonly WordTable<ErrorKind> ErrorKinds = new WordTable<ErrorKind>(
            ErrorKind.Unknown,
            W("config", ErrorKind.Config),
            W("invalid_input", ErrorKind.InvalidInput),
            W("token_unavailable", ErrorKind.TokenUnavailable),
            W("token_rejected", ErrorKind.TokenRejected),
            W("forbidden", ErrorKind.Forbidden),
            W("billing", ErrorKind.Billing),
            W("not_found", ErrorKind.NotFound),
            W("rate_limited", ErrorKind.RateLimited),
            W("unavailable", ErrorKind.Unavailable),
            W("network", ErrorKind.Network),
            W("busy", ErrorKind.Busy),
            W("invalid_state", ErrorKind.InvalidState),
            W("unknown_id", ErrorKind.UnknownId),
            W("closed", ErrorKind.Closed),
            W("internal", ErrorKind.Internal));

        public static readonly WordTable<TokenReason> TokenReasons = new WordTable<TokenReason>(
            TokenReason.Unknown, W("initial", TokenReason.Initial), W("expiring", TokenReason.Expiring), W("rejected", TokenReason.Rejected));

        public static readonly WordTable<LogLevel> LogLevels = new WordTable<LogLevel>(
            LogLevel.Unknown,
            W("error", LogLevel.Error),
            W("warn", LogLevel.Warn),
            W("info", LogLevel.Info),
            W("debug", LogLevel.Debug),
            W("trace", LogLevel.Trace));

        public static readonly WordTable<CloseReason> CloseReasons = new WordTable<CloseReason>(
            CloseReason.Unknown, W("requested", CloseReason.Requested), W("internal", CloseReason.Internal));

        public static readonly WordTable<PracticePhase> PracticePhases = new WordTable<PracticePhase>(
            PracticePhase.Unknown,
            W("requesting", PracticePhase.Requesting),
            W("generating", PracticePhase.Generating),
            W("creating", PracticePhase.Creating),
            W("loading", PracticePhase.Loading),
            W("unanswered", PracticePhase.Unanswered),
            W("awaiting_self_grade", PracticePhase.AwaitingSelfGrade),
            W("reviewed", PracticePhase.Reviewed),
            W("set_complete", PracticePhase.SetComplete),
            W("reporting", PracticePhase.Reporting),
            W("reported", PracticePhase.Reported),
            W("report_failed", PracticePhase.ReportFailed),
            W("failed", PracticePhase.Failed),
            W("ended", PracticePhase.Ended));

        public static readonly WordTable<PracticeOutcome> PracticeOutcomes = new WordTable<PracticeOutcome>(
            PracticeOutcome.Unknown,
            W("correct", PracticeOutcome.Correct),
            W("incorrect", PracticeOutcome.Incorrect),
            W("revealed", PracticeOutcome.Revealed),
            W("self_correct", PracticeOutcome.SelfCorrect),
            W("self_incorrect", PracticeOutcome.SelfIncorrect),
            W("skipped", PracticeOutcome.Skipped));

        public static readonly WordTable<DialoguePhase> DialoguePhases = new WordTable<DialoguePhase>(
            DialoguePhase.Unknown, W("idle", DialoguePhase.Idle), W("sending", DialoguePhase.Sending), W("streaming", DialoguePhase.Streaming), W("ended", DialoguePhase.Ended),
            W("loading", DialoguePhase.Loading));

        public static readonly WordTable<Speaker> Speakers = new WordTable<Speaker>(
            Speaker.Unknown, W("player", Speaker.Player), W("npc", Speaker.Npc));

        private static KeyValuePair<string, T> W<T>(string word, T value) => new KeyValuePair<string, T>(word, value);
    }
}
