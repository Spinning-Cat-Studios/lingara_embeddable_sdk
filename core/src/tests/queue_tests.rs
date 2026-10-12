use std::time::Duration;

use serde_json::json;

use crate::embed::Embed;
use crate::tests::fake_lingara::{After, FakeLingara, Reply, event_frame};
use crate::tests::{SECS, config, token};
use crate::types::{EmbedConfig, EmbedEvent, TokenReason};

// 1.10.26y AC16
#[test]
fn a_full_queue_backpressures_without_loss() {
    let frames: Vec<String> = (0..24).map(|i| event_frame(&format!("e{i}"), "game.custom", json!({ "n": i }))).collect();
    let fake = FakeLingara::start(move |_, _| Reply::Sse(frames.clone(), After::Hold));
    let embed = Embed::start(EmbedConfig { queue_capacity: 8, stream_on_start: true, ..config(&fake.url) }).unwrap();
    assert!(matches!(embed.poll_event(), Some(EmbedEvent::TokenNeeded { reason: TokenReason::Initial, .. })));
    // Stale at 1 s: the next need arrives while the host is not polling.
    embed.provide_token_for(token(1), 2).unwrap();
    std::thread::sleep(Duration::from_millis(1_500));
    assert!(
        matches!(embed.poll_event(), Some(EmbedEvent::TokenNeeded { reason: TokenReason::Expiring, .. })),
        "a full queue never hides TokenNeeded"
    );
    let mut ids = Vec::new();
    let deadline = std::time::Instant::now() + 5 * SECS;
    while ids.len() < 24 && std::time::Instant::now() < deadline {
        match embed.poll_event() {
            Some(EmbedEvent::Lingara { event }) => ids.push(event.id),
            Some(_) => {}
            None => std::thread::sleep(Duration::from_millis(5)),
        }
    }
    let want: Vec<String> = (0..24).map(|i| format!("e{i}")).collect();
    assert_eq!(ids, want, "every event once, in order, three queue-fulls of them");
    assert_eq!(fake.count("GET", "/v1/events/stream"), 1, "backpressure, not a reconnect");
}
