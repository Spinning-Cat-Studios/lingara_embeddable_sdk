// The Bevy snippets the docs show (ADR 1.10.26z D13): a game that adds
// bevy_lingara, gets its tokens from its own server, talks to an NPC and
// practises. It compiles in CI; its regions are what the docs page reads.

use bevy::prelude::*;
use bevy_lingara::lingara_embed::PracticeSource;
use bevy_lingara::*;

// lingara:begin tokenSource
/// The common case: the plugin POSTs to your own server's token route, with
/// your game's session as a header, and reads `{token, expires_at}`.
fn lingara_config(session_header: bevy_lingara::lingara_embed::Header) -> LingaraConfig {
    LingaraConfig::new()
        .token_endpoint("https://game.example.com/lingara-token", vec![session_header])
        .source_lang("en")
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
        .add_systems(Startup, spawn_seller)
        .add_systems(Update, (answer_token_requests, talk, show_reply, practise, on_practice_finished))
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
