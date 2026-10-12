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
            Assert.Equal(9, examples.Count);
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
    }
}
