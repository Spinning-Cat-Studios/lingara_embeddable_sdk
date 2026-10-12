namespace Lingara.Embed.Events
{
    /// <summary>
    /// What a game polls (K1 D2): one nested class per K1 <c>EmbedEvent</c>
    /// variant, then <see cref="Unknown"/> for a type a newer core added. Four
    /// names differ from K1's on purpose (ADR 1.10.26aa D8): <c>stream</c> is
    /// <see cref="StreamChanged"/>, <c>lingara</c> is <see cref="LingaraReceived"/>
    /// and <c>error</c> is <see cref="Failed"/>. The wire tags stay K1's.
    /// </summary>
    public abstract class EmbedEvent
    {
        private EmbedEvent(string type) => Type = type;

        /// <summary>The wire tag: <c>token_needed</c>, <c>npc_turn</c>, …</summary>
        public string Type { get; }

        /// <summary>The event's JSON as polled, when <c>EmbedOptions.KeepRawJson</c> is set.</summary>
        public string? RawJson { get; internal set; }

        /// <summary>The core needs a token: answer with <c>ProvideToken</c> or <c>TokenUnavailable</c>.</summary>
        public sealed class TokenNeeded : EmbedEvent
        {
            public TokenNeeded() : base("token_needed") { }

            public RequestId Request { get; init; }

            public TokenReason Reason { get; init; }
        }

        public sealed class StreamChanged : EmbedEvent
        {
            public StreamChanged() : base("stream") { }

            public StreamState State { get; init; } = new StreamState.Paused();

            public string? Cursor { get; init; }
        }

        public sealed class LingaraReceived : EmbedEvent
        {
            public LingaraReceived() : base("lingara") { }

            public LingaraEvent Event { get; init; } = new LingaraEvent();
        }

        public sealed class WorldSent : EmbedEvent
        {
            public WorldSent() : base("world_sent") { }

            public RequestId Request { get; init; }

            public string EventId { get; init; } = string.Empty;

            public Reaction? Reaction { get; init; }
        }

        public sealed class Practice : EmbedEvent
        {
            public Practice() : base("practice") { }

            public SessionId Session { get; init; }

            public PracticeSnapshot Snapshot { get; init; } = new PracticeSnapshot();
        }

        public sealed class NpcTurn : EmbedEvent
        {
            public NpcTurn() : base("npc_turn") { }

            public DialogueId Dialogue { get; init; }

            public RequestId Request { get; init; }

            public NpcTurnUpdate Update { get; init; } = new NpcTurnUpdate.Opened();
        }

        /// <summary>An asynchronous failure; <c>Error.Request</c> names the command it answers.</summary>
        public sealed class Failed : EmbedEvent
        {
            public Failed() : base("error") { }

            public EmbedError Error { get; init; } = new EmbedError();
        }

        /// <summary>The core's diagnostics at or above <c>LogLevel</c>: route to the engine's log.</summary>
        public sealed class Log : EmbedEvent
        {
            public Log() : base("log") { }

            public LogLevel Level { get; init; }

            public string Target { get; init; } = string.Empty;

            public string Message { get; init; } = string.Empty;
        }

        /// <summary>The core closed itself; every later command is refused.</summary>
        public sealed class Closed : EmbedEvent
        {
            public Closed() : base("closed") { }

            public CloseReason Reason { get; init; }
        }

        /// <summary>A type this package does not know: its tag and its raw JSON.</summary>
        public sealed class Unknown : EmbedEvent
        {
            public Unknown(string type, string json) : base(type) => Json = json;

            public string Json { get; }
        }
    }
}
