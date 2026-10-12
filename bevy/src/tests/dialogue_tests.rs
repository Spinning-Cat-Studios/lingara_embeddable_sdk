//! NPC dialogue (D7): the component's lifecycle, its bounds, the transcript.

use bevy::app::App;
use bevy::ecs::entity::Entity;
use lingara_embed::{Command, DialoguePhase, EmbedError, EmbedEvent, ErrorKind, NpcTurnUpdate, Speaker, WindowEntry};

use super::fake_core::{Fake, app, read, write};
use crate::config::LingaraConfig;
use crate::messages::*;
use crate::{NpcDialogue, NpcTranscript};

fn mei() -> NpcDialogue {
    NpcDialogue::new("Mei", "zh", 2).unwrap().with_persona("a market seller").unwrap()
}

fn transcript(app: &App, npc: Entity) -> NpcTranscript {
    app.world().get::<NpcTranscript>(npc).cloned().expect("a transcript")
}

/// 1.10.26z AC5: insert opens (with B3's fields and the starting window);
/// a failed open writes an error and no transcript; a real level or scene
/// change forwards once; remove, despawn and replace all close.
#[test]
fn the_component_lifecycle_drives_the_session() {
    let fake = Fake::default();
    let mut app = app(LingaraConfig::new().source_lang("en"), &fake);
    let window = vec![WindowEntry { speaker: Speaker::Player, text: "你好".into() }];
    let npc = app.world_mut().spawn(mei().with_scene("the night market").unwrap().with_window(window.clone())).id();
    app.update();
    let opens = fake.sent("dialogue_open");
    let [Command::DialogueOpen { config }] = opens.as_slice() else { panic!("{opens:?}") };
    assert_eq!((config.npc.name.as_str(), config.npc.persona.as_deref(), config.scene.as_deref()), ("Mei", Some("a market seller"), Some("the night market")));
    assert_eq!((config.source_lang.as_str(), config.target_lang.as_str(), config.level, &config.window), ("en", "zh", 2, &window));
    let id = transcript(&app, npc).id().expect("an id");

    app.update();
    app.world_mut().get_mut::<NpcDialogue>(npc).unwrap().set_level(2).unwrap();
    app.update();
    assert!(fake.sent("dialogue_set_level").is_empty() && fake.sent("dialogue_set_scene").is_empty());
    app.world_mut().get_mut::<NpcDialogue>(npc).unwrap().set_level(3).unwrap();
    app.world_mut().get_mut::<NpcDialogue>(npc).unwrap().set_scene(Some("the tea house".into())).unwrap();
    app.update();
    app.update();
    assert_eq!(fake.sent("dialogue_set_level"), [Command::DialogueSetLevel { dialogue: id, level: 3 }]);
    assert_eq!(fake.sent("dialogue_set_scene"), [Command::DialogueSetScene { dialogue: id, scene: Some("the tea house".into()) }]);

    app.world_mut().entity_mut(npc).insert(NpcDialogue::new("Tanaka", "ja", 6).unwrap());
    app.update();
    assert_eq!(fake.sent("dialogue_close"), [Command::DialogueClose { dialogue: id }]);
    assert_eq!(fake.sent("dialogue_open").len(), 2);
    let second = transcript(&app, npc).id().expect("a new id");
    assert!(second != id && transcript(&app, npc).turns.is_empty());

    app.world_mut().entity_mut(npc).remove::<NpcDialogue>();
    let other = app.world_mut().spawn(mei()).id();
    app.update();
    let third = transcript(&app, other).id().unwrap();
    app.world_mut().entity_mut(other).despawn();
    app.update();
    let closes = fake.sent("dialogue_close");
    assert_eq!(closes[1..], [Command::DialogueClose { dialogue: second }, Command::DialogueClose { dialogue: third }]);

    fake.refuse("dialogue_open", ErrorKind::InvalidInput);
    let refused = app.world_mut().spawn(mei()).id();
    app.update();
    assert!(app.world().get::<NpcTranscript>(refused).is_none());
    let errors = read::<LingaraError>(&mut app);
    assert_eq!(errors.iter().map(|e| (e.error.kind, e.entity)).collect::<Vec<_>>(), [(ErrorKind::InvalidInput, Some(refused))]);
}

/// 1.10.26z AC6: K1's inbound bounds, at construction.
#[test]
fn construction_enforces_the_inbound_bounds() {
    let refused = |r: Result<NpcDialogue, EmbedError>| r.err().map(|e| e.kind) == Some(ErrorKind::InvalidInput);
    assert!(refused(NpcDialogue::new("", "zh", 2)));
    assert!(refused(NpcDialogue::new("  ", "zh", 2)));
    assert!(refused(NpcDialogue::new("x".repeat(33), "zh", 2)));
    assert!(NpcDialogue::new("x".repeat(32), "zh", 2).is_ok());
    assert!(refused(NpcDialogue::new("Mei", "zh", 0)));
    assert!(refused(NpcDialogue::new("Mei", "zh", 10)));
    assert!(NpcDialogue::new("Mei", "zh", 9).is_ok());
    assert!(refused(NpcDialogue::new("Mei", "zh", 2).unwrap().with_persona("p".repeat(121))));
    assert!(NpcDialogue::new("Mei", "zh", 2).unwrap().with_persona("p".repeat(120)).is_ok());
    assert!(refused(NpcDialogue::new("Mei", "zh", 2).unwrap().with_scene("s".repeat(161))));
    assert!(NpcDialogue::new("Mei", "zh", 2).unwrap().with_scene("场".repeat(160)).is_ok());
    let plain = NpcDialogue::new("Mei", "zh", 1).unwrap();
    assert!(plain.npc().persona.is_none() && plain.scene().is_none());
}

fn turn(dialogue: u64, request: u64, update: NpcTurnUpdate) -> EmbedEvent {
    EmbedEvent::NpcTurn { dialogue, request, update }
}

/// 1.10.26z AC7: sending → streaming (empty) → idle; fragments in order; a
/// notice keeps the turn; `Finished` adds the uncut text and one
/// `NpcReplied`; cancel and errors add nothing; `Busy` queues nothing.
#[test]
fn fragments_assemble_into_one_turn() {
    let fake = Fake::default();
    let mut app = app(LingaraConfig::new(), &fake);
    let npc = app.world_mut().spawn(mei()).id();
    app.update();
    let id = transcript(&app, npc).id().unwrap();
    write(&mut app, NpcSay { entity: npc, line: "多少钱？".into() });
    app.update();
    let Some(Command::DialogueSay { line, .. }) = fake.sent("dialogue_say").pop() else { panic!() };
    assert_eq!((line.as_str(), transcript(&app, npc).phase), ("多少钱？", DialoguePhase::Sending));
    let request = 2;
    fake.push(turn(id, request, NpcTurnUpdate::Opened));
    app.update();
    assert_eq!((transcript(&app, npc).phase, transcript(&app, npc).streaming.as_deref()), (DialoguePhase::Streaming, Some("")));
    let long = "很".repeat(600);
    for update in [NpcTurnUpdate::Fragment { text: "十块".into() }, NpcTurnUpdate::Notice { code: "truncated".into(), message: "m".into() }] {
        fake.push(turn(id, request, update));
    }
    fake.push(turn(id, request, NpcTurnUpdate::Fragment { text: long.clone() }));
    app.update();
    let t = transcript(&app, npc);
    assert_eq!((t.streaming.unwrap(), t.notice.map(|n| n.0), t.phase), (format!("十块{long}"), Some("truncated".into()), DialoguePhase::Streaming));
    fake.push(turn(id, request, NpcTurnUpdate::Finished { text: format!("十块{long}") }));
    app.update();
    let t = transcript(&app, npc);
    assert_eq!((t.phase, t.turns.len(), t.turns[0].reply.chars().count()), (DialoguePhase::Idle, 1, 602));
    assert_eq!(read::<NpcReplied>(&mut app), [NpcReplied { entity: npc, text: format!("十块{long}"), level: 2 }]);

    write(&mut app, NpcSay { entity: npc, line: "again".into() });
    app.update();
    write(&mut app, NpcCancel { entity: npc });
    app.update();
    assert_eq!(fake.sent("dialogue_cancel"), [Command::DialogueCancel { dialogue: id }]);
    fake.push(turn(id, 3, NpcTurnUpdate::Cancelled));
    write(&mut app, NpcSay { entity: npc, line: "third".into() });
    app.update();
    fake.push(EmbedEvent::Error { error: lingara_embed::EmbedError { request: Some(4), ..super::fake_core::error(ErrorKind::Unavailable) } });
    app.update();
    let t = transcript(&app, npc);
    assert_eq!((t.phase, t.turns.len(), t.streaming), (DialoguePhase::Idle, 1, None));
    let errors = read::<LingaraError>(&mut app);
    assert_eq!(errors.iter().map(|e| (e.error.kind, e.entity)).collect::<Vec<_>>(), [(ErrorKind::Unavailable, Some(npc))]);

    fake.refuse("dialogue_say", ErrorKind::Busy);
    write(&mut app, NpcSay { entity: npc, line: "busy".into() });
    app.update();
    let errors = read::<LingaraError>(&mut app);
    assert_eq!(errors.iter().map(|e| (e.error.kind, e.entity)).collect::<Vec<_>>(), [(ErrorKind::Busy, Some(npc))]);
    assert_eq!(transcript(&app, npc).phase, DialoguePhase::Idle);
}
