// The Unity sample's one scene (ADR 1.10.26aa D10): fetch a token from
// tools/demo-token-server, open a dialogue and show the NPC's streamed reply, then
// run one practice item. Drawn with IMGUI so the scene needs no Canvas.
using Lingara.Embed;
using Lingara.Embed.Events;
using UnityEngine;

public sealed class SampleScene : MonoBehaviour
{
    public LingaraEmbedBehaviour lingara;

    [Tooltip("A demo plan's id: it is started with report = false, so playing it records nothing.")]
    public string demoPlanId = "8f0e2c1a-0000-4000-8000-000000000001";

    private string reply = string.Empty;
    private string status = "waiting for a token…";
    private SessionId session;
    private PracticeSnapshot snapshot;

    private void Start()
    {
        lingara.NpcTurn += OnTurn;
        lingara.Practice += p => snapshot = p.Snapshot;
        lingara.Failed += f => status = $"{f.Error.KindName}: {f.Error.Message}";

        var dialogue = lingara.OpenDialogue("Mei", "en", "zh", 2);
        lingara.Say(dialogue, "Ni hao! What do you sell here?");
        session = lingara.StartPractice(demoPlanId, report: false);
    }

    private void OnTurn(EmbedEvent.NpcTurn turn)
    {
        switch (turn.Update)
        {
            case NpcTurnUpdate.Opened _: reply = string.Empty; status = "Mei is answering…"; break;
            case NpcTurnUpdate.Fragment f: reply += f.Text; break;
            case NpcTurnUpdate.Finished f: reply = f.Text; status = "Mei answered."; break;
        }
    }

    private void OnGUI()
    {
        GUILayout.BeginArea(new Rect(16, 16, Screen.width - 32, Screen.height - 32));
        GUILayout.Label(status);
        GUILayout.Label("Mei: " + reply);
        var question = snapshot?.Question;
        if (question != null && snapshot.Phase == PracticePhase.Unanswered)
        {
            GUILayout.Label(question.Prompt);
            var options = question.Options ?? new[] { "ni hao" };
            for (var i = 0; i < options.Count; i++)
            {
                if (GUILayout.Button(options[i]))
                {
                    snapshot = lingara.Answer(session, snapshot, i, options[i]);
                }
            }
        }
        else if (snapshot?.LastResult != null)
        {
            GUILayout.Label($"{snapshot.LastResult.Outcome}: {snapshot.LastResult.Expected} — {snapshot.LastResult.Explanation}");
            if (GUILayout.Button("Next")) snapshot = lingara.Next(session);
        }
        GUILayout.EndArea();
    }
}
