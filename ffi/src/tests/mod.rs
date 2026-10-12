//! The ABI's unit tests (ADR 1.10.26aa D12). They call the nine `extern "C"`
//! functions exactly as C would, through the small safe helpers below, and
//! run offline: with `stream_on_start: false` nothing needs a token at start,
//! and a `token_unavailable` fails every waiter as an event carrying its
//! `request`. Only `free`'s bounded join needs a peer: `loopback.rs`.

mod loopback;

mod abi_tests;
mod execute_tests;
mod handle_tests;
mod header_tests;
mod panic_tests;
mod poll_tests;
mod schema_tests;

use std::time::{Duration, Instant};

use serde_json::Value;

use crate::*;

/// The `#[cfg(test)]` intercept in `execute.rs`: never compiled into a release.
pub(crate) const PANIC_COMMAND: &[u8] = br#"{"type": "__panic"}"#;

/// No server listens here: a request that gets this far fails at once.
pub const OFFLINE: &str = r#"{"base_url": "http://127.0.0.1:9", "stream_on_start": false}"#;

pub const DIALOGUE: &str = r#"{"type": "dialogue_open", "config": {"npc": {"name": "Mei"}, "source_lang": "en", "target_lang": "zh", "level": 2}}"#;

/// An embed token of the right shape: `lgr_et_` and 43 characters.
pub fn token(n: u32) -> String {
    format!("lgr_et_{n:043}")
}

/// `lingara_embed_new` at this library's major, which must succeed.
pub fn open(config: &str) -> *mut LingaraEmbed {
    let mut handle = std::ptr::null_mut();
    let status = unsafe { lingara_embed_new(LINGARA_EMBED_ABI_VERSION_MAJOR, config.as_ptr(), config.len(), &mut handle) };
    assert_eq!(status, LINGARA_EMBED_OK, "new: {}", last_error());
    assert!(!handle.is_null());
    handle
}

/// Calls a buffer-taking function once with `cap` bytes.
pub fn call(cap: usize, f: impl FnOnce(*mut u8, usize, *mut usize) -> i32) -> (i32, usize, String) {
    let mut buf = vec![0u8; cap];
    let mut len = usize::MAX;
    let status = f(if cap == 0 { std::ptr::null_mut() } else { buf.as_mut_ptr() }, cap, &mut len);
    let text = if len <= cap { String::from_utf8_lossy(&buf[..len]).into_owned() } else { String::new() };
    (status, len, text)
}

/// `execute` into a buffer large enough for any answer here.
pub fn exec(handle: *mut LingaraEmbed, command: &str) -> (i32, String) {
    exec_bytes(handle, command.as_bytes())
}

pub fn exec_bytes(handle: *mut LingaraEmbed, command: &[u8]) -> (i32, String) {
    let (status, _, text) = call(64 * 1024, |b, c, l| unsafe { lingara_embed_execute(handle, command.as_ptr(), command.len(), b, c, l) });
    (status, text)
}

pub fn json(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|e| panic!("{e}: {text}"))
}

/// The `id` of an `{"type": "id"}` answer.
pub fn answered_id(handle: *mut LingaraEmbed, command: &str) -> u64 {
    let (status, text) = exec(handle, command);
    assert_eq!(status, LINGARA_EMBED_OK, "{command}: {text} / {}", last_error());
    json(&text)["id"].as_u64().expect("an id answer")
}

pub fn poll(handle: *mut LingaraEmbed) -> Option<Value> {
    let (status, _, text) = call(64 * 1024, |b, c, l| unsafe { lingara_embed_poll(handle, b, c, l) });
    match status {
        LINGARA_EMBED_OK => Some(json(&text)),
        LINGARA_EMBED_EMPTY => None,
        other => panic!("poll returned {other}: {}", last_error()),
    }
}

/// Polls until `matches`, returning it and what came before. Panics after `within`.
pub fn poll_until(handle: *mut LingaraEmbed, within: Duration, matches: impl Fn(&Value) -> bool) -> (Value, Vec<Value>) {
    let deadline = Instant::now() + within;
    let mut before = Vec::new();
    while Instant::now() < deadline {
        match poll(handle) {
            Some(event) if matches(&event) => return (event, before),
            Some(event) => before.push(event),
            None => std::thread::sleep(Duration::from_millis(5)),
        }
    }
    panic!("no matching event within {within:?}; saw {before:?}");
}

pub fn last_error() -> String {
    call(4096, |b, c, l| unsafe { lingara_embed_last_error(b, c, l) }).2
}

pub fn last_answer() -> (i32, String) {
    let (status, _, text) = call(64 * 1024, |b, c, l| unsafe { lingara_embed_last_answer(b, c, l) });
    (status, text)
}

/// Runs `f` on a thread of its own, so it starts with empty thread slots.
pub fn on_fresh_thread<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::spawn(f).join().expect("the test thread")
}
