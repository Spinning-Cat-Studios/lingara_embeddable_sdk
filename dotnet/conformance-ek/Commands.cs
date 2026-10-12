using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json.Nodes;
using Lingara.Embed;

namespace Lingara.Embed.Conformance
{
    /// <summary>
    /// The command-type table (ADR 1.10.26aa D12): each K1 command <c>type</c> a
    /// case can name, sent through its typed method. A type with no row fails the
    /// case, so a command K1 adds fails here until C# has a method for it.
    /// </summary>
    internal static class Commands
    {
        private delegate void Send(LingaraEmbedClient c, JsonObject o, Action<long> opened);

        private static readonly Dictionary<string, Send> Table = new Dictionary<string, Send>
        {
            ["provide_token"] = (c, o, _) => c.ProvideToken(Str(o, "token"), Str(o, "expires_at")),
            ["provide_token_for"] = (c, o, _) => c.ProvideTokenFor(Str(o, "token"), (uint)Long(o, "expires_in_secs")),
            ["token_unavailable"] = (c, o, _) => c.TokenUnavailable(Str(o, "reason")),
            ["set_token_endpoint"] = (c, o, _) => c.SetTokenEndpoint(Str(o, "url"), Headers(o["headers"] as JsonArray)),
            ["stream_pause"] = (c, _, _) => c.StreamPause(),
            ["stream_resume"] = (c, _, _) => c.StreamResume(),
            ["send_context"] = (c, o, _) => c.SendContext(Context(Obj(o, "context"))),
            ["practice_start"] = (c, o, opened) => opened(c.PracticeStart(Source(Obj(o, "source"))).Value),
            ["practice_snapshot"] = (c, o, _) => c.GetPracticeSnapshot(Session(o)),
            ["practice_answer"] = (c, o, _) => c.PracticeAnswer(Session(o), Str(o, "answer")),
            ["practice_choose"] = (c, o, _) => c.PracticeChoose(Session(o), (uint)Long(o, "option")),
            ["practice_reveal"] = (c, o, _) => c.PracticeReveal(Session(o)),
            ["practice_self_grade"] = (c, o, _) => c.PracticeSelfGrade(Session(o), o["correct"]!.GetValue<bool>()),
            ["practice_next"] = (c, o, _) => c.PracticeNext(Session(o)),
            ["practice_finish"] = (c, o, _) => c.PracticeFinish(Session(o)),
            ["dialogue_open"] = (c, o, opened) => opened(c.DialogueOpen(Dialogue(Obj(o, "config"))).Value),
            ["dialogue_say"] = (c, o, _) => c.DialogueSay(DialogueId(o), Str(o, "line")),
            ["dialogue_set_level"] = (c, o, _) => c.DialogueSetLevel(DialogueId(o), (byte)Long(o, "level")),
            ["dialogue_set_scene"] = (c, o, _) => c.DialogueSetScene(DialogueId(o), OptStr(o, "scene")),
            ["dialogue_cancel"] = (c, o, _) => c.DialogueCancel(DialogueId(o)),
            ["dialogue_close"] = (c, o, _) => c.DialogueClose(DialogueId(o)),
            ["dialogue_snapshot"] = (c, o, _) => c.GetDialogueSnapshot(DialogueId(o)),
        };

        public static void Run(LingaraEmbedClient client, JsonObject command, Action<long> opened)
        {
            var type = Str(command, "type");
            var send = Table.TryGetValue(type, out var row) ? row : throw new InvalidOperationException($"no C# method for the command `{type}`");
            send(client, command, opened);
        }

        /// <summary>A case's <c>start.config</c>, whose fields are closed (CONTRACT.md); <c>base_url</c> is the server's.</summary>
        public static EmbedOptions Options(string server, JsonObject config)
        {
            var options = new EmbedOptions { BaseUrl = server, KeepRawJson = true };
            foreach (var (key, value) in config)
            {
                var set = Config.TryGetValue(key, out var row) ? row : throw new InvalidOperationException($"start.config has no field `{key}`");
                set(options, value!);
            }

            return options;
        }

        /// <summary>CONTRACT.md's closed <c>start.config</c> fields, each onto its option.</summary>
        private static readonly Dictionary<string, Action<EmbedOptions, JsonNode>> Config = new Dictionary<string, Action<EmbedOptions, JsonNode>>
        {
            ["stream_on_start"] = (o, v) => o.StreamOnStart = v.GetValue<bool>(),
            ["queue_capacity"] = (o, v) => o.QueueCapacity = v.GetValue<uint>(),
            ["stream_idle_timeout_ms"] = (o, v) => o.StreamIdleTimeoutMs = v.GetValue<uint>(),
            ["api_version"] = (o, v) => o.ApiVersion = v.GetValue<string>(),
            ["user_agent_suffix"] = (o, v) => o.UserAgentSuffix = v.GetValue<string>(),
            ["resume_cursor"] = (o, v) => o.ResumeCursor = v.GetValue<string>(),
            ["event_types"] = (o, v) => o.EventTypes = Strings(v as JsonArray),
            ["log_level"] = (o, v) => o.LogLevel = Level(v.GetValue<string>()),
        };

        /// <summary>K1's <c>EmbedError</c> JSON, for a refusal the harness turns into an event.</summary>
        public static JsonObject ErrorJson(EmbedError e) => new JsonObject
        {
            ["kind"] = e.KindName,
            ["message"] = e.Message,
            ["status"] = e.Status,
            ["code"] = e.Code,
            ["retry_after_ms"] = e.RetryAfterMs,
            ["retryable"] = e.Retryable,
            ["request"] = e.Request?.Value,
        };

        private static LogLevel Level(string word) => word switch
        {
            "error" => LogLevel.Error,
            "warn" => LogLevel.Warn,
            "info" => LogLevel.Info,
            "debug" => LogLevel.Debug,
            "trace" => LogLevel.Trace,
            _ => throw new InvalidOperationException($"no log level `{word}`"),
        };

        private static string Str(JsonObject o, string key) => o[key]?.GetValue<string>() ?? throw new InvalidOperationException($"`{key}` is missing");

        private static string? OptStr(JsonObject o, string key) => o[key]?.GetValue<string>();

        private static long Long(JsonObject o, string key) => o[key]?.GetValue<long>() ?? throw new InvalidOperationException($"`{key}` is missing");

        private static JsonObject Obj(JsonObject o, string key) => o[key] as JsonObject ?? throw new InvalidOperationException($"`{key}` is not an object");

        private static SessionId Session(JsonObject o) => new SessionId(Long(o, "session"));

        private static DialogueId DialogueId(JsonObject o) => new DialogueId(Long(o, "dialogue"));

        private static List<string> Strings(JsonArray? a) => a?.Select(n => n!.GetValue<string>()).ToList() ?? new List<string>();

        private static List<Header> Headers(JsonArray? a) =>
            a?.Select(h => new Header(Str(h!.AsObject(), "name"), Str(h!.AsObject(), "value"))).ToList() ?? new List<Header>();

        private static Npc? Npc(JsonNode? n) => n is JsonObject o ? new Npc { Name = Str(o, "name"), Persona = OptStr(o, "persona") } : null;

        private static WorldContext Context(JsonObject o) => new WorldContext
        {
            Scene = Str(o, "scene"),
            SourceLang = Str(o, "source_lang"),
            TargetLang = Str(o, "target_lang"),
            Level = (byte)Long(o, "level"),
            Npc = Npc(o["npc"]),
            Tags = Strings(o["tags"] as JsonArray),
            Generate = o["generate"]?.GetValue<bool>() ?? false,
        };

        private static PracticeSource Source(JsonObject o) => Str(o, "type") switch
        {
            "request" => new PracticeSource.Request
            {
                Topic = Str(o, "topic"),
                SourceLang = Str(o, "source_lang"),
                TargetLang = Str(o, "target_lang"),
                Level = (byte)Long(o, "level"),
                Tags = Strings(o["tags"] as JsonArray),
            },
            "create" => new PracticeSource.Create { Context = Str(o, "context"), SourceLang = Str(o, "source_lang"), TargetLang = Str(o, "target_lang"), Level = (byte)Long(o, "level") },
            "plan" => new PracticeSource.Plan { PlanId = Str(o, "plan_id"), Report = o["report"]?.GetValue<bool>() ?? true },
            var other => throw new InvalidOperationException($"no practice source `{other}`"),
        };

        private static DialogueConfig Dialogue(JsonObject o) => new DialogueConfig
        {
            Npc = Npc(o["npc"]) ?? throw new InvalidOperationException("`npc` is missing"),
            Scene = OptStr(o, "scene"),
            SourceLang = Str(o, "source_lang"),
            TargetLang = Str(o, "target_lang"),
            Level = (byte)Long(o, "level"),
            Window = (o["window"] as JsonArray)?.Select(e => new WindowEntry
            {
                Speaker = Str(e!.AsObject(), "speaker") == "npc" ? Speaker.Npc : Speaker.Player,
                Text = Str(e!.AsObject(), "text"),
            }).ToList() ?? new List<WindowEntry>(),
        };
    }
}
