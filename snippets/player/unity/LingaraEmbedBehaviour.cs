// The optional Unity component over Lingara.Embed (ADR 1.10.26aa D10). Drop it on
// a GameObject, set the token URL, and listen to its C# events. It is a snippet,
// not part of the package: a nupkg cannot reference UnityEngine.dll.
using System;
using System.Collections;
using System.Collections.Generic;
using Lingara.Embed;
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
