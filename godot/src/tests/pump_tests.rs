//! The per-frame drain and the handle's life (ADR 1.10.26ab D2, D10).

use std::sync::Arc;

use lingara_embed::{EmbedEvent, LogLevel};

use super::fake_core::FakeCore;
use crate::pump::{Library, Pump, UNLOAD_JOIN_MS};

fn log(n: usize) -> EmbedEvent {
    EmbedEvent::Log { level: LogLevel::Info, target: "fake".into(), message: n.to_string() }
}

fn message(event: &EmbedEvent) -> String {
    match event {
        EmbedEvent::Log { message, .. } => message.clone(),
        other => format!("{other:?}"),
    }
}

/// 1.10.26ab AC3: over the fake core, 1 000 queued events take four frames
/// at `max_events_per_frame` 256, in order, with one `drain` call per frame;
/// the pump's stop path, which `stop()` and `_exit_tree` both call, calls
/// `close()` and never a blocking join; the deinit path calls
/// `close_and_wait(1000)` on every closed handle still listed.
#[test]
fn drain_is_bounded_and_only_deinit_joins() {
    let library = Arc::new(Library::default());
    let fake = FakeCore::default();
    fake.state().queue.extend((0..1000).map(log));
    let mut pump = Pump::new(Arc::clone(&library));
    assert!(!pump.install(Box::new(fake.clone())), "the first handle is the only one");

    let mut seen = Vec::new();
    for _ in 0..4 {
        seen.extend(pump.frame(256).iter().map(message));
    }
    assert_eq!(fake.state().drains, vec![256; 4], "one drain call per frame");
    assert_eq!(seen, (0..1000).map(|n| n.to_string()).collect::<Vec<_>>(), "every event, in order");

    pump.stop();
    assert_eq!(fake.state().closes, 1);
    assert!(fake.state().joins.is_empty(), "stop() never joins");
    assert!(!pump.is_running());
    assert_eq!(library.live_count(), 0);

    // The closing handle is still drained, so its `Closed` reaches the game;
    // draining it retires the handle to the library's list.
    let closed = pump.frame(256);
    assert!(matches!(closed.last(), Some(EmbedEvent::Closed { .. })));
    assert_eq!(library.retired_count(), 1);
    assert!(fake.state().joins.is_empty(), "a retired handle is not joined on the frame path");

    library.join_all(UNLOAD_JOIN_MS);
    assert_eq!(fake.state().joins, vec![1000], "the deinit path joins, bounded");
    assert_eq!(library.retired_count(), 0);
}

/// A second live handle is D2's warning; a freed node's handle still
/// reaches the deinit join, and a drained one is pruned at the next stop().
#[test]
fn a_second_handle_warns_and_a_freed_one_is_still_joined() {
    let library = Arc::new(Library::default());
    let (first, second) = (FakeCore::default(), FakeCore::default());
    let mut a = Pump::new(Arc::clone(&library));
    let mut b = Pump::new(Arc::clone(&library));
    assert!(!a.install(Box::new(first.clone())));
    assert!(b.install(Box::new(second.clone())), "two live handles for one player");

    drop(b);
    assert_eq!(second.state().closes, 1, "a freed node closes its handle");
    assert_eq!(library.retired_count(), 1);

    a.stop();
    a.frame(256);
    assert_eq!(library.retired_count(), 2);
    let mut c = Pump::new(Arc::clone(&library));
    c.stop();
    assert_eq!(library.retired_count(), 1, "the drained handle was pruned with a zero-timeout join");
    assert_eq!(first.state().joins, vec![0]);
    library.join_all(UNLOAD_JOIN_MS);
    assert_eq!(second.state().joins, vec![1000]);
}
