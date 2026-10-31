using System.Collections.Generic;
using Lingara.Embed.Commands;
using Lingara.Embed.Json;

namespace Lingara.Embed
{
    /// <summary>
    /// K1's <c>EmbedConfig</c>, field for field (ADR 1.10.26aa D8). A property left
    /// null is omitted from the config, so K1's own default applies and is never
    /// restated here. It never holds a token: tokens arrive by <c>ProvideToken</c>,
    /// or from <see cref="TokenEndpoint"/>.
    /// </summary>
    public sealed class EmbedOptions
    {
        /// <summary>Default <c>https://api.getlingara.com</c>.</summary>
        public string? BaseUrl { get; set; }

        /// <summary>K1's Http token source; null polls for tokens with <c>TokenNeeded</c> (D6).</summary>
        public TokenEndpoint? TokenEndpoint { get; set; }

        /// <summary>Follows <c>lingara-dotnet/&lt;version&gt;</c> in the user agent, e.g. <c>unity/6000.0.23f1</c>.</summary>
        public string? UserAgentSuffix { get; set; }

        public LogLevel? LogLevel { get; set; }

        /// <summary><c>false</c> starts paused: a game that never listens pays nothing.</summary>
        public bool? StreamOnStart { get; set; }

        /// <summary>A cursor saved from <c>Status</c> or a <c>StreamChanged</c> event.</summary>
        public string? ResumeCursor { get; set; }

        /// <summary>The stream's event types; empty or null is every type.</summary>
        public IReadOnlyList<string>? EventTypes { get; set; }

        /// <summary>The <c>Lingara-Version</c> pin.</summary>
        public string? ApiVersion { get; set; }

        /// <summary>
        /// The platform embed version this build was registered against, e.g. <c>1.1.0</c>
        /// (ADR 30.10.26p). The Http source sends it to your token endpoint as
        /// <c>Lingara-Embed-Version</c>; a polling game sends that header itself.
        /// </summary>
        public string? EmbedVersion { get; set; }

        public uint? QueueCapacity { get; set; }

        public uint? TokenWaitTimeoutMs { get; set; }

        public uint? StreamIdleTimeoutMs { get; set; }

        /// <summary>Keep each event's JSON on <c>EmbedEvent.RawJson</c>: for the EK harness and debugging.</summary>
        public bool KeepRawJson { get; set; }

        /// <summary>The config <c>lingara_embed_new</c> takes. <paramref name="kit"/> leads the user agent suffix.</summary>
        internal byte[] ToConfigJson(string kit)
        {
            var w = new JsonWriter().BeginObject();
            OptString(w, "base_url", BaseUrl);
            if (TokenEndpoint != null)
            {
                w.Name("token_source");
                CommandEncoder.Endpoint(w, TokenEndpoint);
            }

            w.Prop("user_agent_suffix", string.IsNullOrEmpty(UserAgentSuffix) ? kit : kit + " " + UserAgentSuffix);
            OptString(w, "log_level", LogLevel.HasValue ? Words.LogLevels.Name(LogLevel.Value) : null);
            if (StreamOnStart.HasValue)
            {
                w.Prop("stream_on_start", StreamOnStart.Value);
            }

            OptString(w, "resume_cursor", ResumeCursor);
            if (EventTypes != null)
            {
                w.Name("event_types");
                CommandEncoder.Strings(w, EventTypes);
            }

            OptString(w, "api_version", ApiVersion);
            OptString(w, "embed_version", EmbedVersion);
            OptNumber(w, "queue_capacity", QueueCapacity);
            OptNumber(w, "token_wait_timeout_ms", TokenWaitTimeoutMs);
            OptNumber(w, "stream_idle_timeout_ms", StreamIdleTimeoutMs);
            return w.EndObject().ToUtf8();
        }

        private static void OptString(JsonWriter w, string name, string? value)
        {
            if (value != null)
            {
                w.Prop(name, value);
            }
        }

        private static void OptNumber(JsonWriter w, string name, uint? value)
        {
            if (value.HasValue)
            {
                w.Prop(name, value.Value);
            }
        }
    }
}
