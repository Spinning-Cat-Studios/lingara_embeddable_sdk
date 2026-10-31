//! Events to signals, methods to K1 commands, refusals to sentinels, and
//! no secret anywhere (ADR 1.10.26ab D3, D4, D10). K1's own refusals come
//! from a real `Embed` on a closed loopback port, Poll-sourced and with no
//! stream, so nothing leaves the machine.

use std::sync::Arc;

use lingara_embed::{Command, Embed, EmbedEvent, EmbedError, PracticePhase};
use serde_json::{Value, json};

use super::fake_core::{FakeCore, snapshot};
use crate::binding::{Binding, Returns};
use crate::commands::build;
use crate::config::Props;
use crate::convert::{Finished, SignalCall, signals_for};
use crate::pump::{Core, Library};

fn vector() -> Value {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../conformance/vectors/json-shape.json")).expect("json-shape.json");
    serde_json::from_str(&text).expect("the vector parses")
}

fn examples(kind: &str) -> Vec<Value> {
    vector()["tagged"][kind].as_array().cloned().expect(kind)
}

/// D3's typed signal per event `type`, and the JSON type of each argument.
fn expected(event: &Value) -> (&'static str, Vec<&'static str>) {
    match event["type"].as_str().unwrap_or_default() {
        "token_needed" => ("token_needed", vec!["int", "str"]),
        "stream" => ("stream_changed", vec!["obj", "str"]),
        "lingara" => ("event_received", vec!["str", "str", "str", "obj"]),
        "world_sent" => ("world_sent", vec!["int", "str", "obj"]),
        "practice" => ("practice_changed", vec!["int", "obj"]),
        "error" => ("error_raised", vec!["obj"]),
        "log" => ("log_received", vec!["str", "str", "str"]),
        "closed" => ("closed", vec!["str"]),
        "checkpoint_reached" => ("checkpoint_reached", vec!["int", "str", "str", "str"]),
        "dialogue_ended" => ("dialogue_ended", vec!["int", "str", "str", "str"]),
        // 28.10.26i AC8
        "dialogue_loaded" => ("dialogue_loaded", vec!["int", "str", "int"]),
        "dialogue_load_failed" => ("dialogue_load_failed", vec!["int", "obj"]),
        // 30.10.26p AC7: (latest, deprecated, message).
        "update_available" => ("update_available", vec!["str", "bool", "str"]),
        _ => match event["update"]["type"].as_str().unwrap_or_default() {
            "opened" => ("npc_turn_opened", vec!["int", "int"]),
            "fragment" => ("npc_fragment", vec!["int", "str"]),
            "notice" => ("npc_notice", vec!["int", "str", "str"]),
            "finished" => ("npc_replied", vec!["int", "str"]),
            _ => ("npc_turn_cancelled", vec!["int"]),
        },
    }
}

fn kind_of(value: &Value) -> &'static str {
    match value {
        Value::Number(n) if n.is_i64() || n.is_u64() => "int",
        Value::String(_) => "str",
        Value::Bool(_) => "bool",
        Value::Object(_) => "obj",
        _ => "other",
    }
}

/// Every event in the vector, plus one `npc_turn` per update in it.
fn every_event() -> Vec<Value> {
    let mut events = examples("EmbedEvent");
    let turns = examples("NpcTurnUpdate").into_iter().map(|update| json!({ "type": "npc_turn", "dialogue": 4, "request": 5, "update": update }));
    events.extend(turns);
    events
}

fn practice(session: u64, phase: PracticePhase, retryable: Option<bool>) -> EmbedEvent {
    let mut snapshot = snapshot(phase);
    snapshot.failure = retryable.map(|r| EmbedError { retryable: r, ..EmbedError::new(lingara_embed::ErrorKind::Unavailable, "report failed") });
    EmbedEvent::Practice { session, snapshot }
}

fn names(calls: &[SignalCall]) -> Vec<&'static str> {
    calls.iter().map(|c| c.name).collect()
}

/// 1.10.26ab AC1: every `EmbedEvent` variant in `json-shape.json` maps to
/// `core_event` with the whole event, then its D3 signal with D3's arguments
/// in order; `NpcTurn`'s five updates map to five signals; an unknown
/// Lingara event type keeps its raw `data`; `practice_finished` fires once
/// per session, on D3's terminal rule.
#[test]
fn every_core_event_maps_to_its_signals() {
    let events = every_event();
    let mut turn_signals = std::collections::BTreeSet::new();
    for raw in &events {
        let event: EmbedEvent = serde_json::from_value(raw.clone()).expect("a K1 event");
        let calls = signals_for(&event, &mut Finished::default());
        assert_eq!(calls[0], SignalCall::new("core_event", vec![serde_json::to_value(&event).unwrap()]), "{raw}");
        let (name, kinds) = expected(raw);
        assert_eq!(calls[1].name, name, "{raw}");
        assert_eq!(calls[1].args.iter().map(kind_of).collect::<Vec<_>>(), kinds, "{raw}");
        if raw["type"] == "npc_turn" {
            turn_signals.insert(name);
        }
    }
    assert_eq!(turn_signals.len(), 5, "opened, fragment, notice, finished, cancelled");

    let lingara = examples("EmbedEvent").into_iter().find(|e| e["type"] == "lingara").unwrap();
    let event: EmbedEvent = serde_json::from_value(lingara).unwrap();
    assert_eq!(signals_for(&event, &mut Finished::default())[1].args[3], json!({ "n": 1 }), "an unknown type keeps its raw data");

    let mut finished = Finished::default();
    let walk = [practice(3, PracticePhase::Unanswered, None), practice(3, PracticePhase::ReportFailed, Some(true)), practice(3, PracticePhase::Reported, None), practice(3, PracticePhase::Ended, None)];
    let fired: Vec<_> = walk.iter().map(|e| names(&signals_for(e, &mut finished))).collect();
    assert_eq!(fired[1], ["core_event", "practice_changed"], "a retryable report_failed is not terminal");
    assert_eq!(fired[2], ["core_event", "practice_changed", "practice_finished"]);
    assert_eq!(fired[3], ["core_event", "practice_changed"], "once per session");
    for phase in [PracticePhase::Ended, PracticePhase::Failed] {
        assert!(names(&signals_for(&practice(9, phase, None), &mut Finished::default())).contains(&"practice_finished"));
    }
    assert!(names(&signals_for(&practice(9, PracticePhase::ReportFailed, Some(false)), &mut finished)).contains(&"practice_finished"));
}

/// A binding over a real K1 handle that cannot reach anything.
pub fn real_binding() -> Binding {
    let start = Box::new(|c| Embed::start(c).map(|e| Box::new(e) as Box<dyn Core>));
    let mut binding = Binding::new(Arc::new(Library::default()), start);
    let props = Props { base_url: "http://127.0.0.1:9".into(), stream_on_start: false, ..Props::default() };
    let source = binding.tokens.source("");
    assert_eq!(binding.start(props.config(source)), Some(false));
    binding.take_signals();
    binding
}

/// A refusal: the sentinel, `error_raised` with the error, and `last_error`.
fn refused(binding: &mut Binding, kind: &str, fields: Vec<(&str, Value)>, returns: Returns) -> Value {
    let answer = binding.call(build(kind, fields), returns);
    assert_eq!(answer, crate::binding::sentinel(returns), "{kind} returns D3's sentinel");
    let signals = binding.take_signals();
    assert_eq!(names(&signals), ["error_raised"], "{kind} emits error_raised before it returns");
    assert_eq!(signals[0].args[0], binding.last_error(), "{kind}'s error is readable from last_error");
    binding.last_error()
}

fn mei(level: u8) -> Value {
    json!({ "npc": { "name": "Mei" }, "source_lang": "en", "target_lang": "zh", "level": level })
}

/// 1.10.26ab AC2: every flat method builds the `Command` K1's flat method
/// would; K1's refusals and failed narrowings return D3's sentinel, emit
/// `error_raised` before returning and are readable from `last_error`; a
/// refused `execute` does the same beside its `{"error": …}`; a practice
/// method's snapshot is both returned and emitted.
#[test]
fn methods_build_k1_commands_and_refusals_surface() {
    for example in examples("Command") {
        let mut fields: Vec<(&str, Value)> = Vec::new();
        let object = example.as_object().unwrap();
        for (key, value) in object.iter().filter(|(k, _)| *k != "type") {
            fields.push((key.as_str(), value.clone()));
        }
        let want: Command = serde_json::from_value(example.clone()).unwrap();
        assert_eq!(build(example["type"].as_str().unwrap(), fields).unwrap(), want);
    }

    let mut b = real_binding();
    let long = json!({ "npc": { "name": "M".repeat(40) }, "source_lang": "en", "target_lang": "zh", "level": 2 });
    assert_eq!(refused(&mut b, "dialogue_open", vec![("config", long)], Returns::Id)["kind"], "invalid_input");
    assert_eq!(refused(&mut b, "provide_token", vec![("token", "lgr_at_x".into()), ("expires_at", "2030-01-01T00:00:00Z".into())], Returns::Bool)["kind"], "invalid_input");
    let negative = refused(&mut b, "dialogue_say", vec![("dialogue", (-1).into()), ("line", "hi".into())], Returns::Id);
    assert!(negative["kind"] == "invalid_input" && negative["message"].as_str().unwrap().starts_with("dialogue"), "{negative}");
    let level = refused(&mut b, "dialogue_set_level", vec![("dialogue", 1.into()), ("level", 300.into())], Returns::Bool);
    assert!(level["message"].as_str().unwrap().starts_with("level"), "{level}");

    let id = b.call(build("dialogue_open", vec![("config", mei(2))]), Returns::Id);
    assert!(id.as_i64().unwrap() > 0 && b.last_error() == json!({}));
    let say = || vec![("dialogue", id.clone()), ("line", Value::from("你好"))];
    assert!(b.call(build("dialogue_say", say()), Returns::Id).as_i64().unwrap() > 0, "the turn waits for a token");
    assert_eq!(refused(&mut b, "dialogue_say", say(), Returns::Id)["kind"], "busy");

    let answer = b.execute(json!({ "type": "dialogue_set_level", "dialogue": id, "level": 0 }));
    assert_eq!(answer["error"], b.last_error());
    assert_eq!(names(&b.take_signals()), ["error_raised"]);
    assert_eq!(b.execute(json!({ "type": "dialogue_snapshot", "dialogue": id }))["ok"]["type"], "dialogue");
    b.stop();

    let fake = FakeCore::default();
    fake.state().phase = Some(PracticePhase::Reported);
    let mut f = Binding::new(Arc::new(Library::default()), Box::new(move |_| Ok(Box::new(fake.clone()) as Box<dyn Core>)));
    f.start(Props::default().config(json!({ "type": "poll" })));
    f.take_signals();
    let snap = f.call(build("practice_next", vec![("session", 7.into())]), Returns::Practice(Some(7)));
    let signals = f.take_signals();
    assert_eq!(names(&signals), ["practice_changed", "practice_finished"]);
    assert_eq!(signals[0].args, vec![Value::from(7), snap.clone()], "returned and emitted");
    assert_eq!(snap["phase"], "reported");
}

/// 28.10.26f D8: the graph's three methods build K1's commands and answer
/// as K1 does; its two events reach their signals with `""` for no edge.
#[test]
fn graph_methods_and_events() {
    let ir = r#"{"schema":"dialogue.v1","id":"g","npcs":[{"id":"elena","name":"Elena"}],"start":"intro","checkpoints":[{"id":"intro","goal":"Wary"}]}"#;
    let mut b = real_binding();
    assert_eq!(b.call(build("dialogue_validate", vec![("ir_json", ir.into())]), Returns::Bool), json!(true));
    assert_eq!(refused(&mut b, "dialogue_validate", vec![("ir_json", "{}".into())], Returns::Bool)["kind"], "invalid_input");
    let source = json!({ "type": "ir", "ir_json": ir });
    let session = json!({ "source_lang": "en", "target_lang": "es", "level": 2 });
    let id = b.call(build("dialogue_load", vec![("source", source), ("session", session)]), Returns::Id);
    assert!(id.as_i64().unwrap() > 0, "{id}");
    let fact = vec![("dialogue", id.clone()), ("key", "trust".into()), ("value", 0.4.into())];
    assert_eq!(refused(&mut b, "dialogue_set_fact", fact, Returns::Bool)["kind"], "invalid_input", "an undeclared fact");
    b.stop();

    let start = EmbedEvent::CheckpointReached { dialogue: 4, checkpoint: "intro".into(), edge: None, via: lingara_embed::CrossedVia::Start };
    let calls = signals_for(&start, &mut Finished::default());
    assert_eq!(calls[1], SignalCall::new("checkpoint_reached", vec![4.into(), "intro".into(), "".into(), "start".into()]));
    let ended = EmbedEvent::DialogueEnded { dialogue: 4, outcome: lingara_embed::DialogueOutcome::Abandoned, checkpoint: "intro".into(), edge: None };
    let calls = signals_for(&ended, &mut Finished::default());
    assert_eq!(calls[1], SignalCall::new("dialogue_ended", vec![4.into(), "abandoned".into(), "intro".into(), "".into()]));
}

fn leaks(value: &Value, secret: &str) -> bool {
    value.to_string().contains(secret)
}

/// 1.10.26ab AC13: no token or header value reaches a signal argument,
/// `execute`'s answer, `last_error` or a log line; the token commands echo
/// `[REDACTED]`; an `extra_config` holding `token_source` is refused with
/// `config`.
#[test]
fn no_secret_reaches_a_signal_or_an_answer() {
    let secret = "SeCrEtVaLuE123";
    let token = format!("lgr_et_{secret}");
    let commands = [
        build("provide_token", vec![("token", token.clone().into()), ("expires_at", "2030-01-01T00:00:00Z".into())]),
        build("provide_token_for", vec![("token", token.clone().into()), ("expires_in_secs", 900.into())]),
        build("set_token_endpoint", vec![("url", "https://game.example/t".into()), ("headers", json!([{ "name": "cookie", "value": secret }]))]),
    ];
    for command in commands {
        let echoed = serde_json::to_value(command.unwrap()).unwrap();
        assert!(!leaks(&echoed, secret) && leaks(&echoed, lingara_embed::REDACTED), "{echoed}");
    }

    let mut b = real_binding();
    let mut seen = Vec::new();
    let at = format!("lgr_at_{secret}");
    seen.push(b.call(build("provide_token", vec![("token", at.into()), ("expires_at", "2030-01-01T00:00:00Z".into())]), Returns::Bool));
    seen.push(b.last_error());
    assert!(b.set_token_endpoint("https://game.example/t", json!({ "cookie": secret })));
    seen.push(b.execute(json!({ "type": "provide_token", "token": token, "expires_at": "not a time" })));
    seen.push(b.last_error());
    seen.push(b.execute(json!({ "type": "set_token_endpoint", "url": "https://game.example/t", "headers": [{ "name": "c", "value": secret }] })));
    b.frame(256);
    seen.extend(b.take_signals().into_iter().flat_map(|c| c.args));
    seen.push(b.status());
    assert!(seen.iter().all(|v| !leaks(v, secret)), "{seen:?}");
    b.stop();

    let props = Props { extra_config: json!({ "token_source": { "type": "poll" } }), ..Props::default() };
    assert_eq!(props.config(json!({ "type": "poll" })).unwrap_err().kind, lingara_embed::ErrorKind::Config);
    let owned = Props { extra_config: json!({ "base_url": "https://x.example" }), ..Props::default() };
    assert_eq!(owned.config(json!({ "type": "poll" })).unwrap_err().kind, lingara_embed::ErrorKind::Config);
}

/// D1's kit token, then the game's suffix; D2's `extra_config` fills only
/// the fields no property owns; D4's endpoint before start is the source.
#[test]
fn config_names_the_kit_and_tokens_pick_their_source() {
    let props = Props { user_agent_suffix: "my-game/1.4".into(), extra_config: json!({ "token_wait_timeout_ms": 2000 }), ..Props::default() };
    let config = props.config(json!({ "type": "poll" })).unwrap();
    assert_eq!(config.user_agent_suffix.as_deref(), Some(concat!("lingara-godot/", env!("CARGO_PKG_VERSION"), " my-game/1.4")));
    assert_eq!(config.token_wait_timeout_ms, 2000);
    assert_eq!(config.api_version, None, "an empty api_version is no pin");
    // 30.10.26p D4: the embed_version property, empty being none.
    assert_eq!(config.embed_version, None);
    let declared = Props { embed_version: "1.1.0".into(), ..Props::default() }.config(json!({ "type": "poll" })).unwrap();
    assert_eq!(declared.embed_version.as_deref(), Some("1.1.0"));

    let mut tokens = crate::token::Tokens::default();
    assert_eq!(tokens.source(""), json!({ "type": "poll" }));
    assert_eq!(tokens.source("https://game.example/t")["type"], "http");
    assert!(matches!(tokens.set_endpoint(false, "https://game.example/s", json!({ "cookie": "x" })), Ok(crate::token::Endpoint::Kept)));
    assert_eq!(tokens.source("https://game.example/t")["url"], "https://game.example/s", "the kept endpoint wins");
}
