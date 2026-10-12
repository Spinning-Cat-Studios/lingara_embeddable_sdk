use std::time::Duration;

use crate::tests::{DIALOGUE, OFFLINE, call, exec, exec_bytes, json, last_answer, last_error, on_fresh_thread, open, poll};
use crate::*;

/// The raw token, and the parts of it any echo would carry.
const SECRET: &str = "lgr_et_DoNotEchoMe0000000000000000000000000000000";

fn assert_no_secret(where_: &str, text: &str) {
    assert!(!text.contains("DoNotEchoMe"), "{where_} echoes the secret: {text}");
    assert!(!text.contains("hunter2"), "{where_} echoes the header value: {text}");
}

// 1.10.26aa AC5
#[test]
fn a_bad_command_is_refused_with_a_message_and_no_secret() {
    let handle = open(OFFLINE);
    let cases: [(&[u8], i32, &str); 5] = [
        (b"{\"type\": \"stream_\xff\"}", LINGARA_EMBED_ERR_INVALID_UTF8, "ERR_INVALID_UTF8"),
        (b"[1, 2]", LINGARA_EMBED_ERR_INVALID_JSON, "ERR_INVALID_JSON"),
        (br#"{"kind": "x"}"#, LINGARA_EMBED_ERR_INVALID_JSON, "ERR_INVALID_JSON"),
        (br#"{"type": "teleport", "to": "moon"}"#, LINGARA_EMBED_ERR_UNKNOWN_COMMAND, "`teleport`"),
        (br#"{"type": "practice_choose", "session": 3, "option": "first"}"#, LINGARA_EMBED_ERR_INVALID_ARGUMENT, "`practice_choose`"),
    ];
    for (bytes, status, names) in cases {
        assert_eq!(exec_bytes(handle, bytes).0, status, "{}", String::from_utf8_lossy(bytes));
        let message = last_error();
        assert!(message.contains(names), "{message}");
        assert!(message.starts_with("ERR_"), "the message names the status: {message}");
    }

    // A malformed provide_token: serde's own text would quote the value.
    let bad = format!(r#"{{"type": "provide_token", "token": "{SECRET}", "expires_at": 12}}"#);
    let (status, answer) = exec(handle, &bad);
    assert_eq!(status, LINGARA_EMBED_ERR_INVALID_ARGUMENT);
    assert!(last_error().contains("`provide_token`"));
    assert_no_secret("the last error", &last_error());
    assert_no_secret("the answer", &answer);
    // A token in the type itself is not printed either.
    assert_eq!(exec(handle, &format!(r#"{{"type": "{SECRET}"}}"#)).0, LINGARA_EMBED_ERR_UNKNOWN_COMMAND);
    assert_no_secret("an unknown type", &last_error());
    // A well-formed one with a bad token is the core's refusal, redacted.
    let refused = format!(r#"{{"type": "provide_token", "token": "{SECRET}x", "expires_at": "not a time"}}"#);
    assert_eq!(exec(handle, &refused).0, LINGARA_EMBED_REFUSED);
    assert_no_secret("a refusal", &last_answer().1);
    while let Some(event) = poll(handle) {
        assert_no_secret("an event", &event.to_string());
    }
    unsafe { lingara_embed_free(handle) };

    // An Http source's header value in a malformed config.
    let config = r#"{"token_source": {"type": "http", "url": "https://game.example/t", "headers": [{"name": "cookie", "value": "hunter2"}]}, "queue_capacity": "many"}"#;
    let mut h = std::ptr::null_mut();
    assert_eq!(unsafe { lingara_embed_new(1, config.as_ptr(), config.len(), &mut h) }, LINGARA_EMBED_ERR_INVALID_ARGUMENT);
    assert!(h.is_null());
    assert_no_secret("a config error", &last_error());
    assert_eq!(crate::redact::redact("a lgr_et_abc-123 b lgr_x"), "a lgr_et_[REDACTED] b lgr_x");
}

// 1.10.26aa AC6
#[test]
fn an_answer_that_does_not_fit_waits_and_the_command_runs_once() {
    on_fresh_thread(|| {
        assert_eq!(last_answer().0, LINGARA_EMBED_EMPTY, "no answer on a thread that made no call");
        assert_eq!(call(64, |b, c, l| unsafe { lingara_embed_last_error(b, c, l) }).0, LINGARA_EMBED_EMPTY);

        let handle = open(OFFLINE);
        let (status, needed, _) = call(1, |b, c, l| unsafe { lingara_embed_execute(handle, DIALOGUE.as_ptr(), DIALOGUE.len(), b, c, l) });
        assert_eq!(status, LINGARA_EMBED_BUFFER_TOO_SMALL);
        let (status, answer) = last_answer();
        assert_eq!(status, LINGARA_EMBED_OK);
        assert_eq!(answer.len(), needed);
        let first = json(&answer);
        assert_eq!(first["type"], "id");
        let n = first["id"].as_u64().unwrap();
        let (_, again) = exec(handle, DIALOGUE);
        assert_eq!(json(&again)["id"].as_u64(), Some(n + 1), "the first dialogue_open ran exactly once");
        unsafe { lingara_embed_free(handle) };
    });
}

// 1.10.26aa AC7
#[test]
fn the_cores_refusal_is_an_embed_error_answer() {
    on_fresh_thread(|| {
        let handle = open(OFFLINE);
        let (status, text) = exec(handle, r#"{"type": "practice_answer", "session": 999, "answer": "x"}"#);
        assert_eq!(status, LINGARA_EMBED_REFUSED);
        let error = json(&text);
        assert_eq!((error["kind"].as_str(), error["retryable"].as_bool()), (Some("unknown_id"), Some(false)));
        assert_eq!(call(64, |b, c, l| unsafe { lingara_embed_last_error(b, c, l) }).0, LINGARA_EMBED_EMPTY, "a refusal is no fault");
        unsafe { lingara_embed_free(handle) };

        let config = r#"{"queue_capacity": 0}"#;
        let mut h = std::ptr::null_mut();
        assert_eq!(unsafe { lingara_embed_new(1, config.as_ptr(), config.len(), &mut h) }, LINGARA_EMBED_REFUSED);
        assert!(h.is_null(), "no handle is made");
        let (status, answer) = last_answer();
        assert_eq!(status, LINGARA_EMBED_REFUSED);
        assert_eq!(json(&answer)["kind"], "config");
    });
    std::thread::sleep(Duration::from_millis(1));
}
