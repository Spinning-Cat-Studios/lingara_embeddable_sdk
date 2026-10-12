// The Godot sample's one scene (ADR 1.10.26aa D11): fetch a token from
// tools/demo-token-server (the Lingara node's TokenUrl), open a dialogue and show
// the NPC's streamed reply, then run one practice item.
using Godot;
using Lingara.Embed;

public partial class Main : Control
{
    /// <summary>A demo plan's id: started with report = false, so playing it records nothing.</summary>
    [Export] public string DemoPlanId { get; set; } = "8f0e2c1a-0000-4000-8000-000000000001";

    private LingaraEmbedNode lingara;
    private Label status;
    private Label reply;
    private Label question;
    private VBoxContainer options;
    private long session;
    private PracticeSnapshot snapshot;

    public override void _Ready()
    {
        lingara = GetNode<LingaraEmbedNode>("Lingara");
        status = GetNode<Label>("Column/Status");
        reply = GetNode<Label>("Column/Reply");
        question = GetNode<Label>("Column/Question");
        options = GetNode<VBoxContainer>("Column/Options");

        lingara.NpcTurnFragment += (_, _, text) => reply.Text += text;
        lingara.NpcTurnFinished += (_, _, text) => { reply.Text = "Mei: " + text; status.Text = "Mei answered."; };
        lingara.Failed += (kind, message, _) => status.Text = $"{kind}: {message}";
        lingara.PracticeChanged += (_, _) => Render(lingara.Client.GetPracticeSnapshot(new SessionId(session)));

        var dialogue = lingara.OpenDialogue("Mei", "en", "zh", 2);
        lingara.Say(dialogue, "Ni hao! What do you sell here?");
        session = lingara.StartPractice(DemoPlanId, report: false);
    }

    // Not `Show`: Control already has one.
    private void Render(PracticeSnapshot next)
    {
        snapshot = next;
        foreach (var child in options.GetChildren())
        {
            child.QueueFree();
        }

        if (snapshot?.Question != null && snapshot.Phase == PracticePhase.Unanswered)
        {
            question.Text = snapshot.Question.Prompt;
            var choices = snapshot.Question.Options ?? new[] { "ni hao" };
            for (var i = 0; i < choices.Count; i++)
            {
                var index = i;
                var button = new Button { Text = choices[i] };
                button.Pressed += () => Render(lingara.Answer(session, snapshot, index, choices[index]));
                options.AddChild(button);
            }
        }
        else if (snapshot?.LastResult != null)
        {
            question.Text = $"{snapshot.LastResult.Outcome}: {snapshot.LastResult.Expected} — {snapshot.LastResult.Explanation}";
            var next = new Button { Text = "Next" };
            next.Pressed += () => Render(lingara.Next(session));
            options.AddChild(next);
        }
    }
}
