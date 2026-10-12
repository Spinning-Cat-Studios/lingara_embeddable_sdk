//! Connect, disconnect, exit, drop, and no live handle (D3).

use std::time::{Duration, Instant};

use bevy::app::AppExit;
use lingara_embed::{CloseReason, Command, EmbedConfig, EmbedEvent, ErrorKind, PracticePhase, PracticeSource, WorldContext};

use super::fake_core::{Fake, app, read, snapshot, write};
use crate::config::LingaraConfig;
use crate::core::Lingara;
use crate::messages::*;
use crate::{NpcDialogue, NpcTranscript, PracticeSession};

fn closed(fake: &Fake) {
    fake.push(EmbedEvent::Closed { reason: CloseReason::Requested });
}

/// 1.10.26z AC18: deferred start, an ignored second connect, disconnect
/// waiting for `Closed`, a fresh handle after it, `AppExit` closing once,
/// and a drop that never joins K1's thread.
#[test]
fn connect_disconnect_exit_and_drop() {
    let fake = Fake::default();
    let mut app = app(LingaraConfig::new().autostart(false), &fake);
    app.update();
    assert!(fake.state().starts.is_empty() && !app.world().contains_resource::<Lingara>());
    write(&mut app, LingaraConnect);
    app.update();
    write(&mut app, LingaraConnect);
    app.update();
    assert_eq!((fake.state().starts.len(), &app.world().resource::<LingaraStatus>().phase), (1, &LingaraPhase::Running));

    write(&mut app, LingaraDisconnect);
    app.update();
    assert_eq!(fake.state().closes, 1);
    assert!(app.world().contains_resource::<Lingara>(), "kept until Closed is drained");
    closed(&fake);
    app.update();
    assert!(!app.world().contains_resource::<Lingara>());
    write(&mut app, LingaraConnect);
    app.update();
    assert_eq!(fake.state().starts.len(), 2);

    write(&mut app, AppExit::Success);
    app.update();
    write(&mut app, AppExit::Success);
    app.update();
    assert_eq!(fake.state().closes, 2, "one close for the disconnect, one for the exit");

    // The real core: dropping its resource is `close()` with no join.
    let config = EmbedConfig { base_url: "http://127.0.0.1:9".into(), stream_on_start: false, ..EmbedConfig::default() };
    let embed = lingara_embed::Embed::start(config).expect("a started core");
    let lingara = Lingara::new(Box::new(embed));
    assert!(lingara.handle().embed().is_some());
    let started = Instant::now();
    drop(lingara);
    assert!(started.elapsed() < Duration::from_millis(200), "drop took {:?}", started.elapsed());
}

fn context() -> WorldContext {
    WorldContext { scene: "s".into(), source_lang: "en".into(), target_lang: "zh".into(), level: 2, npc: None, tags: vec![], generate: false }
}

/// 1.10.26z AC22: with no live handle, inbound Messages are `Closed` errors;
/// an NPC opens at the next Connect; a close drops every transcript's id
/// and detaches unfinished sessions; a reconnect says through a fresh id; a
/// failed start is `StartFailed`.
#[test]
fn no_live_handle_refuses_defers_and_detaches() {
    let fake = Fake::default();
    let mut app = app(LingaraConfig::new().autostart(false), &fake);
    let npc = app.world_mut().spawn(NpcDialogue::new("Mei", "zh", 2).unwrap()).id();
    let learner = app.world_mut().spawn_empty().id();
    write(&mut app, WorldContextChanged(context()));
    write(&mut app, NpcSay { entity: npc, line: "hi".into() });
    write(&mut app, StartPractice { entity: learner, source: PracticeSource::Plan { plan_id: "p".into(), report: false } });
    write(&mut app, ProvideToken::new("lgr_et_x", "2026-10-01T12:00:00Z"));
    app.update();
    let kinds: Vec<ErrorKind> = read::<LingaraError>(&mut app).iter().map(|e| e.error.kind).collect();
    assert_eq!(kinds, [ErrorKind::Closed; 4]);
    assert!(fake.state().commands.is_empty());

    write(&mut app, LingaraConnect);
    app.update();
    let first = app.world().get::<NpcTranscript>(npc).and_then(NpcTranscript::id).expect("opened at connect");
    write(&mut app, StartPractice { entity: learner, source: PracticeSource::Plan { plan_id: "p".into(), report: false } });
    app.update();
    let session = app.world().get::<PracticeSession>(learner).unwrap().id();
    fake.push(EmbedEvent::Practice { session, snapshot: snapshot(PracticePhase::Unanswered) });
    write(&mut app, LingaraDisconnect);
    app.update();
    closed(&fake);
    app.update();
    let t = app.world().get::<NpcTranscript>(npc).unwrap();
    assert!(t.id().is_none() && t.phase == lingara_embed::DialoguePhase::Idle);
    assert!(app.world().get::<PracticeSession>(learner).unwrap().is_detached());
    let detached: Vec<_> = read::<LingaraError>(&mut app).iter().map(|e| (e.error.kind, e.entity)).collect();
    assert_eq!(detached, [(ErrorKind::Closed, Some(learner))]);

    write(&mut app, LingaraConnect);
    app.update();
    write(&mut app, NpcSay { entity: npc, line: "again".into() });
    app.update();
    let second = app.world().get::<NpcTranscript>(npc).and_then(NpcTranscript::id).unwrap();
    assert_ne!(first, second);
    assert_eq!(fake.sent("dialogue_say"), [Command::DialogueSay { dialogue: second, line: "again".into() }]);

    let failing = Fake::default();
    failing.state().start_error = Some(super::fake_core::error(ErrorKind::Config));
    let mut app = super::fake_core::app(LingaraConfig::new(), &failing);
    assert!(matches!(app.world().resource::<LingaraStatus>().phase, LingaraPhase::StartFailed(_)));
    app.update();
    assert_eq!(read::<LingaraError>(&mut app).iter().map(|e| e.error.kind).collect::<Vec<_>>(), [ErrorKind::Config]);
}
