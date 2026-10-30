// The optional Unity component over Lingara.Embed (ADR 1.10.26aa D10). Drop it on
// a GameObject, set the token URL, and listen to its C# events. It is a snippet,
// not part of the package: a nupkg cannot reference UnityEngine.dll.
using System;
using System.Collections;
using System.Collections.Generic;
using Lingara.Embed;
using Lingara.Embed.Dialogues;
using Lingara.Embed.Events;
using UnityEngine;
using UnityEngine.Networking;

public sealed class LingaraEmbedBehaviour : MonoBehaviour
{
    [Tooltip("Your own server's endpoint: it mints a native embed token (no origin) and returns { token, expires_at }.")]
    public string tokenUrl = "http://127.0.0.1:8787/token"; // tools/demo-token-server while you develop
    public bool streamOnStart;

    public event Action<EmbedEvent.NpcTurn> NpcTurn;
    public event Action<EmbedEvent.Practice> Practice;
    public event Action<EmbedEvent.LingaraReceived> LingaraReceived;
    public event Action<EmbedEvent.Failed> Failed;

    public LingaraEmbedClient Client { get; private set; }

    private readonly List<EmbedEvent> events = new List<EmbedEvent>();

    private void Awake()
    {
#if UNITY_IOS && !UNITY_EDITOR
        LingaraEmbedClient.NativeMode = NativeLibraryMode.StaticInternal;
#endif
        Client = LingaraEmbedClient.Create(new EmbedOptions
        {
            StreamOnStart = streamOnStart,
            UserAgentSuffix = "unity/" + Application.unityVersion,
        });
#if UNITY_EDITOR
        // Unity never unloads a native plugin in the editor: a client leaked across
        // a domain reload would keep its I/O thread alive.
        UnityEditor.AssemblyReloadEvents.beforeAssemblyReload += Shutdown;
#endif
        Application.quitting += Shutdown;
    }

    private void OnDestroy() => Shutdown();

    private void Shutdown()
    {
#if UNITY_EDITOR
        UnityEditor.AssemblyReloadEvents.beforeAssemblyReload -= Shutdown;
#endif
        Application.quitting -= Shutdown;
        Client?.Dispose();
        Client = null;
    }

    // lingara:begin poll
    private void Update()
    {
        if (Client == null) return;
        events.Clear();
        Client.Drain(events); // at most 256 a frame; the rest wait for the next
        foreach (var e in events)
        {
            switch (e)
            {
                case EmbedEvent.TokenNeeded _: StartCoroutine(FetchToken()); break;
                case EmbedEvent.NpcTurn turn: NpcTurn?.Invoke(turn); break;
                case EmbedEvent.Practice practice: Practice?.Invoke(practice); break;
                case EmbedEvent.LingaraReceived received: LingaraReceived?.Invoke(received); break;
                case EmbedEvent.Failed failed: Failed?.Invoke(failed); break;
                case EmbedEvent.Log log: Debug.Log($"[lingara] {log.Level} {log.Target}: {log.Message}"); break;
            }
        }
    }
    // lingara:end

    // lingara:begin tokenSource
    [Serializable]
    private struct Handoff { public string token; public string expires_at; }

    // Your own server mints the token: never put a client secret in a game.
    private IEnumerator FetchToken()
    {
        using (var request = UnityWebRequest.PostWwwForm(tokenUrl, string.Empty))
        {
            yield return request.SendWebRequest();
            if (Client == null) yield break;
            if (request.result != UnityWebRequest.Result.Success)
            {
                Client.TokenUnavailable(request.error); // fail the waiting calls now
                yield break;
            }
            var handoff = JsonUtility.FromJson<Handoff>(request.downloadHandler.text);
            Client.ProvideToken(handoff.token, handoff.expires_at);
        }
    }
    // lingara:end

    // lingara:begin dialogue
    public DialogueId OpenDialogue(string npc, string sourceLang, string targetLang, byte level) =>
        Client.DialogueOpen(new DialogueConfig { Npc = new Npc { Name = npc }, SourceLang = sourceLang, TargetLang = targetLang, Level = level });

    // The reply streams back as NpcTurn events: append each Fragment's text to your label.
    public RequestId Say(DialogueId dialogue, string line) => Client.DialogueSay(dialogue, line);
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
    // events follow as the story moves, and DialogueSay talks to her as to any NPC.
    public DialogueId LoadElena(string sourceLang) =>
        Client.DialogueLoad(MissingBrother(), new GraphSession { Npc = "elena", SourceLang = sourceLang, TargetLang = "es", Level = 2 });

    public void RaiseTheAlarm(DialogueId elena) => Client.DialogueSetFact(elena, "alarm_raised", FactValue.Of(true));
    // lingara:end

    // lingara:begin practice
    // A demo plan reports nothing: start it with Report = false.
    public SessionId StartPractice(string planId, bool report) =>
        Client.PracticeStart(new PracticeSource.Plan { PlanId = planId, Report = report });

    // Each answer returns the new snapshot in the same frame: show its LastResult, then PracticeNext.
    public PracticeSnapshot Answer(SessionId session, PracticeSnapshot current, int choice, string typed) =>
        current.Question?.Options != null ? Client.PracticeChoose(session, (uint)choice) : Client.PracticeAnswer(session, typed);

    public PracticeSnapshot Next(SessionId session) => Client.PracticeNext(session);
    // lingara:end
}
