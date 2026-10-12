using System.Collections.Generic;
using Lingara.Embed.Json;

namespace Lingara.Embed.Commands
{
    /// <summary>
    /// One writer per K1 <c>Command</c> (ADR 1.10.26aa D2): the JSON object
    /// <c>lingara_embed_execute</c> takes, in K1's own shape. Every optional field
    /// is written, as null when absent, so each command's bytes have one shape.
    /// </summary>
    internal static class CommandEncoder
    {
        public static byte[] ProvideToken(string token, string expiresAt) =>
            Begin("provide_token").Prop("token", token).Prop("expires_at", expiresAt).EndObject().ToUtf8();

        public static byte[] ProvideTokenFor(string token, uint expiresInSecs) =>
            Begin("provide_token_for").Prop("token", token).Prop("expires_in_secs", expiresInSecs).EndObject().ToUtf8();

        public static byte[] TokenUnavailable(string reason) => Begin("token_unavailable").Prop("reason", reason).EndObject().ToUtf8();

        public static byte[] SetTokenEndpoint(string url, IReadOnlyList<Header> headers)
        {
            var w = Begin("set_token_endpoint").Prop("url", url).Name("headers");
            Headers(w, headers);
            return w.EndObject().ToUtf8();
        }

        public static byte[] StreamPause() => Begin("stream_pause").EndObject().ToUtf8();

        public static byte[] StreamResume() => Begin("stream_resume").EndObject().ToUtf8();

        public static byte[] SendContext(WorldContext context)
        {
            var w = Begin("send_context").Name("context").BeginObject()
                .Prop("scene", context.Scene)
                .Prop("source_lang", context.SourceLang)
                .Prop("target_lang", context.TargetLang)
                .Prop("level", context.Level)
                .Name("npc");
            Npc(w, context.Npc);
            w.Name("tags");
            Strings(w, context.Tags);
            return w.Prop("generate", context.Generate).EndObject().EndObject().ToUtf8();
        }

        public static byte[] PracticeStart(PracticeSource source)
        {
            var w = Begin("practice_start").Name("source");
            Source(w, source);
            return w.EndObject().ToUtf8();
        }

        public static byte[] PracticeSnapshot(SessionId session) => Session("practice_snapshot", session).EndObject().ToUtf8();

        public static byte[] PracticeAnswer(SessionId session, string answer) =>
            Session("practice_answer", session).Prop("answer", answer).EndObject().ToUtf8();

        public static byte[] PracticeChoose(SessionId session, uint option) =>
            Session("practice_choose", session).Prop("option", option).EndObject().ToUtf8();

        public static byte[] PracticeReveal(SessionId session) => Session("practice_reveal", session).EndObject().ToUtf8();

        public static byte[] PracticeSelfGrade(SessionId session, bool correct) =>
            Session("practice_self_grade", session).Prop("correct", correct).EndObject().ToUtf8();

        public static byte[] PracticeNext(SessionId session) => Session("practice_next", session).EndObject().ToUtf8();

        public static byte[] PracticeFinish(SessionId session) => Session("practice_finish", session).EndObject().ToUtf8();

        public static byte[] DialogueOpen(DialogueConfig config)
        {
            var w = Begin("dialogue_open").Name("config").BeginObject().Name("npc");
            Npc(w, config.Npc);
            w.Prop("scene", config.Scene)
                .Prop("source_lang", config.SourceLang)
                .Prop("target_lang", config.TargetLang)
                .Prop("level", config.Level)
                .Name("window").BeginArray();
            foreach (var entry in config.Window)
            {
                w.BeginObject().Prop("speaker", Words.Speakers.Name(entry.Speaker)).Prop("text", entry.Text).EndObject();
            }

            return w.EndArray().EndObject().EndObject().ToUtf8();
        }

        public static byte[] DialogueSay(DialogueId dialogue, string line) =>
            Dialogue("dialogue_say", dialogue).Prop("line", line).EndObject().ToUtf8();

        public static byte[] DialogueSetLevel(DialogueId dialogue, byte level) =>
            Dialogue("dialogue_set_level", dialogue).Prop("level", level).EndObject().ToUtf8();

        public static byte[] DialogueSetScene(DialogueId dialogue, string? scene) =>
            Dialogue("dialogue_set_scene", dialogue).Prop("scene", scene).EndObject().ToUtf8();

        public static byte[] DialogueCancel(DialogueId dialogue) => Dialogue("dialogue_cancel", dialogue).EndObject().ToUtf8();

        public static byte[] DialogueClose(DialogueId dialogue) => Dialogue("dialogue_close", dialogue).EndObject().ToUtf8();

        public static byte[] DialogueSnapshot(DialogueId dialogue) => Dialogue("dialogue_snapshot", dialogue).EndObject().ToUtf8();

        /// <summary>K1's <c>TokenSourceConfig::Http</c>, shared with the config writer.</summary>
        public static void Endpoint(JsonWriter w, TokenEndpoint endpoint)
        {
            w.BeginObject().Prop("type", "http").Prop("url", endpoint.Url).Name("headers");
            Headers(w, endpoint.Headers);
            w.EndObject();
        }

        public static void Strings(JsonWriter w, IReadOnlyList<string> items)
        {
            w.BeginArray();
            foreach (var item in items)
            {
                w.String(item);
            }

            w.EndArray();
        }

        private static JsonWriter Begin(string type) => new JsonWriter().BeginObject().Prop("type", type);

        private static JsonWriter Session(string type, SessionId session) => Begin(type).Prop("session", session.Value);

        private static JsonWriter Dialogue(string type, DialogueId dialogue) => Begin(type).Prop("dialogue", dialogue.Value);

        private static void Headers(JsonWriter w, IReadOnlyList<Header> headers)
        {
            w.BeginArray();
            foreach (var header in headers)
            {
                w.BeginObject().Prop("name", header.Name).Prop("value", header.Value).EndObject();
            }

            w.EndArray();
        }

        private static void Npc(JsonWriter w, Npc? npc)
        {
            if (npc == null)
            {
                w.Null();
                return;
            }

            w.BeginObject().Prop("name", npc.Name).Prop("persona", npc.Persona).EndObject();
        }

        private static void Source(JsonWriter w, PracticeSource source)
        {
            switch (source)
            {
                case PracticeSource.Request r:
                    w.BeginObject().Prop("type", "request").Prop("topic", r.Topic).Prop("source_lang", r.SourceLang)
                        .Prop("target_lang", r.TargetLang).Prop("level", r.Level).Name("tags");
                    Strings(w, r.Tags);
                    w.EndObject();
                    break;
                case PracticeSource.Create c:
                    w.BeginObject().Prop("type", "create").Prop("context", c.Context).Prop("source_lang", c.SourceLang)
                        .Prop("target_lang", c.TargetLang).Prop("level", c.Level).EndObject();
                    break;
                case PracticeSource.Plan p:
                    w.BeginObject().Prop("type", "plan").Prop("plan_id", p.PlanId).Prop("report", p.Report).EndObject();
                    break;
            }
        }
    }
}
