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
        public static byte[] ProvideToken(string token, string expiresAt, UpdateAvailable? update = null) =>
            Update(Begin("provide_token").Prop("token", token).Prop("expires_at", expiresAt), update).EndObject().ToUtf8();

        public static byte[] ProvideTokenFor(string token, uint expiresInSecs, UpdateAvailable? update = null) =>
            Update(Begin("provide_token_for").Prop("token", token).Prop("expires_in_secs", expiresInSecs), update).EndObject().ToUtf8();

        /// <summary>
        /// The notice (ADR 30.10.26p), written only when present: the key is new,
        /// and an older core's command has none. Inside it all three keys are
        /// written, nulls kept, in V1's order.
        /// </summary>
        private static JsonWriter Update(JsonWriter w, UpdateAvailable? update)
        {
            if (update == null)
            {
                return w;
            }

            return w.Name("update_available").BeginObject()
                .Prop("latest", update.Latest).Prop("deprecated", update.Deprecated).Prop("message", update.Message)
                .EndObject();
        }

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
                .Name("window");
            Window(w, config.Window);
            Profile(w, config);
            return w.EndObject().EndObject().ToUtf8();
        }

        /// <summary>A graph dialogue (ADR 28.10.26f): the IR as text, and the run's session.</summary>
        public static byte[] DialogueLoad(string irJson, GraphSession session) => DialogueLoad(DialogueSource.Ir(irJson), session);

        /// <summary>A graph dialogue from either source (ADR 28.10.26i): inline text, or a stored id.</summary>
        public static byte[] DialogueLoad(DialogueSource source, GraphSession session)
        {
            var w = Begin("dialogue_load").Name("source").BeginObject();
            w = source switch
            {
                DialogueSource.StoredSource s => w.Prop("type", "stored").Prop("dialogue_id", s.DialogueId),
                DialogueSource.IrSource i => w.Prop("type", "ir").Prop("ir_json", i.IrJson),
                _ => throw new System.ArgumentException("an unknown dialogue source", nameof(source)),
            };
            w.EndObject()
                .Name("session").BeginObject()
                .Prop("npc", session.Npc)
                .Prop("source_lang", session.SourceLang)
                .Prop("target_lang", session.TargetLang)
                .Prop("level", session.Level)
                .Prop("scene", session.Scene)
                .Name("facts").BeginObject();
            foreach (var fact in session.Facts)
            {
                Fact(w.Name(fact.Key), fact.Value);
            }

            w.EndObject().Prop("checkpoint", session.Checkpoint).Name("window");
            Window(w, session.Window);
            return w.EndObject().EndObject().ToUtf8();
        }

        public static byte[] DialogueSetFact(DialogueId dialogue, string key, FactValue value)
        {
            var w = Dialogue("dialogue_set_fact", dialogue).Prop("key", key).Name("value");
            Fact(w, value);
            return w.EndObject().ToUtf8();
        }

        public static byte[] DialogueValidate(string irJson) => Begin("dialogue_validate").Prop("ir_json", irJson).EndObject().ToUtf8();

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

        private static void Window(JsonWriter w, IReadOnlyList<WindowEntry> window)
        {
            w.BeginArray();
            foreach (var entry in window)
            {
                w.BeginObject().Prop("speaker", Words.Speakers.Name(entry.Speaker)).Prop("text", entry.Text).EndObject();
            }

            w.EndArray();
        }

        /// <summary>N1's profile, written only when set: K1 omits both when absent.</summary>
        private static void Profile(JsonWriter w, DialogueConfig config)
        {
            if (config.Languages.Count > 0)
            {
                w.Name("languages").BeginArray();
                foreach (var l in config.Languages)
                {
                    w.BeginObject().Prop("code", l.Code).Prop("speaking", l.Speaking).Prop("comprehension", l.Comprehension).EndObject();
                }

                w.EndArray();
            }

            if (config.Pedagogy is Pedagogy pedagogy)
            {
                w.Prop("pedagogy", GraphWords.Pedagogies.Name(pedagogy));
            }
        }

        private static void Fact(JsonWriter w, FactValue value)
        {
            switch (value)
            {
                case FactValue.Flag b:
                    w.Bool(b.Value);
                    break;
                case FactValue.Number n:
                    w.Literal(n.Literal);
                    break;
                case FactValue.Text t:
                    w.String(t.Value);
                    break;
            }
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
