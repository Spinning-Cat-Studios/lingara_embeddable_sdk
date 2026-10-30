using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using Lingara.Embed.Commands;
using Xunit;

namespace Lingara.Embed.Tests
{
    /// <summary>
    /// The encoder against K1's shape vector (ADR 1.10.26aa D12): every command
    /// in the schema has a typed client method, whose bytes equal the vector's
    /// example. A command K1 adds fails here until C# has it.
    /// </summary>
    public class CommandEncoderTests
    {
        private static readonly SessionId S = new SessionId(3);
        private static readonly DialogueId D = new DialogueId(4);

        /// <summary>Each command type: the client method that sends it, and that method's bytes for the vector's example.</summary>
        private static readonly Dictionary<string, (string Method, Func<byte[]> Encode)> Table = new Dictionary<string, (string, Func<byte[]>)>
        {
            ["provide_token"] = ("ProvideToken", () => CommandEncoder.ProvideToken("lgr_et_example", "2026-10-01T12:15:00Z")),
            ["provide_token_for"] = ("ProvideTokenFor", () => CommandEncoder.ProvideTokenFor("lgr_et_example", 900)),
            ["token_unavailable"] = ("TokenUnavailable", () => CommandEncoder.TokenUnavailable("offline")),
            ["set_token_endpoint"] = ("SetTokenEndpoint", () => CommandEncoder.SetTokenEndpoint("https://game.example/lingara/token", new[] { new Header("cookie", "session=abc") })),
            ["stream_pause"] = ("StreamPause", CommandEncoder.StreamPause),
            ["stream_resume"] = ("StreamResume", CommandEncoder.StreamResume),
            ["send_context"] = ("SendContext", () => CommandEncoder.SendContext(new WorldContext
            {
                Scene = "the night market",
                SourceLang = "en",
                TargetLang = "zh",
                Level = 2,
                Npc = new Npc { Name = "Mei", Persona = "a tea seller" },
                Tags = new[] { "market" },
                Generate = false,
            })),
            ["practice_start"] = ("PracticeStart", () => CommandEncoder.PracticeStart(new PracticeSource.Plan { PlanId = "8f0e2c1a-0000-4000-8000-000000000001", Report = false })),
            ["practice_snapshot"] = ("GetPracticeSnapshot", () => CommandEncoder.PracticeSnapshot(S)),
            ["practice_answer"] = ("PracticeAnswer", () => CommandEncoder.PracticeAnswer(S, "ni hao")),
            ["practice_choose"] = ("PracticeChoose", () => CommandEncoder.PracticeChoose(S, 1)),
            ["practice_reveal"] = ("PracticeReveal", () => CommandEncoder.PracticeReveal(S)),
            ["practice_self_grade"] = ("PracticeSelfGrade", () => CommandEncoder.PracticeSelfGrade(S, true)),
            ["practice_next"] = ("PracticeNext", () => CommandEncoder.PracticeNext(S)),
            ["practice_finish"] = ("PracticeFinish", () => CommandEncoder.PracticeFinish(S)),
            ["dialogue_open"] = ("DialogueOpen", () => CommandEncoder.DialogueOpen(new DialogueConfig
            {
                Npc = new Npc { Name = "Mei" },
                Scene = "a tea house",
                SourceLang = "en",
                TargetLang = "zh",
                Level = 2,
                Window = new[] { new WindowEntry { Speaker = Speaker.Player, Text = "hi" } },
            })),
            ["dialogue_say"] = ("DialogueSay", () => CommandEncoder.DialogueSay(D, "how are you?")),
            ["dialogue_set_level"] = ("DialogueSetLevel", () => CommandEncoder.DialogueSetLevel(D, 3)),
            ["dialogue_set_scene"] = ("DialogueSetScene", () => CommandEncoder.DialogueSetScene(D, "the market")),
            ["dialogue_cancel"] = ("DialogueCancel", () => CommandEncoder.DialogueCancel(D)),
            ["dialogue_close"] = ("DialogueClose", () => CommandEncoder.DialogueClose(D)),
            ["dialogue_snapshot"] = ("GetDialogueSnapshot", () => CommandEncoder.DialogueSnapshot(D)),
            ["dialogue_load"] = ("DialogueLoad", () => CommandEncoder.DialogueLoad(Ir, new GraphSession
            {
                Npc = "elena",
                SourceLang = "en",
                TargetLang = "es",
                Level = 2,
                Facts = new Dictionary<string, FactValue> { ["trust"] = FactValue.Of(0.4) },
            })),
            ["dialogue_set_fact"] = ("DialogueSetFact", () => CommandEncoder.DialogueSetFact(D, "trust", FactValue.Of(0.4))),
            ["dialogue_validate"] = ("DialogueValidate", () => CommandEncoder.DialogueValidate(Ir)),
        };

        /// <summary>The shape vector's one IR, as text.</summary>
        private const string Ir = "{\"schema\":\"dialogue.v1\",\"id\":\"gate\",\"npcs\":[{\"id\":\"elena\",\"name\":\"Elena\"}],\"start\":\"intro\",\"checkpoints\":[{\"id\":\"intro\",\"goal\":\"Elena is wary\",\"edges\":[{\"id\":\"gate\",\"to\":\"success\",\"when\":\"the player asks\"}]}]}";

        /// <summary>JSON equality: objects by key in any order, everything else exactly.</summary>
        internal static bool JsonEqual(JsonElement a, JsonElement b)
        {
            if (a.ValueKind != b.ValueKind)
            {
                return false;
            }

            return a.ValueKind switch
            {
                JsonValueKind.Object => a.EnumerateObject().Count() == b.EnumerateObject().Count()
                    && a.EnumerateObject().All(p => b.TryGetProperty(p.Name, out var other) && JsonEqual(p.Value, other)),
                JsonValueKind.Array => a.GetArrayLength() == b.GetArrayLength() && a.EnumerateArray().Zip(b.EnumerateArray()).All(x => JsonEqual(x.First, x.Second)),
                _ => a.GetRawText() == b.GetRawText(),
            };
        }

        // 1.10.26aa AC16
        [Fact]
        public void EveryCommandEncodesToItsShapeVector()
        {
            using var schema = JsonDocument.Parse(File.ReadAllText(Support.Vector("messages.schema.json")));
            var commands = schema.RootElement.GetProperty("$defs").GetProperty("Command").GetProperty("oneOf").EnumerateArray()
                .Select(v => v.GetProperty("properties").GetProperty("type").GetProperty("const").GetString()!).ToList();
            Assert.Equal(25, commands.Count);
            Assert.All(commands, c => Assert.True(Table.ContainsKey(c), $"no C# method for the command `{c}`"));

            using var vector = JsonDocument.Parse(File.ReadAllText(Support.Vector("json-shape.json")));
            foreach (var example in vector.RootElement.GetProperty("tagged").GetProperty("Command").EnumerateArray())
            {
                var type = example.GetProperty("type").GetString()!;
                var (method, encode) = Table[type];
                // By name: a method may have overloads (DialogueLoad takes text or a built Dialogue).
                Assert.Contains(typeof(LingaraEmbedClient).GetMethods(), m => m.Name == method);
                using var written = JsonDocument.Parse(encode());
                Assert.True(JsonEqual(example, written.RootElement), $"{type}: wrote {written.RootElement.GetRawText()}, the vector has {example.GetRawText()}");
            }

            // 28.10.26i AC7: the stored source, against its DialogueSource example.
            var stored = vector.RootElement.GetProperty("tagged").GetProperty("DialogueSource").EnumerateArray().Single(s => s.GetProperty("type").GetString() == "stored");
            using var load = JsonDocument.Parse(CommandEncoder.DialogueLoad(DialogueSource.Stored("3f2a9c1e-7b4d-4e8a-9c0f-1a2b3c4d5e6f"), new GraphSession { Npc = "elena", SourceLang = "en", TargetLang = "es", Level = 2 }));
            Assert.True(JsonEqual(stored, load.RootElement.GetProperty("source")), load.RootElement.GetRawText());
        }

        // 28.10.26f AC15
        [Fact]
        public void GraphCommandsAndEventsMatchTheirShapeVectors()
        {
            using var vector = JsonDocument.Parse(File.ReadAllText(Support.Vector("json-shape.json")));
            var root = vector.RootElement;
            foreach (var type in new[] { "dialogue_load", "dialogue_set_fact", "dialogue_validate" })
            {
                var example = root.GetProperty("tagged").GetProperty("Command").EnumerateArray().Single(c => c.GetProperty("type").GetString() == type);
                using var written = JsonDocument.Parse(Table[type].Encode());
                Assert.True(JsonEqual(example, written.RootElement), $"{type}: wrote {written.RootElement.GetRawText()}");
            }

            // N1's profile on the free-form config, written only when set.
            var profiled = new DialogueConfig
            {
                Npc = new Npc { Name = "Elena", Persona = "a farmer" },
                SourceLang = "en",
                TargetLang = "es",
                Level = 2,
                Languages = new[] { new NpcLanguage { Code = "es", Speaking = 9, Comprehension = 9 }, new NpcLanguage { Code = "en", Speaking = 2, Comprehension = 3 } },
                Pedagogy = Pedagogy.None,
            };
            using var open = JsonDocument.Parse(CommandEncoder.DialogueOpen(profiled));
            var config = root.GetProperty("structs").GetProperty("DialogueConfig")[0];
            Assert.True(JsonEqual(config, open.RootElement.GetProperty("config")), open.RootElement.GetRawText());

            var events = root.GetProperty("tagged").GetProperty("EmbedEvent").EnumerateArray()
                .Where(e => e.GetProperty("type").GetString() is "checkpoint_reached" or "dialogue_ended")
                .Select(e => Lingara.Embed.Events.EventDecoder.Decode(System.Text.Encoding.UTF8.GetBytes(e.GetRawText()), keepRaw: false)).ToList();
            var reached = Assert.IsType<Lingara.Embed.Events.EmbedEvent.CheckpointReached>(events[0]);
            Assert.Equal((4L, "ask_about_brother", "explained", CrossedVia.Judge), (reached.Dialogue.Value, reached.Checkpoint, reached.Edge, reached.Via));
            var ended = Assert.IsType<Lingara.Embed.Events.EmbedEvent.DialogueEnded>(events[1]);
            Assert.Equal((DialogueOutcome.Success, "ask_about_brother", "gate"), (ended.Outcome, ended.Checkpoint, ended.Edge));

            var snapshot = Lingara.Embed.Json.TypeDecoders.OptDialogue(Lingara.Embed.Json.JsonReader.Parse(System.Text.Encoding.UTF8.GetBytes(root.GetProperty("structs").GetProperty("DialogueSnapshot")[0].GetRawText())))!;
            Assert.Equal((DialoguePhase.Ended, "elena", "ask", DialogueOutcome.Success), (snapshot.Phase, snapshot.Graph!.Npc, snapshot.Graph.Checkpoint, snapshot.Graph.Outcome));
            Assert.Equal(0.4, Assert.IsType<FactValue.Number>(snapshot.Graph.Facts["trust"]).Value);
        }
    }
}
