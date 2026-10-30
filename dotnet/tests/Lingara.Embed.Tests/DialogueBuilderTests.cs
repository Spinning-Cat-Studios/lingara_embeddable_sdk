using System;
using System.IO;
using System.Linq;
using System.Text;
using Lingara.Embed.Dialogues;
using Lingara.Embed.Json;
using Xunit;

namespace Lingara.Embed.Tests
{
    /// <summary>The C# builder (ADR 28.10.26g D1, D3, D5, D6) against the shared dialogue vectors.</summary>
    public class DialogueBuilderTests
    {
        internal static string DialogueVector(string name) => File.ReadAllText(Support.Vector(Path.Combine("dialogue", name)), new UTF8Encoding(false));

        internal static JsonValue ParsedVector(string name) => JsonReader.Parse(File.ReadAllBytes(Support.Vector(Path.Combine("dialogue", name))));

        /// <summary>N2's two-keepers dialogue, written with the builder.</summary>
        private static Dialogue TwoKeepers() => new DialogueBuilder("night_market_keepers", start: "suspicious_of_player")
            .Title("The two keepers").Pedagogy(Pedagogy.None)
            .Npc("old_wei", "Old Wei", n => n.Persona("a retired ferryman who distrusts strangers").Speaks("zh", Level.Native, Level.Native))
            .Fact("trust", FactType.Number, defaultValue: 0)
            .Fact("eastern_gate", FactType.String)
            .Checkpoint("suspicious_of_player", "Size the stranger up; warm to them only if they explain why they are here", c => c
                .Edge("asks_about_brother", "reveal_gate", predicate: "trust >= 0.4", when: "the player has asked about their missing brother")
                .Edge("turns_hostile", Outcome.Failure, when: "the player threatens or insults Old Wei"))
            .Checkpoint("reveal_gate", "Tell the player the eastern gate is guarded, reluctantly", c => c
                .Discloses("eastern_gate")
                .Edge("thanks", Outcome.Success, when: "the player thanks Old Wei or says goodbye"))
            .Build();

        /// <summary>D1's worked dialogue, extended as D4 describes.</summary>
        internal static Dialogue MissingBrother() => new DialogueBuilder("missing_brother", start: "introduction")
            .Title("The missing brother").Pedagogy(Pedagogy.None)
            .Npc("elena", "Elena", n => n.Persona("a farmer who distrusts soldiers")
                .Speaks("es", Level.Native, Level.Native)
                .Speaks("en", Level.Cefr("A2"), Level.Cefr("B1")))
            .Npc("tomas", "Tomás", n => n.Persona("Elena's neighbour, a retired soldier")
                .Speaks("es", Level.Native, Level.Native)
                .Speaks("en", 6, 7))
            .Fact("trust", FactType.Number, defaultValue: 0)
            .Fact("suspicion", FactType.Number, defaultValue: 0.4)
            .Fact("alarm_raised", FactType.Boolean, defaultValue: false)
            .Fact("eastern_gate", FactType.String)
            .Checkpoint("introduction", "Elena is wary and asks who the player is", c => c
                .Edge("alarm", Outcome.Failure, predicate: "alarm_raised")
                .Edge("explained", "ask_about_brother", when: "the player says who they are and why they are here"))
            .Checkpoint("ask_about_brother", "Admit she saw soldiers ride toward \"the old mill\"; reveal the gate only if she trusts the player", c => c
                .Discloses("eastern_gate")
                .Edge("gate", Outcome.Success, predicate: "trust >= 0.4", when: "the player asks where soldiers might take a prisoner")
                .Edge("neighbour", "ask_tomas", when: "the player asks who else saw the soldiers")
                .Edge("leaves", Outcome.Abandoned, when: "the player walks away"))
            .Checkpoint("ask_tomas", "Tomás says the soldiers spoke of the road to 長安", c => c
                .Edge("suspicious", Outcome.Failure, predicate: "suspicion > 0.5")
                .Edge("back_to_elena", "ask_about_brother", when: "the player returns to Elena with what Tomás said"))
            .Build();

        // 28.10.26g AC5
        [Fact]
        public void TheWorkedDialoguesMatchTheGoldenBytes()
        {
            Assert.Equal(DialogueVector("two-keepers.comparison.json"), TwoKeepers().ToCanonicalJson());
            Assert.Equal(DialogueVector("missing-brother.comparison.json"), MissingBrother().ToCanonicalJson());
        }

        /// <summary>A one-checkpoint dialogue with <paramref name="change"/> applied before its checkpoints.</summary>
        private static DialogueBuilder Small(Action<DialogueBuilder>? change = null)
        {
            var b = new DialogueBuilder("d", start: "a").Npc("n", "N").Fact("x", FactType.Number);
            change?.Invoke(b);
            return b;
        }

        private static string Refused(DialogueBuilder b) => Assert.Throws<ArgumentException>(() => b.Build()).Message.Split(':')[0];

        // 28.10.26g AC7
        [Fact]
        public void LevelsAndStructuralRefusals()
        {
            var labels = ParsedVector("level-labels.json");
            foreach (var label in labels["cefr"].Members)
            {
                Assert.Equal((byte)long.Parse(label.Value.Text!, System.Globalization.CultureInfo.InvariantCulture), Level.Cefr(label.Key));
            }

            foreach (var label in labels["jlpt"].Members)
            {
                Assert.Equal((byte)long.Parse(label.Value.Text!, System.Globalization.CultureInfo.InvariantCulture), Level.Jlpt(label.Key));
            }

            Assert.Equal(Level.Native, labels.Long("native"));
            Assert.Throws<ArgumentException>(() => Level.Cefr("D1"));
            Assert.Throws<ArgumentException>(() => Level.Jlpt("N6"));

            var judged = (Action<CheckpointBuilder>)(c => c.Edge("e", Outcome.Success, when: "w"));
            Assert.Equal("id", Refused(new DialogueBuilder("Bad-Id", start: "a").Npc("n", "N").Checkpoint("a", "A", judged)));
            Assert.Equal("facts[1].key", Refused(Small(b => b.Fact("x", FactType.Boolean)).Checkpoint("a", "A", judged)));
            Assert.Equal("checkpoints[1].edges[0].id", Refused(Small().Checkpoint("a", "A", judged).Checkpoint("b", "B", judged)));
            Assert.Equal("checkpoints[0].edges[0].to", Refused(Small().Checkpoint("a", "A", c => c.Edge("e", "nowhere", when: "w"))));
            Assert.Equal("checkpoints[1].id", Refused(Small().Checkpoint("a", "A", judged).Checkpoint("success", "S")));
            Assert.Equal("checkpoints[0].may_disclose[0]", Refused(Small().Checkpoint("a", "A", c => c.Discloses("secret").Edge("e", Outcome.Success, when: "w"))));
            foreach (var x in new[] { 0.00001, 1000000000.5, 0.12345, 9007199254740992d })
            {
                Assert.Equal("facts[1].default", Refused(Small(b => b.Fact("y", FactType.Number, defaultValue: x)).Checkpoint("a", "A", judged)));
            }

            Assert.Equal("facts[1].default", Refused(Small(b => b.Fact("y", FactType.Number, defaultValue: true)).Checkpoint("a", "A", judged)));
            Assert.Equal("npcs[1].languages[0]", Refused(Small(b => b.Npc("m", "M", n => n.Speaks("en", 10, 1))).Checkpoint("a", "A", judged)));

            // Two checkpoints joined both ways with no `when`: refused. One judged edge breaks it.
            var loop = Small().Checkpoint("a", "A", c => c.Edge("forth", "b", predicate: "x > 1")).Checkpoint("b", "B", c => c.Edge("back", "a", predicate: "x < 1"));
            Assert.StartsWith("checkpoints[", Refused(loop));
            var judgedLoop = Small().Checkpoint("a", "A", c => c.Edge("forth", "b", predicate: "x > 1")).Checkpoint("b", "B", c => c.Edge("back", "a", when: "the player asks to go back"));
            Assert.Equal(2, judgedLoop.Build().Checkpoints.Count);
        }

        // 28.10.26g AC12
        [Fact]
        public void LoadSendsTheCanonicalText()
        {
            var fake = new FakeNative { Answer = "{\"type\":\"id\",\"id\":7}" };
            using var client = LingaraEmbedClient.Create(Support.Offline(), fake, LingaraEmbedClient.PollBufferBytes);
            var ir = MissingBrother();
            var id = client.DialogueLoad(ir, new GraphSession { Npc = "elena", SourceLang = "en", TargetLang = "es", Level = 2 });
            Assert.Equal(7, id.Value);
            var sent = JsonReader.Parse(Encoding.UTF8.GetBytes(fake.Commands.Last()));
            Assert.Equal("dialogue_load", sent.Str("type"));
            Assert.Equal("ir", sent["source"].Str("type"));
            Assert.Equal(ir.ToCanonicalJson(), sent["source"].Str("ir_json"));
            Assert.Equal("elena", sent["session"].Str("npc"));
        }
    }
}
