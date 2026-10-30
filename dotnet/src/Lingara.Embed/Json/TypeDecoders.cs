using System;
using System.Collections.Generic;
using System.Globalization;

namespace Lingara.Embed.Json
{
    /// <summary>
    /// One decode function per type in <c>Types/</c> (ADR 1.10.26aa D8): K1's JSON
    /// shape read by key, unknown keys ignored, an unknown tag kept as an
    /// <c>Unknown</c> case rather than refused (D5).
    /// </summary>
    internal static class TypeDecoders
    {
        public static EmbedError Error(JsonValue v)
        {
            var kind = v.Str("kind");
            var request = v.OptLong("request");
            return new EmbedError
            {
                Kind = Words.ErrorKinds.Parse(kind),
                KindName = kind,
                Message = v.Str("message"),
                Status = (int?)v.OptLong("status"),
                Code = v.OptStr("code"),
                RetryAfterMs = v.OptLong("retry_after_ms"),
                Retryable = v.Bool("retryable"),
                Request = request.HasValue ? new RequestId(request.Value) : (RequestId?)null,
            };
        }

        public static EmbedError? OptError(JsonValue v) => v.IsNull ? null : Error(v);

        public static StreamState Stream(JsonValue v)
        {
            var type = v.Str("type");
            return type switch
            {
                "connecting" => new StreamState.Connecting(),
                "live" => new StreamState.Live(),
                "paused" => new StreamState.Paused(),
                "down" => new StreamState.Down { Error = Error(v.Obj("error")), RetryInMs = v.Long("retry_in_ms") },
                _ => new StreamState.Unknown { Type = type },
            };
        }

        public static EmbedStatus Status(JsonValue v) => new EmbedStatus
        {
            Stream = Stream(v.Obj("stream")),
            Cursor = v.OptStr("cursor"),
            NeedsToken = v.Bool("needs_token"),
            Closed = v.Bool("closed"),
        };

        public static Npc Npc(JsonValue v) => new Npc { Name = v.Str("name"), Persona = v.OptStr("persona") };

        public static Reaction? Reaction(JsonValue v) => v.IsNull ? null : new Reaction
        {
            Status = v.Str("status"),
            PlanId = v.OptStr("plan_id"),
            PlanStatus = v.OptStr("plan_status"),
            Code = v.OptStr("code"),
            Error = v.OptStr("error"),
        };

        public static LingaraEvent LingaraEvent(JsonValue v) => new LingaraEvent
        {
            Id = v.Str("id"),
            EventType = v.Str("event_type"),
            CreatedAt = v.Str("created_at"),
            DataJson = v.Str("data_json"),
            Detail = Detail(v.Obj("detail")),
        };

        public static LingaraEventDetail Detail(JsonValue v) => v.Str("type") switch
        {
            "lesson_plan_ready" => new LingaraEventDetail.LessonPlanReady
            {
                PlanId = v.Str("plan_id"),
                Title = v.OptStr("title"),
                Level = Byte(v, "level"),
                SourceLang = v.Str("source_lang"),
                TargetLang = v.Str("target_lang"),
                Complete = v.Bool("complete"),
            },
            "lesson_plan_failed" => new LingaraEventDetail.LessonPlanFailed { PlanId = v.Str("plan_id"), Reason = v.Str("reason") },
            _ => new LingaraEventDetail.Other(),
        };

        public static NpcTurnUpdate TurnUpdate(JsonValue v)
        {
            var type = v.Str("type");
            return type switch
            {
                "opened" => new NpcTurnUpdate.Opened(),
                "fragment" => new NpcTurnUpdate.Fragment { Text = v.Str("text") },
                "notice" => new NpcTurnUpdate.Notice { Code = v.Str("code"), Message = v.Str("message") },
                "finished" => new NpcTurnUpdate.Finished { Text = v.Str("text") },
                "cancelled" => new NpcTurnUpdate.Cancelled(),
                _ => new NpcTurnUpdate.Unknown { Type = type },
            };
        }

        public static PracticeSnapshot? OptPractice(JsonValue v) => v.IsNull ? null : Practice(v);

        public static PracticeSnapshot Practice(JsonValue v) => new PracticeSnapshot
        {
            Phase = Words.PracticePhases.Parse(v.Str("phase")),
            Plan = v["plan"].IsNull ? null : Plan(v["plan"]),
            SetIndex = UInt(v, "set_index"),
            SetCount = UInt(v, "set_count"),
            QuestionIndex = UInt(v, "question_index"),
            QuestionCount = UInt(v, "question_count"),
            Question = v["question"].IsNull ? null : Question(v["question"]),
            LastResult = v["last_result"].IsNull ? null : Result(v["last_result"]),
            Failure = OptError(v["failure"]),
        };

        public static DialogueSnapshot? OptDialogue(JsonValue v) => v.IsNull ? null : new DialogueSnapshot
        {
            Phase = Words.DialoguePhases.Parse(v.Str("phase")),
            Npc = Npc(v.Obj("npc")),
            Scene = v.OptStr("scene"),
            Level = Byte(v, "level"),
            Window = List(v["window"], e => new WindowEntry { Speaker = Words.Speakers.Parse(e.Str("speaker")), Text = e.Str("text") }),
            Graph = v["graph"].IsNull ? null : Graph(v["graph"]),
        };

        /// <summary>A graph dialogue's story (ADR 28.10.26f D6).</summary>
        public static GraphSnapshot Graph(JsonValue v) => new GraphSnapshot
        {
            Id = v.Str("id"),
            Npc = v.Str("npc"),
            Checkpoint = v.Str("checkpoint"),
            Facts = Facts(v.Obj("facts")),
            Outcome = v.OptStr("outcome") is string outcome ? GraphWords.Outcomes.Parse(outcome) : (DialogueOutcome?)null,
        };

        private static Dictionary<string, FactValue> Facts(JsonValue v)
        {
            var facts = new Dictionary<string, FactValue>();
            foreach (var member in v.Members)
            {
                facts[member.Key] = member.Value.Kind switch
                {
                    JsonKind.True => FactValue.Of(true),
                    JsonKind.False => FactValue.Of(false),
                    JsonKind.String => FactValue.Of(member.Value.Text!),
                    JsonKind.Number => FactValue.Of(double.Parse(member.Value.Text!, NumberStyles.Float, CultureInfo.InvariantCulture)),
                    _ => throw new JsonFormatException($"fact `{member.Key}` is not a boolean, a number or a string"),
                };
            }

            return facts;
        }

        public static IReadOnlyList<T> List<T>(JsonValue v, Func<JsonValue, T> item)
        {
            var list = new List<T>(v.Items.Count);
            foreach (var element in v.Items)
            {
                list.Add(item(element));
            }

            return list;
        }

        public static byte Byte(JsonValue v, string key) => checked((byte)v.Long(key));

        public static uint UInt(JsonValue v, string key) => checked((uint)v.Long(key));

        private static PlanSummary Plan(JsonValue v) => new PlanSummary
        {
            Id = v.Str("id"),
            Title = v.OptStr("title"),
            Level = Byte(v, "level"),
            SourceLang = v.Str("source_lang"),
            TargetLang = v.Str("target_lang"),
            Vocabulary = List(v["vocabulary"], w => new PlanWord { Word = w.Str("word"), Translation = w.Str("translation"), Pronunciation = w.OptStr("pronunciation") }),
        };

        private static PracticeQuestion Question(JsonValue v) => new PracticeQuestion
        {
            Kind = v.Str("kind"),
            Prompt = v.Str("prompt"),
            Options = v["options"].IsNull ? null : List(v["options"], o => o.AsString() ?? string.Empty),
            Hint = v.OptStr("hint"),
            SelfGraded = v.Bool("self_graded"),
        };

        private static AnswerResult Result(JsonValue v) => new AnswerResult
        {
            Outcome = v["outcome"].IsNull ? (PracticeOutcome?)null : Words.PracticeOutcomes.Parse(v.Str("outcome")),
            Expected = v.Str("expected"),
            Explanation = v.Str("explanation"),
        };
    }
}
