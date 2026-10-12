//! The mirrors against K1 (ADR 1.10.26ae D2, D9): the JSON-shape tripwire,
//! the command round trips and range refusals, and the unknown fallback.

use lingara_embed as core;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::convert::{from_core, to_core, unknown_from_json};
use crate::types::*;
use crate::{command_from_json, event_to_json};

fn vector() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance/vectors/json-shape.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("json-shape.json")).expect("JSON")
}

fn examples(vector: &Value, group: &str, name: &str) -> Vec<Value> {
    vector[group][name].as_array().unwrap_or_else(|| panic!("{group}.{name} in the vector")).clone()
}

/// Each example of `name` into K1's type, then into its mirror: never a
/// failure, so never `Unknown`. Returns the mirrors.
fn mirrored<K: DeserializeOwned + Serialize, M: DeserializeOwned>(vector: &Value, group: &str, name: &str) -> Vec<M> {
    let all = examples(vector, group, name);
    assert!(!all.is_empty(), "{name} has examples");
    all.into_iter()
        .map(|example| {
            let k1: K = serde_json::from_value(example.clone()).unwrap_or_else(|e| panic!("K1 reads {example}: {e}"));
            from_core::<K, M>(&k1).unwrap_or_else(|json| panic!("{name} has no mirror for {json}"))
        })
        .collect()
}

// 1.10.26ae AC1: every example of every K1 type in the shape vector lands in
// a mirror, and each event's `event_to_json` is K1's own JSON for it.
#[test]
fn every_k1_variant_has_a_mirror() {
    let v = vector();
    mirrored::<core::Command, Command>(&v, "tagged", "Command");
    mirrored::<core::CommandOutcome, CommandOutcome>(&v, "tagged", "CommandOutcome");
    mirrored::<core::StreamState, StreamState>(&v, "tagged", "StreamState");
    mirrored::<core::NpcTurnUpdate, NpcTurnUpdate>(&v, "tagged", "NpcTurnUpdate");
    mirrored::<core::PracticeSource, PracticeSource>(&v, "tagged", "PracticeSource");
    mirrored::<core::TokenSourceConfig, TokenSourceConfig>(&v, "tagged", "TokenSourceConfig");
    mirrored::<core::LingaraEventDetail, LingaraEventDetail>(&v, "tagged", "LingaraEventDetail");
    mirrored::<core::TokenReason, TokenReason>(&v, "words", "TokenReason");
    mirrored::<core::ErrorKind, ErrorKind>(&v, "words", "ErrorKind");
    mirrored::<core::LogLevel, LogLevel>(&v, "words", "LogLevel");
    mirrored::<core::CloseReason, CloseReason>(&v, "words", "CloseReason");
    mirrored::<core::PracticePhase, PracticePhase>(&v, "words", "PracticePhase");
    mirrored::<core::PracticeOutcome, PracticeOutcome>(&v, "words", "PracticeOutcome");
    mirrored::<core::DialoguePhase, DialoguePhase>(&v, "words", "DialoguePhase");
    mirrored::<core::Speaker, Speaker>(&v, "words", "Speaker");
    for example in examples(&v, "tagged", "EmbedEvent") {
        let k1: core::EmbedEvent = serde_json::from_value(example).expect("K1 reads the event");
        let mirror = crate::convert::event(&k1);
        assert!(!matches!(mirror, EmbedEvent::Unknown { .. }), "no mirror for {k1:?}");
        let written: Value = serde_json::from_str(&event_to_json(mirror)).expect("event_to_json is JSON");
        assert_eq!(written, serde_json::to_value(&k1).expect("K1 serialises"), "event_to_json is K1's shape");
    }
}

/// `serde_json::to_value` of a K1 value: its JSON, secrets redacted.
fn k1_json<K: Serialize>(value: &K) -> Value {
    serde_json::to_value(value).expect("K1 serialises")
}

// 1.10.26ae AC2: every command goes mirror → K1 unchanged, and back to the
// same mirror. The way back passes through K1's `Serialize`, which redacts
// by design, so a secret-bearing command comes back with `[REDACTED]` in
// its secret and nothing else changed.
#[test]
fn commands_round_trip() {
    let v = vector();
    for example in examples(&v, "tagged", "Command") {
        let mirror = command_from_json(example.to_string()).expect("the mirror reads it");
        let direct: core::Command = serde_json::from_value(example.clone()).expect("K1 reads it");
        let through: core::Command = to_core("command", &mirror).expect("into K1");
        assert_eq!(through, direct, "mirror → K1 keeps {example}");
        let back: Command = from_core(&through).expect("back into the mirror");
        let redacted: Command = serde_json::from_value(k1_json(&direct)).expect("redacted mirror");
        assert_eq!(back, redacted, "K1 → mirror keeps {example}");
    }
}

fn refused(command: Command) {
    let err = to_core::<_, core::Command>("command", &command).expect_err("out of range");
    let EmbedException::Failed { error } = err;
    assert_eq!(error.kind, ErrorKind::InvalidInput, "{command:?}");
    assert!(!error.retryable);
}

// 1.10.26ae AC2: an id or count outside K1's unsigned range is refused with
// `invalid_input`, never cast and never a panic.
#[test]
fn out_of_range_values_are_refused() {
    refused(Command::PracticeAnswer { session: -1, answer: "a".into() });
    refused(Command::PracticeChoose { session: 3, option: i64::from(u32::MAX) + 1 });
    refused(Command::ProvideTokenFor { token: String::from("lgr_et_x").into(), expires_in_secs: -5 });
    refused(Command::DialogueSetLevel { dialogue: 4, level: 300 });
    refused(Command::DialogueClose { dialogue: i64::MIN });
    let narrowed = crate::convert::narrow::<u8>("level", 256i32).expect_err("out of range");
    let EmbedException::Failed { error } = narrowed;
    assert!(error.message.contains("level"), "{}", error.message);
}

// 1.10.26ae AC16: a variant of a newer core becomes `Unknown` with its tag
// and its JSON intact; a value with no `type` becomes `Unknown` with an
// empty tag. Neither panics.
#[test]
fn an_unknown_variant_becomes_unknown() {
    let future = json!({ "type": "future_thing", "weather": "rain", "n": 3 });
    match unknown_from_json(future.clone()) {
        EmbedEvent::Unknown { type_tag, json } => {
            assert_eq!(type_tag, "future_thing");
            assert_eq!(serde_json::from_str::<Value>(&json).expect("JSON"), future);
            assert_eq!(event_to_json(EmbedEvent::Unknown { type_tag, json: json.clone() }), json);
        }
        other => panic!("not Unknown: {other:?}"),
    }
    match unknown_from_json(json!({ "weather": "rain" })) {
        EmbedEvent::Unknown { type_tag, .. } => assert_eq!(type_tag, ""),
        other => panic!("not Unknown: {other:?}"),
    }
    assert!(from_core::<Value, EmbedEvent>(&future).is_err(), "the mirror refuses an unknown tag");
}

// 1.10.26ae D2: a command that is not K1's shape is refused by K1's own
// deserialiser, before the mirror reads it.
#[test]
fn command_from_json_refuses_what_k1_refuses() {
    for bad in [r#"{"type":"no_such_command"}"#, r#"{"type":"practice_answer","session":3}"#, "not json"] {
        let EmbedException::Failed { error } = command_from_json(bad.into()).expect_err(bad);
        assert_eq!(error.kind, ErrorKind::InvalidInput, "{bad}");
    }
}

// 1.10.26ae D2: the config's null fields take K1's defaults, and a set one
// reaches K1 as written.
#[test]
fn a_default_config_is_k1s_default() {
    let k1: core::EmbedConfig = to_core("config", &EmbedConfig::default()).expect("into K1");
    assert_eq!(k1, core::EmbedConfig::default());
    let set = EmbedConfig { queue_capacity: Some(8), log_level: Some(LogLevel::Debug), ..EmbedConfig::default() };
    let k1: core::EmbedConfig = to_core("config", &set).expect("into K1");
    assert_eq!((k1.queue_capacity, k1.log_level), (8, core::LogLevel::Debug));
    let negative = EmbedConfig { queue_capacity: Some(-1), ..EmbedConfig::default() };
    assert!(to_core::<_, core::EmbedConfig>("config", &negative).is_err());
}

// 1.10.26ae D2: the version symbol is the crate version, NUL-terminated.
#[test]
fn the_version_symbol_is_the_crate_version() {
    // SAFETY: the symbol returns a pointer to a static NUL-terminated string.
    let version = unsafe { std::ffi::CStr::from_ptr(crate::lingara_embed_uniffi_version()) };
    assert_eq!(version.to_str().expect("UTF-8"), env!("CARGO_PKG_VERSION"));
}
