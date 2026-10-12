use std::sync::Arc;
use std::time::Duration;

use serde_json::json;

use crate::error::ErrorKind;
use crate::limits;
use crate::tests::fake_api::FakeApi;
use crate::tests::fake_lingara::{After, FakeLingara, Reply, event_frame};
use crate::tests::{SECS, config, paused, started, token, wait_for};
use crate::types::{EmbedConfig, EmbedError, EmbedEvent, LingaraEventDetail, StreamState};

fn lingara(id: &'static str) -> impl Fn(&EmbedEvent) -> bool {
    move |e| matches!(e, EmbedEvent::Lingara { event } if event.id == id)
}

fn on(url: &str) -> EmbedConfig {
    EmbedConfig { stream_on_start: true, ..config(url) }
}

// 1.10.26y AC5
#[test]
fn the_tail_resumes_dedupes_and_reports_state() {
    let fake = FakeLingara::start(|_, i| match i {
        0 => Reply::Sse(vec![event_frame("e1", "game.custom", json!({})), event_frame("e2", "game.custom", json!({}))], After::Drop),
        _ => Reply::Sse(vec![event_frame("e2", "game.custom", json!({})), event_frame("e3", "game.newer_type", json!({ "keep": [1, 2] }))], After::Hold),
    });
    let embed = started(EmbedConfig { resume_cursor: Some("c0".into()), ..on(&fake.url) });
    let (e3, before) = wait_for(&embed, 6 * SECS, lingara("e3"));
    let EmbedEvent::Lingara { event } = e3 else { unreachable!() };
    assert_eq!((event.event_type.as_str(), event.detail), ("game.newer_type", LingaraEventDetail::Other));
    assert_eq!(serde_json::from_str::<serde_json::Value>(&event.data_json).unwrap(), json!({ "keep": [1, 2] }), "an unknown type keeps its raw data");
    let ids: Vec<_> = before.iter().filter_map(|e| if let EmbedEvent::Lingara { event } = e { Some(event.id.as_str()) } else { None }).collect();
    assert_eq!(ids, ["e1", "e2"], "the repeated e2 reaches the host once");
    let states: Vec<_> = before.iter().filter_map(|e| if let EmbedEvent::Stream { state, .. } = e { Some(state.clone()) } else { None }).collect();
    assert_eq!(states[..2], [StreamState::Connecting, StreamState::Live]);
    let seen = fake.seen();
    assert_eq!(seen[0].header("last-event-id"), Some("c0"), "the first open resumes from resume_cursor");
    assert_eq!(seen[1].header("last-event-id"), Some("e2"), "the reopen resumes after the drop");
    assert_eq!(embed.status().cursor.as_deref(), Some("e3"));
}

fn down(error: ErrorKind, retry: u64) -> impl Fn(&EmbedEvent) -> bool {
    move |e| matches!(e, EmbedEvent::Stream { state: StreamState::Down { error: err, retry_in_ms }, .. } if err.kind == error && *retry_in_ms == retry)
}

// 1.10.26y AC6
#[test]
fn gaps_reconnect_and_a_raised_tail_restarts() {
    assert_eq!(crate::types::EmbedConfig::default().stream_idle_timeout_ms, 45_000, "three missed heartbeats");
    let fake = FakeLingara::start(|_, i| match i {
        0 => Reply::Sse(vec![event_frame("e1", "game.custom", json!({}))], After::Silent),
        _ => Reply::Sse(vec![event_frame("e2", "game.custom", json!({}))], After::Hold),
    });
    let embed = started(EmbedConfig { stream_idle_timeout_ms: 1_000, ..on(&fake.url) });
    wait_for(&embed, 6 * SECS, lingara("e2"));
    assert_eq!(fake.seen()[1].header("last-event-id"), Some("e1"), "a silent gap is a drop");

    let api = Arc::new(FakeApi::default());
    let refusal = |kind| EmbedError::new(kind, "refused");
    api.tails.lock().unwrap().extend([vec![Err(refusal(ErrorKind::NotFound))], vec![Err(refusal(ErrorKind::Forbidden))], vec![]]);
    let embed = paused(on("http://127.0.0.1:9"), &api);
    wait_for(&embed, 5 * SECS, down(ErrorKind::NotFound, limits::STREAM_RESTART_MS));
    wait_for(&embed, 5 * SECS, down(ErrorKind::Forbidden, 0));
    let opens = api.at("tail");
    assert_eq!(opens[1] - opens[0], Duration::from_millis(limits::STREAM_RESTART_MS), "a raised tail, a 404 included, restarts after 60 s");
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(api.at("tail").len(), 2, "after a 403 the stream waits for the next token");
    embed.provide_token_for(token(1), 900).unwrap();
    let deadline = std::time::Instant::now() + 5 * SECS;
    while api.at("tail").len() < 3 && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(api.at("tail").len(), 3, "a new token reopens it");
}

// 1.10.26y AC27
#[test]
fn pause_holds_no_connection_and_resume_uses_the_cursor() {
    let fake = FakeLingara::start(|_, _| Reply::Sse(vec![event_frame("e1", "game.custom", json!({}))], After::Hold));
    let embed = started(on(&fake.url));
    wait_for(&embed, 5 * SECS, lingara("e1"));
    embed.stream_pause().unwrap();
    wait_for(&embed, 5 * SECS, |e| matches!(e, EmbedEvent::Stream { state: StreamState::Paused, .. }));
    let deadline = std::time::Instant::now() + 2 * SECS;
    while fake.open() > 0 && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(fake.open(), 0, "a paused stream holds no connection");
    assert_eq!((embed.status().stream, embed.status().cursor.as_deref()), (StreamState::Paused, Some("e1")));
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(fake.count("GET", "/v1/events/stream"), 1, "nothing is requested while paused");
    embed.stream_resume().unwrap();
    wait_for(&embed, 5 * SECS, |e| matches!(e, EmbedEvent::Stream { state: StreamState::Live, .. }));
    assert_eq!(fake.seen()[1].header("last-event-id"), Some("e1"));
}
