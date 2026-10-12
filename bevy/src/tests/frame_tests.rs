//! The frame (D4, D6): the drain bound, the event mapping, inbound
//! Messages, and a stalled core.

use std::time::{Duration, Instant};

use bevy::MinimalPlugins;
use bevy::app::App;
use lingara_embed::{
    CloseReason, Command, EmbedError, EmbedEvent, ErrorKind, LingaraEvent, LingaraEventDetail, LogLevel, NpcTurnUpdate, PracticePhase,
    StreamState, TokenReason, WorldContext,
};

use super::fake_core::{Fake, app, read, snapshot, write};
use crate::config::LingaraConfig;
use crate::messages::*;
use crate::plugin::LingaraPlugin;
use crate::{NpcDialogue, NpcSay};

fn stream_event(n: usize) -> EmbedEvent {
    let event = LingaraEvent {
        id: format!("e{n}"),
        event_type: "practice.completed".into(),
        created_at: "2026-10-01T12:00:00Z".into(),
        data_json: format!("{{\"n\":{n}}}"),
        detail: LingaraEventDetail::Other,
    };
    EmbedEvent::Lingara { event }
}

/// 1.10.26z AC1: drain(256) once per frame; 256, 256, 256, 232, in order.
#[test]
fn drain_is_bounded_per_frame_and_keeps_order() {
    let fake = Fake::default();
    (0..1000).for_each(|n| fake.push(stream_event(n)));
    let mut app = app(LingaraConfig::new(), &fake);
    let mut seen = Vec::new();
    let mut counts = Vec::new();
    for _ in 0..5 {
        app.update();
        let raw = read::<LingaraRawEvent>(&mut app);
        counts.push(raw.len());
        seen.extend(raw.into_iter().map(|LingaraRawEvent(e)| e));
    }
    assert_eq!(counts, [256, 256, 256, 232, 0]);
    assert_eq!(fake.state().drains, [256; 5]);
    let want: Vec<EmbedEvent> = (0..1000).map(stream_event).collect();
    assert_eq!(seen, want);
}

/// 1.10.26z AC2: each variant becomes its Message after its raw one;
/// `Stream` and `Closed` reach `LingaraStatus` only; `Log` makes no
/// Message; an event for an id with no live entity is raw only.
#[test]
fn every_core_event_maps_to_its_message() {
    let fake = Fake::default();
    let error = EmbedError::new(ErrorKind::RateLimited, "slow down");
    let events = vec![
        EmbedEvent::TokenNeeded { request: 1, reason: TokenReason::Initial },
        stream_event(7),
        EmbedEvent::WorldSent { request: 2, event_id: "lgr_evt_1".into(), reaction: None },
        EmbedEvent::Stream { state: StreamState::Live, cursor: Some("c1".into()) },
        EmbedEvent::Log { level: LogLevel::Warn, target: "t".into(), message: "m".into() },
        EmbedEvent::Practice { session: 99, snapshot: snapshot(PracticePhase::Reported) },
        EmbedEvent::NpcTurn { dialogue: 98, request: 97, update: NpcTurnUpdate::Finished { text: "hi".into() } },
        EmbedEvent::Error { error: error.clone() },
    ];
    events.iter().cloned().for_each(|e| fake.push(e));
    fake.state().stream = Some(StreamState::Live);
    let mut app = app(LingaraConfig::new(), &fake);
    app.update();
    let raw: Vec<EmbedEvent> = read::<LingaraRawEvent>(&mut app).into_iter().map(|r| r.0).collect();
    assert_eq!(raw, events);
    assert_eq!(read::<LingaraTokenNeeded>(&mut app), [LingaraTokenNeeded { request: 1, reason: TokenReason::Initial }]);
    let received = read::<LingaraEventReceived>(&mut app);
    assert_eq!((received.len(), received[0].0.data_json.as_str()), (1, "{\"n\":7}"));
    assert_eq!(read::<WorldContextSent>(&mut app), [WorldContextSent { request: 2, event_id: "lgr_evt_1".into(), reaction: None }]);
    assert_eq!(read::<LingaraError>(&mut app), [LingaraError { error, entity: None }]);
    assert!(read::<PracticeFinished>(&mut app).is_empty() && read::<NpcReplied>(&mut app).is_empty());
    assert_eq!(app.world().resource::<LingaraStatus>().embed.as_ref().map(|s| s.stream.clone()), Some(StreamState::Live));

    fake.push(EmbedEvent::Closed { reason: CloseReason::Internal });
    app.update();
    assert_eq!(read::<LingaraRawEvent>(&mut app).len(), 1);
    assert_eq!(app.world().resource::<LingaraStatus>().phase, LingaraPhase::Closed(CloseReason::Internal));
    assert!(!app.world().contains_resource::<crate::Lingara>());
}

/// 1.10.26z AC3: with the core's I/O stalled (nothing ever drains, no turn
/// ever answers), 100 frames under MinimalPlugins take under a second.
#[test]
fn a_stalled_core_never_blocks_the_frame() {
    let fake = Fake::default();
    let mut app = App::new();
    app.add_plugins(MinimalPlugins).add_plugins(LingaraPlugin::with_start(LingaraConfig::new(), fake.starter()));
    let npc = app.world_mut().spawn(NpcDialogue::new("Mei", "zh", 2).unwrap()).id();
    let started = Instant::now();
    for n in 0..100 {
        write(&mut app, NpcSay { entity: npc, line: format!("line {n}") });
        app.update();
    }
    assert!(started.elapsed() < Duration::from_secs(1), "100 frames took {:?}", started.elapsed());
    assert_eq!(fake.state().drains.len(), 100);
}

/// 1.10.26z AC21: `WorldContextChanged` reaches `send_context` unchanged; a
/// refusal comes back as a `LingaraError`.
#[test]
fn inbound_messages_reach_the_core() {
    let fake = Fake::default();
    let mut app = app(LingaraConfig::new(), &fake);
    let context = WorldContext {
        scene: "the night market".into(),
        source_lang: "en".into(),
        target_lang: "zh".into(),
        level: 2,
        npc: None,
        tags: vec!["market".into()],
        generate: true,
    };
    write(&mut app, WorldContextChanged(context.clone()));
    app.update();
    assert_eq!(fake.sent("send_context"), [Command::SendContext { context: context.clone() }]);
    assert!(read::<LingaraError>(&mut app).is_empty());

    fake.refuse("send_context", ErrorKind::InvalidInput);
    write(&mut app, WorldContextChanged(context));
    app.update();
    let errors = read::<LingaraError>(&mut app);
    assert_eq!(errors.iter().map(|e| e.error.kind).collect::<Vec<_>>(), [ErrorKind::InvalidInput]);
}
