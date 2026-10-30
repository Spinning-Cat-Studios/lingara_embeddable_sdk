// The optional Godot 4 C# node over Lingara.Embed (ADR 1.10.26aa D11). Add it to
// a scene, set the token URL, and connect to its signals. GDScript projects use
// the Godot addon instead: never both in one project.
using System;
using System.Collections.Generic;
using System.Net.Http;
using System.Text.Json;
using System.Threading.Tasks;
using Godot;
using Lingara.Embed;
using Lingara.Embed.Dialogues;
using Lingara.Embed.Events;

public partial class LingaraEmbedNode : Node
{
    [Signal] public delegate void NpcTurnFragmentEventHandler(long dialogue, long request, string text);
    [Signal] public delegate void NpcTurnFinishedEventHandler(long dialogue, long request, string text);
    [Signal] public delegate void PracticeChangedEventHandler(long session, string phase);
    [Signal] public delegate void FailedEventHandler(string kind, string message, long request);

    /// <summary>Your own server's endpoint: it mints a native embed token (no origin) and returns { token, expires_at }.</summary>
    [Export] public string TokenUrl { get; set; } = "http://127.0.0.1:8787/token"; // tools/demo-token-server while you develop

    public LingaraEmbedClient Client { get; private set; }

    // System.Net.Http's, not Godot's own HttpClient node.
    private static readonly System.Net.Http.HttpClient Http = new System.Net.Http.HttpClient();
    private readonly List<EmbedEvent> events = new List<EmbedEvent>();

    public override void _Ready()
    {
        Client = LingaraEmbedClient.Create(new EmbedOptions
        {
            StreamOnStart = false,
            UserAgentSuffix = "godot/" + (string)Engine.GetVersionInfo()["string"],
        });
    }

    public override void _ExitTree()
    {
        Client?.Dispose();
        Client = null;
    }

    // lingara:begin poll
    public override void _Process(double delta)
    {
        if (Client == null) return;
        events.Clear();
        Client.Drain(events); // at most 256 a frame; the rest wait for the next
        foreach (var e in events)
        {
            switch (e)
            {
                case EmbedEvent.TokenNeeded _: _ = FetchToken(); break;
                case EmbedEvent.NpcTurn { Update: NpcTurnUpdate.Fragment f } t:
                    EmitSignal(SignalName.NpcTurnFragment, t.Dialogue.Value, t.Request.Value, f.Text); break;
                case EmbedEvent.NpcTurn { Update: NpcTurnUpdate.Finished f } t:
                    EmitSignal(SignalName.NpcTurnFinished, t.Dialogue.Value, t.Request.Value, f.Text); break;
                case EmbedEvent.Practice p: EmitSignal(SignalName.PracticeChanged, p.Session.Value, p.Snapshot.Phase.ToString()); break;
                case EmbedEvent.Failed f: EmitSignal(SignalName.Failed, f.Error.KindName, f.Error.Message, f.Error.Request?.Value ?? 0); break;
                case EmbedEvent.Log log: GD.Print($"[lingara] {log.Level} {log.Target}: {log.Message}"); break;
            }
        }
    }
    // lingara:end

    // lingara:begin tokenSource
    // Your own server mints the token: never put a client secret in a game. The
    // continuation may run off the main thread, which the client allows.
    private async Task FetchToken()
    {
        try
        {
            using var response = await Http.PostAsync(TokenUrl, new StringContent("{}"));
            response.EnsureSuccessStatusCode();
            using var body = JsonDocument.Parse(await response.Content.ReadAsStringAsync());
            Client?.ProvideToken(body.RootElement.GetProperty("token").GetString(), body.RootElement.GetProperty("expires_at").GetString());
        }
        catch (Exception e) when (e is HttpRequestException || e is JsonException || e is KeyNotFoundException)
        {
            Client?.TokenUnavailable(e.Message); // fail the waiting calls now
        }
    }
    // lingara:end

    // lingara:begin dialogue
    public long OpenDialogue(string npc, string sourceLang, string targetLang, byte level) =>
        Client.DialogueOpen(new DialogueConfig { Npc = new Npc { Name = npc }, SourceLang = sourceLang, TargetLang = targetLang, Level = level }).Value;

    // The reply streams back as NpcTurnFragment signals, then one NpcTurnFinished.
    public long Say(long dialogue, string line) => Client.DialogueSay(new DialogueId(dialogue), line).Value;
    // lingara:end

    // lingara:begin dialogueGraph
    // A story as a graph: checkpoints joined by edges. An edge's predicate is a game
    // condition Lingara's core evaluates; its `when` is a conversational one the server's
    // judge reads. Two NPCs share this graph, each with their own fluency (1–9).
    public static Dialogue MissingBrother() => new DialogueBuilder("missing_brother", start: "introduction")
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
        .Build(); // structural checks; DialogueLoad runs the core's full ones

    // Elena speaks Spanish to a learner at level 2; CheckpointReached and DialogueEnded
    // events follow as the story moves, and Say talks to her as to any NPC.
    public long LoadElena(string sourceLang) =>
        Client.DialogueLoad(MissingBrother(), new GraphSession { Npc = "elena", SourceLang = sourceLang, TargetLang = "es", Level = 2 }).Value;

    public void RaiseTheAlarm(long elena) => Client.DialogueSetFact(new DialogueId(elena), "alarm_raised", FactValue.Of(true));
    // lingara:end

    // lingara:begin practice
    // A demo plan reports nothing: start it with report = false.
    public long StartPractice(string planId, bool report) =>
        Client.PracticeStart(new PracticeSource.Plan { PlanId = planId, Report = report }).Value;

    // Each answer returns the new snapshot at once: show its LastResult, then Next.
    public PracticeSnapshot Answer(long session, PracticeSnapshot current, int choice, string typed) =>
        current.Question?.Options != null
            ? Client.PracticeChoose(new SessionId(session), (uint)choice)
            : Client.PracticeAnswer(new SessionId(session), typed);

    public PracticeSnapshot Next(long session) => Client.PracticeNext(new SessionId(session));
    // lingara:end
}
