//! The unit tests (ADR 1.10.26y). Most drive a real `Embed` and the real
//! library against `fake_lingara`'s loopback server in real time, with short
//! timeouts; the ones about long timers (a 60 s restart, a 120 s wait, an
//! 840 s stale point) run under paused time against `fake_api`.

mod fake_api;
mod fake_lingara;

mod dialogue_tests;
mod error_tests;
mod events_tests;
mod grading_tests;
mod graph_dialogue_tests;
mod lifecycle_tests;
mod machine_tests;
mod package_shape_tests;
mod practice_io_tests;
mod queue_tests;
mod runner_tests;
mod stored_dialogue_tests;
mod token_tests;
mod window_tests;
mod world_tests;

use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::embed::Embed;
use crate::tests::fake_api::{FakeApi, Handle};
use crate::types::{EmbedConfig, EmbedEvent, TokenReason, WorldContext};

/// An embed token of the right shape: `lgr_et_` and 43 characters.
pub fn token(n: u32) -> String {
    format!("lgr_et_{n:043}")
}

pub fn config(url: &str) -> EmbedConfig {
    EmbedConfig { base_url: url.to_owned(), stream_on_start: false, ..EmbedConfig::default() }
}

/// Started, its first need answered with `token(1)` for 900 s.
pub fn started(config: EmbedConfig) -> Embed {
    let embed = Embed::start(config).expect("start");
    assert!(matches!(embed.poll_event(), Some(EmbedEvent::TokenNeeded { reason: TokenReason::Initial, .. })));
    embed.provide_token_for(token(1), 900).expect("token");
    embed
}

/// Started over a fake `Api` with tokio's clock paused on the I/O thread.
pub fn paused(config: EmbedConfig, api: &Arc<FakeApi>) -> Embed {
    let api = Arc::clone(api);
    Embed::launch(config, None, Box::new(move |_, _| Ok(Arc::new(Handle(api)) as Arc<dyn crate::api::Api>)), true).expect("start")
}

/// Polls until an event matches, returning it and everything polled before
/// it. Panics after `within`.
pub fn wait_for(embed: &Embed, within: Duration, matches: impl Fn(&EmbedEvent) -> bool) -> (EmbedEvent, Vec<EmbedEvent>) {
    let deadline = Instant::now() + within;
    let mut before = Vec::new();
    while Instant::now() < deadline {
        match embed.poll_event() {
            Some(event) if matches(&event) => return (event, before),
            Some(event) => before.push(event),
            None => std::thread::sleep(Duration::from_millis(5)),
        }
    }
    panic!("no matching event within {within:?}; saw {before:#?}");
}

/// Every event polled for `period`.
pub fn collect(embed: &Embed, period: Duration) -> Vec<EmbedEvent> {
    let deadline = Instant::now() + period;
    let mut out = Vec::new();
    while Instant::now() < deadline {
        match embed.poll_event() {
            Some(event) => out.push(event),
            None => std::thread::sleep(Duration::from_millis(5)),
        }
    }
    out
}

pub fn context(scene: &str) -> WorldContext {
    WorldContext { scene: scene.into(), source_lang: "en".into(), target_lang: "zh".into(), level: 2, npc: None, tags: vec![], generate: false }
}

/// The `202` of `POST /v1/events`.
pub fn accepted(id: &str) -> Value {
    json!({ "id": id, "type": "world.context_changed", "created_at": "2026-10-01T12:00:00Z" })
}

/// A Lingara `/v1` refusal body.
pub fn refusal(code: &str) -> Value {
    json!({ "code": code, "error": format!("refused: {code}") })
}

pub const SECS: Duration = Duration::from_secs(1);
