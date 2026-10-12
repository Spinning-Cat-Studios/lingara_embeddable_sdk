use std::time::Duration;

use crate::tests::{DIALOGUE, OFFLINE, answered_id, call, exec, open, poll, poll_until};
use crate::*;

// 1.10.26aa AC3
#[test]
fn a_short_buffer_keeps_the_event_and_names_its_size() {
    let handle = open(OFFLINE);
    // Two events in queue order: the start's token_needed, then the error a
    // token_unavailable makes of a waiting turn.
    let dialogue = answered_id(handle, DIALOGUE);
    let request = answered_id(handle, &format!(r#"{{"type": "dialogue_say", "dialogue": {dialogue}, "line": "hi"}}"#));
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(exec(handle, r#"{"type": "token_unavailable", "reason": "offline"}"#).0, LINGARA_EMBED_OK);
    std::thread::sleep(Duration::from_millis(200));

    let (status, needed, _) = call(0, |b, c, l| unsafe { lingara_embed_poll(handle, b, c, l) });
    assert_eq!(status, LINGARA_EMBED_BUFFER_TOO_SMALL, "the size probe");
    let (status, len, _) = call(needed - 1, |b, c, l| unsafe { lingara_embed_poll(handle, b, c, l) });
    assert_eq!((status, len), (LINGARA_EMBED_BUFFER_TOO_SMALL, needed), "one byte short names the exact size");

    let (status, len, text) = call(needed, |b, c, l| unsafe { lingara_embed_poll(handle, b, c, l) });
    assert_eq!((status, len), (LINGARA_EMBED_OK, needed));
    let first: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(first["type"], "token_needed", "the kept event is the first in queue order");

    let (next, before) = poll_until(handle, Duration::from_secs(5), |e| e["type"] == "error");
    assert_eq!(next["error"]["request"], request, "then the next event, {before:?}");
    assert!(before.iter().all(|e| e["type"] != "token_needed"), "the kept event is served once");
    assert!(poll(handle).is_none());
    unsafe { lingara_embed_free(handle) };
}
