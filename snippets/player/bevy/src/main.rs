// The Bevy snippets the docs show (ADR 1.10.26z D13): a game that adds
// bevy_lingara, gets its tokens from its own server, talks to an NPC and
// practises. It compiles in CI; its regions are what the docs page reads.

use bevy::prelude::*;
use bevy_lingara::lingara_embed::PracticeSource;
use bevy_lingara::*;
// lingara:begin dialogueGraph
use bevy_lingara::lingara_embed::dialogue_ir::build::{Checkpoint, Dialogue, Edge, Fact, Npc, Outcome};
use bevy_lingara::lingara_embed::dialogue_ir::{DialogueIr, IrError, Level, Pedagogy};
use bevy_lingara::lingara_embed::{DialogueOutcome, FactValue};
// lingara:end

// lingara:begin tokenSource
/// The common case: the plugin POSTs to your own server's token route, with
/// your game's session as a header, and reads `{token, expires_at}`.
fn lingara_config(session_header: bevy_lingara::lingara_embed::Header) -> LingaraConfig {
    LingaraConfig::new()
        .token_endpoint("https://game.example.com/lingara-token", vec![session_header])
        .source_lang("en")
        // The embed version this build was registered against: sent to your
        // route as `Lingara-Embed-Version`, for your server's mint.
        .embed_version("1.1.0")
}

/// Once per session, for you: log it, never draw it for the player.
fn on_update_available(mut updates: MessageReader<LingaraUpdateAvailable>) {
    for update in updates.read() {
        warn!("Lingara update: latest {:?}, deprecated {}", update.latest, update.deprecated);
    }
}

/// Or keep the session in ECS: leave the token endpoint unset, answer each
/// `LingaraTokenNeeded` with what your server returned.
fn answer_token_requests(mut needed: MessageReader<LingaraTokenNeeded>, mut provide: MessageWriter<ProvideToken>) {
    for _request in needed.read() {
        let (token, expires_at) = fetch_from_your_server();
        provide.write(ProvideToken::new(token, expires_at));
    }
}
// lingara:end

fn fetch_from_your_server() -> (String, String) {
    (String::new(), String::new())
}

/// The entity that holds the player's practice session.
#[derive(Component)]
struct Player;

fn main() {
    let header = bevy_lingara::lingara_embed::Header { name: "x-game-session".into(), value: "your session".into() };
    // lingara:begin poll
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(LingaraPlugin::new(lingara_config(header)))
        .add_systems(Update, on_lingara_event)
    // lingara:end
        .add_systems(Startup, (spawn_seller, spawn_elena))
        .add_systems(Update, (answer_token_requests, on_update_available, talk, show_reply, practise, on_practice_finished))
        .add_systems(Update, (raise_the_alarm, follow_the_story))
        .run();
}

// lingara:begin poll
/// The plugin drains Lingara's events once per frame: read them as Messages.
fn on_lingara_event(mut events: MessageReader<LingaraEventReceived>) {
    for LingaraEventReceived(event) in events.read() {
        info!("{}: {}", event.event_type, event.data_json);
    }
}
// lingara:end

// lingara:begin dialogue
fn spawn_seller(mut commands: Commands) {
    let mei = NpcDialogue::new("Mei", "zh", 2)
        .and_then(|npc| npc.with_persona("a dumpling seller"))
        .and_then(|npc| npc.with_scene("the night market"))
        .expect("within Lingara's bounds");
    commands.spawn(mei);
    commands.spawn(Player);
}

/// One turn at a time: a line said while the NPC is replying is a `Busy` error.
fn talk(keys: Res<ButtonInput<KeyCode>>, npcs: Query<Entity, With<NpcDialogue>>, mut say: MessageWriter<NpcSay>) {
    if keys.just_pressed(KeyCode::Digit1) {
        for entity in &npcs {
            say.write(NpcSay { entity, line: "饺子多少钱？".into() });
        }
    }
}

/// `streaming` grows as the reply arrives; `turns` holds the finished ones.
fn show_reply(transcripts: Query<&NpcTranscript, Changed<NpcTranscript>>) {
    for transcript in &transcripts {
        if let Some(partial) = &transcript.streaming {
            info!("…{partial}");
        }
    }
}
// lingara:end

// lingara:begin practice
fn practise(
    keys: Res<ButtonInput<KeyCode>>,
    player: Query<(Entity, Option<&PracticeSession>), With<Player>>,
    mut start: MessageWriter<StartPractice>,
    mut input: MessageWriter<PracticeInput>,
) {
    let Ok((entity, session)) = player.single() else { return };
    if keys.just_pressed(KeyCode::KeyP) && session.is_none() {
        let source = PracticeSource::Request {
            topic: "ordering food".into(),
            source_lang: "en".into(),
            target_lang: "zh".into(),
            level: 2,
            tags: vec![],
        };
        start.write(StartPractice { entity, source });
        // A demo plan practises, but must never report:
        // PracticeSource::Plan { plan_id, report: false }
    }
    if keys.just_pressed(KeyCode::Digit2) && session.is_some() {
        input.write(PracticeInput { entity, action: PracticeAction::Choose(1) });
    }
}

/// Written once. Lingara has already sent the one result report, which
/// reaches your webhook as `practice.completed`.
fn on_practice_finished(mut finished: MessageReader<PracticeFinished>) {
    for done in finished.read() {
        info!("practice ended in {:?}", done.snapshot.phase);
    }
}
// lingara:end

// lingara:begin dialogueGraph
/// A story as a graph: checkpoints joined by edges. An edge's `predicate` is
/// a game condition Lingara's core evaluates; its `when` is a conversational
/// one the server's judge reads. Two NPCs share this one graph, each with
/// their own fluency (1–9; `Level::cefr("A2")` is 2, `NATIVE` is 9).
pub fn missing_brother() -> Result<DialogueIr, IrError> {
    Dialogue::new("missing_brother", "introduction")
        .title("The missing brother")
        .pedagogy(Pedagogy::None)
        .npc(
            Npc::new("elena", "Elena")
                .persona("a farmer who distrusts soldiers")
                .speaks("es", Level::NATIVE, Level::NATIVE)
                .speaks("en", Level::cefr("A2")?, Level::cefr("B1")?),
        )
        .npc(
            Npc::new("tomas", "Tomás")
                .persona("Elena's neighbour, a retired soldier")
                .speaks("es", Level::NATIVE, Level::NATIVE)
                .speaks("en", Level(6), Level(7)),
        )
        .fact(Fact::number("trust").default(0))
        .fact(Fact::number("suspicion").default(0.4))
        .fact(Fact::boolean("alarm_raised").default(false))
        .fact(Fact::string("eastern_gate"))
        .checkpoint(
            Checkpoint::new("introduction", "Elena is wary and asks who the player is")
                .edge(Edge::new("alarm", Outcome::Failure).predicate("alarm_raised"))
                .edge(Edge::new("explained", "ask_about_brother").when("the player says who they are and why they are here")),
        )
        .checkpoint(
            Checkpoint::new("ask_about_brother", "Admit she saw soldiers ride toward \"the old mill\"; reveal the gate only if she trusts the player")
                .discloses("eastern_gate")
                .edge(Edge::new("gate", Outcome::Success).predicate("trust >= 0.4").when("the player asks where soldiers might take a prisoner"))
                .edge(Edge::new("neighbour", "ask_tomas").when("the player asks who else saw the soldiers"))
                .edge(Edge::new("leaves", Outcome::Abandoned).when("the player walks away")),
        )
        .checkpoint(
            Checkpoint::new("ask_tomas", "Tomás says the soldiers spoke of the road to 長安")
                .edge(Edge::new("suspicious", Outcome::Failure).predicate("suspicion > 0.5"))
                .edge(Edge::new("back_to_elena", "ask_about_brother").when("the player returns to Elena with what Tomás said")),
        )
        .build() // the core's own checks, so the load never refuses it
}

/// Elena on an entity: the plugin loads the graph with `DialogueLoad`, and
/// `NpcSay` talks to her exactly as to a free-form NPC.
fn spawn_elena(mut commands: Commands) {
    let story = missing_brother().expect("a valid dialogue");
    commands.spawn(NpcDialogue::graph(&story, "elena", "es", 2).expect("Elena is in the cast"));
}

/// The game sets facts; a predicate that reads one may cross an edge.
fn raise_the_alarm(keys: Res<ButtonInput<KeyCode>>, npcs: Query<Entity, With<NpcDialogue>>, mut set: MessageWriter<NpcSetFact>) {
    if keys.just_pressed(KeyCode::KeyA) {
        for entity in &npcs {
            set.write(NpcSetFact { entity, key: "alarm_raised".into(), value: FactValue::Boolean(true) });
        }
    }
}

fn follow_the_story(mut reached: MessageReader<CheckpointReached>, mut ended: MessageReader<DialogueEnded>) {
    for step in reached.read() {
        info!("now at {} (via {:?})", step.checkpoint, step.via);
    }
    for end in ended.read() {
        if end.outcome == DialogueOutcome::Success {
            info!("the player learned where the soldiers went");
        }
    }
}
// lingara:end
