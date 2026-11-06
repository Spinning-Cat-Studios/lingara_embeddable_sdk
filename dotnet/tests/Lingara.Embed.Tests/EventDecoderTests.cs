using System.IO;
using System.Linq;
using System.Text;
using System.Text.Json;
using Lingara.Embed.Events;
using Xunit;

namespace Lingara.Embed.Tests
{
    /// <summary>The decoder against K1's shape vector and the committed schema (ADR 1.10.26aa D12).</summary>
    public class EventDecoderTests
    {
        private static EmbedEvent Decode(JsonElement example) => EventDecoder.Decode(Encoding.UTF8.GetBytes(example.GetRawText()), keepRaw: true);

        private static void Carried(EmbedEvent evt)
        {
            switch (evt)
            {
                case EmbedEvent.TokenNeeded e:
                    Assert.Equal((1L, TokenReason.Initial), (e.Request.Value, e.Reason));
                    break;
                case EmbedEvent.StreamChanged e:
                    Assert.IsType<StreamState.Live>(e.State);
                    Assert.Equal("lgr_evt_1", e.Cursor);
                    break;
                case EmbedEvent.LingaraReceived e:
                    Assert.Equal(("lgr_evt_1", "game.custom", "2026-10-01T12:00:00Z", "{\"n\":1}"), (e.Event.Id, e.Event.EventType, e.Event.CreatedAt, e.Event.DataJson));
                    Assert.IsType<LingaraEventDetail.Other>(e.Event.Detail);
                    break;
                case EmbedEvent.WorldSent e:
                    Assert.Equal((2L, "lgr_evt_2"), (e.Request.Value, e.EventId));
                    Assert.Equal(("started", "8f0e2c1a-0000-4000-8000-000000000001", "generating"), (e.Reaction!.Status, e.Reaction.PlanId, e.Reaction.PlanStatus));
                    Assert.Null(e.Reaction.Code);
                    break;
                case EmbedEvent.Practice e:
                    CarriedPractice(e);
                    break;
                default:
                    CarriedTheRest(evt);
                    break;
            }
        }

        private static void CarriedTheRest(EmbedEvent evt)
        {
            switch (evt)
            {
                case EmbedEvent.NpcTurn e:
                    Assert.Equal((4L, 5L), (e.Dialogue.Value, e.Request.Value));
                    Assert.Equal("Ni", Assert.IsType<NpcTurnUpdate.Fragment>(e.Update).Text);
                    break;
                case EmbedEvent.Failed e:
                    Assert.Equal((ErrorKind.RateLimited, "rate_limited", "slow down", 429, "rate_limited", 30000L, true, 5L),
                        (e.Error.Kind, e.Error.KindName, e.Error.Message, e.Error.Status, e.Error.Code, e.Error.RetryAfterMs, e.Error.Retryable, e.Error.Request!.Value.Value));
                    break;
                case EmbedEvent.Log e:
                    Assert.Equal((LogLevel.Warn, "lingara_embed::token", "a token lifetime was clamped"), (e.Level, e.Target, e.Message));
                    break;
                case EmbedEvent.Closed e:
                    Assert.Equal(CloseReason.Requested, e.Reason);
                    break;
                // 30.10.26p AC6
                case EmbedEvent.UpdateAvailable e:
                    Assert.Equal(("1.2.0", true, "1.1.x stops minting on 1 December; please update."), (e.Latest, e.Deprecated, e.Message));
                    break;
                case EmbedEvent.CheckpointReached e:
                    Assert.Equal((4L, "ask_about_brother", "explained", CrossedVia.Judge), (e.Dialogue.Value, e.Checkpoint, e.Edge, e.Via));
                    break;
                case EmbedEvent.DialogueEnded e:
                    Assert.Equal(("success", "ask_about_brother", "gate"), (e.Outcome, e.Checkpoint, e.Edge));
                    break;
                default:
                    CarriedStored(evt);
                    break;
            }
        }

        // 28.10.26i AC12
        private static void CarriedStored(EmbedEvent evt)
        {
            switch (evt)
            {
                case EmbedEvent.DialogueLoaded e:
                    Assert.Equal((4L, "3f2a9c1e-7b4d-4e8a-9c0f-1a2b3c4d5e6f", 4L), (e.Dialogue.Value, e.StoreId, e.Rev));
                    break;
                case EmbedEvent.DialogueLoadFailed e:
                    Assert.Equal((4L, ErrorKind.NotFound, "dialogue_not_active", false), (e.Dialogue.Value, e.Error.Kind, e.Error.Code, e.Error.Retryable));
                    break;
                // 3.11.26j D9
                case EmbedEvent.DialoguePlanned e:
                    Assert.Equal((4L, PlanOutcome.Kept, 0, (PlanKeptReason?)PlanKeptReason.Stale), (e.Dialogue.Value, e.Outcome, e.Checkpoints.Count, e.Reason));
                    break;
                default:
                    Assert.Fail($"{evt.Type} decoded to {evt.GetType().Name}");
                    break;
            }
        }

        private static void CarriedPractice(EmbedEvent.Practice e)
        {
            var s = e.Snapshot;
            Assert.Equal(3L, e.Session.Value);
            Assert.Equal((PracticePhase.Unanswered, 0u, 2u, 0u, 3u), (s.Phase, s.SetIndex, s.SetCount, s.QuestionIndex, s.QuestionCount));
            Assert.Equal(("At the market", (byte)2, "en", "zh"), (s.Plan!.Title, s.Plan.Level, s.Plan.SourceLang, s.Plan.TargetLang));
            Assert.Equal(("你好", "hello", "nǐ hǎo"), (s.Plan.Vocabulary[0].Word, s.Plan.Vocabulary[0].Translation, s.Plan.Vocabulary[0].Pronunciation));
            Assert.Equal(("multiple_choice", "Hello is…", false), (s.Question!.Kind, s.Question.Prompt, s.Question.SelfGraded));
            Assert.Equal(new[] { "再见", "你好" }, s.Question.Options);
            Assert.Equal((PracticeOutcome.Correct, "你好", "a greeting"), (s.LastResult!.Outcome, s.LastResult.Expected, s.LastResult.Explanation));
            Assert.Null(s.Failure);
        }

        // 1.10.26aa AC15
        [Fact]
        public void EveryShapeVectorEventDecodesAndUnknownIsKept()
        {
            using var vector = JsonDocument.Parse(File.ReadAllText(Support.Vector("json-shape.json")));
            var examples = vector.RootElement.GetProperty("tagged").GetProperty("EmbedEvent").EnumerateArray().ToList();
            Assert.Equal(15, examples.Count);
            foreach (var example in examples)
            {
                var evt = Decode(example);
                Assert.Equal(example.GetProperty("type").GetString(), evt.Type);
                Assert.Equal(example.GetRawText(), evt.RawJson);
                Carried(evt);
            }

            using var schema = JsonDocument.Parse(File.ReadAllText(Support.Vector("messages.schema.json")));
            var types = schema.RootElement.GetProperty("$defs").GetProperty("EmbedEvent").GetProperty("oneOf").EnumerateArray()
                .Select(v => v.GetProperty("properties").GetProperty("type").GetProperty("const").GetString()!);
            Assert.All(types, t => Assert.Contains(t, EventDecoder.Types));

            const string future = "{\"type\":\"weather\",\"sky\":\"clear\"}";
            var unknown = Assert.IsType<EmbedEvent.Unknown>(EventDecoder.Decode(Encoding.UTF8.GetBytes(future), keepRaw: false));
            Assert.Equal(("weather", future), (unknown.Type, unknown.Json));
        }

        // 3.11.26g AC27
        [Fact]
        public void Dialogue_ended_carries_any_ending()
        {
            using var vector = JsonDocument.Parse(File.ReadAllText(Support.Vector("json-shape.json")));
            var named = vector.RootElement.GetProperty("structs").GetProperty("DialogueEnded")[0];
            var fled = Assert.IsType<EmbedEvent.DialogueEnded>(Decode(named));
            Assert.Equal(("fled", "introduction", "threat"), (fled.Outcome, fled.Checkpoint, fled.Edge));

            const string closed = "{\"type\":\"dialogue_ended\",\"dialogue\":4,\"outcome\":\"abandoned\",\"checkpoint\":\"intro\",\"edge\":null}";
            var abandoned = Assert.IsType<EmbedEvent.DialogueEnded>(EventDecoder.Decode(Encoding.UTF8.GetBytes(closed), keepRaw: false));
            Assert.Equal(DialogueOutcome.Abandoned, abandoned.Outcome);
            Assert.Null(abandoned.Edge);
            Assert.Null(abandoned.Trace);
        }

        // 3.11.26j D9: a snapshot's procedural half decodes, and fed back as a session's
        // `frontier` and `progress` it encodes to the shape vector's GraphSession, so a
        // C# game restores a run with what it saved.
        [Fact]
        public void Procedural_snapshot_restores_through_the_session()
        {
            using var vector = JsonDocument.Parse(File.ReadAllText(Support.Vector("json-shape.json")));
            var structs = vector.RootElement.GetProperty("structs");
            var raw = Encoding.UTF8.GetBytes(structs.GetProperty("DialogueSnapshot")[1].GetRawText());
            var snapshot = Lingara.Embed.Json.TypeDecoders.OptDialogue(Lingara.Embed.Json.JsonReader.Parse(raw))!;
            var saved = snapshot.Procedural!;
            Assert.Equal((3, 2, 1), (saved.Progress.NextPrefix, saved.Progress.Trace.Count, saved.Progress.Beats));

            var session = new GraphSession
            {
                Npc = "keeper", SourceLang = "en", TargetLang = "zh", Level = 2,
                Checkpoint = snapshot.Graph!.Checkpoint, Frontier = saved.Frontier, Progress = saved.Progress,
            };
            var sent = Lingara.Embed.Json.JsonReader.Parse(Lingara.Embed.Commands.CommandEncoder.DialogueLoad("{}", session))["session"];
            var want = structs.GetProperty("GraphSession")[0];
            Assert.Equal(want.GetProperty("frontier").GetString(), sent.Str("frontier"));
            var progress = sent.Obj("progress");
            var wanted = want.GetProperty("progress");
            Assert.Equal(wanted.GetProperty("next_prefix").GetInt64(), progress.Long("next_prefix"));
            Assert.Equal(wanted.GetProperty("plans").GetInt64(), progress.Long("plans"));
            var trace = progress["trace"].Items;
            Assert.Equal(2, trace.Count);
            Assert.Equal(("p1_0", "open", "start", 1L), (trace[0].Str("id"), trace[0].Str("atom"), trace[0].Str("via"), trace[0].Long("left_turn")));
            Assert.True(trace[1]["left_turn"].IsNull, "the current beat is still open");
            Assert.Equal("open", progress["visited"].Items[0].Str("atom"));
        }

        // 3.11.26j D9, D13: the new event, and a traced end.
        [Fact]
        public void Procedural_events_decode_with_their_trace()
        {
            using var vector = JsonDocument.Parse(File.ReadAllText(Support.Vector("json-shape.json")));
            var structs = vector.RootElement.GetProperty("structs");
            var planned = Assert.IsType<EmbedEvent.DialoguePlanned>(Decode(structs.GetProperty("DialoguePlanned")[0]));
            Assert.Equal((PlanOutcome.Accepted, (PlanKeptReason?)null), (planned.Outcome, planned.Reason));
            Assert.Equal(new[] { "p1_1", "p2_1", "p2_2" }, planned.Checkpoints);

            var traced = Assert.IsType<EmbedEvent.DialogueEnded>(Decode(structs.GetProperty("DialogueEnded")[1]));
            Assert.Equal(2, traced.Trace!.Count);
            var (first, last) = (traced.Trace[0], traced.Trace[1]);
            Assert.Equal(("p1_0", (AtomKind?)AtomKind.Open, CrossedVia.Start, 0, (int?)1, "p1_0_x0"), (first.Id, first.Atom, first.Via, first.EnteredTurn, first.LeftTurn, first.LeftByEdge));
            Assert.Equal(("sold", "p1_1_end", 1), (last.Ending, last.LeftByEdge, last.PlanSeq));
        }
    }
}
