use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use serde_json::json;

use crate::embed::Embed;
use crate::error::ErrorKind;
use crate::runtime::SPAWNED;
use crate::tests::fake_api::FakeApi;
use crate::tests::fake_lingara::{After, FakeLingara, Reply, event_frame};
use crate::tests::{SECS, config, context, paused, started, wait_for};
use crate::types::{CloseReason, Command, EmbedConfig, EmbedEvent, LogLevel, StreamState, TokenSourceConfig};

fn closed(kind: Result<impl std::fmt::Debug, crate::types::EmbedError>) -> bool {
    kind.is_err_and(|e| e.kind == ErrorKind::Closed)
}

// 1.10.26y AC14
#[test]
fn close_drops_every_connection_and_ends_the_queue() {
    let fake = FakeLingara::start(|_, _| Reply::Sse(vec![event_frame("e1", "game.custom", json!({}))], After::Hold));
    let embed = started(EmbedConfig { stream_on_start: true, ..config(&fake.url) });
    wait_for(&embed, 5 * SECS, |e| matches!(e, EmbedEvent::Stream { state: StreamState::Live, .. }));
    assert_eq!(fake.open(), 1);
    let begun = Instant::now();
    embed.close();
    assert!(begun.elapsed() < Duration::from_millis(50), "close returns at once");
    while fake.open() > 0 && begun.elapsed() < 2 * SECS {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(fake.open(), 0, "every connection is dropped within 2 s");
    assert!(matches!(embed.poll_event(), Some(EmbedEvent::Closed { reason: CloseReason::Requested })));
    assert!((0..3).all(|_| embed.poll_event().is_none()), "one Closed, then nothing");
    assert!(closed(embed.send_context(context("market"))));
    assert!(closed(embed.provide_token_for(crate::tests::token(2), 900)));
    assert!(closed(embed.execute(Command::StreamPause)));
    assert!(closed(embed.practice_answer(1, "x")));
    assert!(embed.status().closed);
    assert!(embed.close_and_wait(1_000), "the I/O thread exits in time");
}

// 1.10.26y AC29
#[test]
fn bad_config_refuses_and_a_task_panic_closes_cleanly() {
    let base = || config("http://127.0.0.1:9");
    let http = |url: &str| EmbedConfig { token_source: TokenSourceConfig::Http { url: url.into(), headers: vec![] }, ..base() };
    let cases = [
        EmbedConfig { base_url: "not a url".into(), ..base() },
        http("ftp://tokens.example"),
        EmbedConfig { api_version: Some(String::new()), ..base() },
        EmbedConfig { queue_capacity: 0, ..base() },
        EmbedConfig { stream_idle_timeout_ms: 999, ..base() },
    ];
    let before = SPAWNED.with(|n| n.get());
    for case in cases {
        let err = Embed::start(case.clone()).expect_err(&format!("{case:?} must be refused"));
        assert_eq!((err.kind, err.retryable), (ErrorKind::Config, false), "{case:?}");
    }
    assert_eq!(SPAWNED.with(|n| n.get()), before, "a refused config starts no thread");

    let api = Arc::new(FakeApi::default());
    api.panic_on_tail.store(true, Ordering::SeqCst);
    let embed = paused(EmbedConfig { stream_on_start: true, log_level: LogLevel::Warn, ..base() }, &api);
    let (closing, _) = wait_for(&embed, 5 * SECS, |e| matches!(e, EmbedEvent::Log { level: LogLevel::Error, .. }));
    assert!(matches!(closing, EmbedEvent::Log { message, .. } if message.contains("panicked")));
    assert!(matches!(embed.poll_event(), Some(EmbedEvent::Closed { reason: CloseReason::Internal })));
    assert!(embed.poll_event().is_none());
    assert!(closed(embed.send_context(context("market"))));
}
