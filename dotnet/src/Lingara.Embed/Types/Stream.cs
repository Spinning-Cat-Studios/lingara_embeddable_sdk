namespace Lingara.Embed
{
    /// <summary>The stream's state (K1 D5).</summary>
    public abstract class StreamState
    {
        private StreamState()
        {
        }

        public sealed class Connecting : StreamState
        {
        }

        /// <summary>An event has arrived since the tail was created.</summary>
        public sealed class Live : StreamState
        {
        }

        public sealed class Paused : StreamState
        {
        }

        public sealed class Down : StreamState
        {
            public EmbedError Error { get; init; } = new EmbedError();

            public long RetryInMs { get; init; }
        }

        /// <summary>A state a newer core added.</summary>
        public sealed class Unknown : StreamState
        {
            public string Type { get; init; } = string.Empty;
        }
    }

    /// <summary>One event off the Lingara stream (K1 D5). <see cref="DataJson"/> is its raw <c>data</c>.</summary>
    public sealed class LingaraEvent
    {
        public string Id { get; init; } = string.Empty;

        public string EventType { get; init; } = string.Empty;

        public string CreatedAt { get; init; } = string.Empty;

        public string DataJson { get; init; } = string.Empty;

        public LingaraEventDetail Detail { get; init; } = new LingaraEventDetail.Other();
    }

    public abstract class LingaraEventDetail
    {
        private LingaraEventDetail()
        {
        }

        public sealed class LessonPlanReady : LingaraEventDetail
        {
            public string PlanId { get; init; } = string.Empty;

            public string? Title { get; init; }

            public byte Level { get; init; }

            public string SourceLang { get; init; } = string.Empty;

            public string TargetLang { get; init; } = string.Empty;

            public bool Complete { get; init; }
        }

        public sealed class LessonPlanFailed : LingaraEventDetail
        {
            public string PlanId { get; init; } = string.Empty;

            public string Reason { get; init; } = string.Empty;
        }

        /// <summary>Every other type, newer ones included: read <see cref="LingaraEvent.DataJson"/>.</summary>
        public sealed class Other : LingaraEventDetail
        {
        }
    }

    /// <summary>K1's <c>status()</c>: persist <see cref="Cursor"/> at quit to resume the stream.</summary>
    public sealed class EmbedStatus
    {
        public StreamState Stream { get; init; } = new StreamState.Paused();

        public string? Cursor { get; init; }

        public bool NeedsToken { get; init; }

        public bool Closed { get; init; }
    }
}
