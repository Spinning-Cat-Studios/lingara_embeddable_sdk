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
        };

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
            Assert.Equal(22, commands.Count);
            Assert.All(commands, c => Assert.True(Table.ContainsKey(c), $"no C# method for the command `{c}`"));

            using var vector = JsonDocument.Parse(File.ReadAllText(Support.Vector("json-shape.json")));
            foreach (var example in vector.RootElement.GetProperty("tagged").GetProperty("Command").EnumerateArray())
            {
                var type = example.GetProperty("type").GetString()!;
                var (method, encode) = Table[type];
                Assert.NotNull(typeof(LingaraEmbedClient).GetMethod(method));
                using var written = JsonDocument.Parse(encode());
                Assert.True(JsonEqual(example, written.RootElement), $"{type}: wrote {written.RootElement.GetRawText()}, the vector has {example.GetRawText()}");
            }
        }
    }
}
