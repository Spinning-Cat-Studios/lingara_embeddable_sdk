//! A graph dialogue over the real library and `fake_lingara` (ADR 28.10.26f
//! D5, D6): what a turn sends, when a `transition` is applied, and the three
//! commands' rules.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::embed::Embed;
use crate::error::ErrorKind;
use crate::tests::fake_lingara::{After, FakeLingara, Reply, frame, graphs_dark, transition};
use crate::tests::{SECS, config, started, wait_for};
use crate::types::{
    Command, CrossedVia, DialogueId, DialogueOutcome, DialoguePhase, DialogueSource, EmbedEvent, FactValue, GraphSession, LogLevel, NpcTurnUpdate,
};

const TURNS: &str = "/v1/embed/dialogue/turns";

/// Two keepers; Elena speaks Spanish natively and little English.
fn keepers() -> Value {
    json!({
        "schema": "dialogue.v1", "id": "missing_brother", "pedagogy": "none",
        "npcs": [
            { "id": "elena", "name": "Elena", "persona": "a farmer", "languages": [{ "code": "es", "speaking": 9, "comprehension": 9 }, { "code": "en", "speaking": 2, "comprehension": 3 }] },
            { "id": "tomas", "name": "Tomas" }
        ],
        "facts": [
            { "key": "trust", "type": "number", "default": 0 }, { "key": "alarm", "type": "boolean", "default": false },
            { "key": "gate", "type": "string" }, { "key": "note", "type": "string", "default": "  " }, { "key": "secret", "type": "string", "default": "kept" }
        ],
        "start": "intro",
        "checkpoints": [
            { "id": "intro", "goal": "Elena is wary", "may_disclose": ["gate", "note", "trust"], "edges": [
                { "id": "alarm", "to": "failure", "predicate": "alarm" },
                { "id": "explained", "to": "ask", "predicate": "trust >= 0.4", "when": "the player explains" },
                { "id": "doubted", "to": "ask", "predicate": "trust < 0.4", "when": "the player is vague" },
                { "id": "hello", "to": "ask", "when": "the player greets her" }] },
            { "id": "hall", "goal": "Walk on", "edges": [{ "id": "on", "to": "ask" }] },
            { "id": "ask", "goal": "Admit she saw soldiers", "edges": [{ "id": "gate", "to": "success", "when": "the player asks where" }] }
        ]
    })
}

fn session(npc: Option<&str>) -> GraphSession {
    GraphSession { npc: npc.map(Into::into), source_lang: "en".into(), target_lang: "es".into(), level: 2, scene: None, facts: BTreeMap::new(), checkpoint: None, window: vec![] }
}

fn ir() -> DialogueSource {
    DialogueSource::Ir { ir_json: keepers().to_string() }
}

fn number(n: f64) -> FactValue {
    FactValue::Number(serde_json::Number::from_f64(n).unwrap())
}

fn text(s: &str) -> FactValue {
    FactValue::String(s.into())
}

fn done_with(mut frames: Vec<String>) -> Reply {
    frames.push(frame("delta", json!({ "text": "Hola" })));
    frames.push(frame("done", json!({})));
    Reply::Sse(frames, After::End)
}

fn checkpoint(embed: &Embed, d: DialogueId) -> String {
    embed.dialogue_snapshot(d).unwrap().graph.unwrap().checkpoint
}

fn finished(e: &EmbedEvent) -> bool {
    matches!(e, EmbedEvent::NpcTurn { update: NpcTurnUpdate::Finished { .. }, .. })
}

fn judged(e: &EmbedEvent) -> bool {
    matches!(e, EmbedEvent::CheckpointReached { via: CrossedVia::Judge, .. })
}

fn warned(events: &[EmbedEvent]) -> bool {
    events.iter().any(|e| matches!(e, EmbedEvent::Log { level: LogLevel::Warn, .. }))
}

// 28.10.26f AC9
#[test]
fn a_graph_turn_sends_the_story_and_nothing_private() {
    let fake = FakeLingara::start(|_, _| done_with(vec![]));
    let embed = started(config(&fake.url));
    let mut s = session(Some("elena"));
    s.facts = [("gate".to_owned(), text("the eastern gate")), ("trust".to_owned(), number(0.5)), ("secret".to_owned(), text("never sent"))].into();
    let d = embed.dialogue_load(ir(), s).unwrap();
    embed.dialogue_say(d, "hola").unwrap();
    wait_for(&embed, 5 * SECS, finished);

    // What the server received, after the library decoded and re-sent it.
    let body = &fake.seen()[0].body;
    assert_eq!(body["npc"]["languages"], json!([{ "code": "es", "speaking": 9, "comprehension": 9 }, { "code": "en", "speaking": 2, "comprehension": 3 }]));
    assert_eq!(body["pedagogy"], "none", "the IR's pedagogy");
    assert_eq!(body["checkpoint"], json!({ "id": "intro", "goal": "Elena is wary", "may_disclose": ["gate", "note", "trust"] }));
    let edges = json!([{ "id": "explained", "to": "ask", "when": "the player explains" }, { "id": "hello", "to": "ask", "when": "the player greets her" }]);
    assert_eq!(body["edges"], edges, "the offered edges in IR order, never a predicate");
    let facts = body["facts"].as_object().unwrap();
    // Sorted: an object's key order is not the wire's, and under
    // `cargo test --workspace` the conformance server's serde_json
    // `preserve_order` reaches this crate, so the order is the library's.
    let mut keys: Vec<_> = facts.keys().collect();
    keys.sort();
    assert_eq!(keys, ["gate", "trust"], "only may_disclose, and not the blank `note` default");
    assert_eq!((facts["gate"].as_str(), facts["trust"].as_f64()), (Some("the eastern gate"), Some(0.5)));
    assert_eq!(body["level"], 2);
}

fn load(embed: &Embed) -> DialogueId {
    embed.dialogue_load(ir(), session(Some("elena"))).unwrap()
}

/// Turn 0 crosses after `done`; turn 1 errors after its transition.
fn applied_after_done(embed: &Embed) {
    let d = load(embed);
    embed.dialogue_say(d, "hola").unwrap();
    let (event, before) = wait_for(embed, 5 * SECS, judged);
    assert!(matches!(event, EmbedEvent::CheckpointReached { checkpoint, edge: Some(edge), .. } if checkpoint == "ask" && edge == "hello"));
    assert!(before.iter().any(finished), "Finished comes first");

    let d = load(embed);
    embed.dialogue_say(d, "hola").unwrap();
    wait_for(embed, 5 * SECS, |e| matches!(e, EmbedEvent::Error { .. }));
    let snapshot = embed.dialogue_snapshot(d).unwrap();
    assert_eq!((snapshot.graph.unwrap().checkpoint.as_str(), snapshot.window.len()), ("intro", 0), "an error after a transition keeps both");
}

// 28.10.26f AC10
#[test]
fn a_transition_is_applied_only_after_done() {
    let hello = || transition("hello", "ask");
    let unavailable = frame("notice", json!({ "code": "transition_unavailable", "message": "the judge could not run" }));
    let fake = FakeLingara::start(move |_, i| match i {
        0 => done_with(vec![hello()]),
        1 => Reply::Sse(vec![hello(), frame("error", json!({ "code": "stream_interrupted", "message": "provider" }))], After::End),
        2 => Reply::Sse(vec![hello(), frame("delta", json!({ "text": "Ho" }))], After::Hold),
        3 => done_with(vec![hello(), transition("doubted", "ask")]),
        _ => done_with(vec![unavailable.clone()]),
    });
    let embed = started(config(&fake.url));
    applied_after_done(&embed);

    let d = load(&embed);
    embed.dialogue_say(d, "hola").unwrap();
    wait_for(&embed, 5 * SECS, |e| matches!(e, EmbedEvent::NpcTurn { update: NpcTurnUpdate::Fragment { .. }, .. }));
    embed.dialogue_cancel(d).unwrap();
    assert_eq!(checkpoint(&embed, d), "intro", "a cancel drops the held transition");

    let d = load(&embed);
    embed.dialogue_say(d, "hola").unwrap();
    let (event, before) = wait_for(&embed, 5 * SECS, judged);
    assert!(matches!(event, EmbedEvent::CheckpointReached { edge: Some(edge), .. } if edge == "hello"), "the first transition is kept");
    assert!(warned(&before), "the second is logged");

    let d = load(&embed);
    embed.dialogue_say(d, "hola").unwrap();
    let (_, before) = wait_for(&embed, 5 * SECS, finished);
    assert!(before.iter().any(|e| matches!(e, EmbedEvent::NpcTurn { update: NpcTurnUpdate::Notice { code, .. }, .. } if code == "transition_unavailable")));
    assert_eq!(checkpoint(&embed, d), "intro", "no transition is a stay");
    assert_eq!(fake.count("POST", TURNS), 5);
}

/// Cast, resume, and seeded-fact rules at load.
fn load_rules(embed: &Embed) {
    assert_eq!(embed.dialogue_load(ir(), session(None)).unwrap_err().kind, ErrorKind::InvalidInput, "two in the cast: name one");
    for bad in ["success", "nowhere"] {
        let s = GraphSession { checkpoint: Some(bad.into()), ..session(Some("tomas")) };
        assert_eq!(embed.dialogue_load(ir(), s).unwrap_err().kind, ErrorKind::InvalidInput, "{bad}");
    }
    let s = GraphSession { facts: [("trust".to_owned(), text("high"))].into(), ..session(Some("tomas")) };
    assert_eq!(embed.dialogue_load(ir(), s).unwrap_err().kind, ErrorKind::InvalidInput, "a seeded fact is checked");

    let s = GraphSession { checkpoint: Some("hall".into()), ..session(Some("tomas")) };
    let d = embed.dialogue_load(ir(), s).unwrap();
    let (_, before) = wait_for(embed, 5 * SECS, |e| matches!(e, EmbedEvent::CheckpointReached { via: CrossedVia::Auto, .. }));
    assert!(before.iter().any(|e| matches!(e, EmbedEvent::CheckpointReached { checkpoint, via: CrossedVia::Start, .. } if checkpoint == "hall")));
    assert_eq!(checkpoint(embed, d), "ask", "a resumed run auto-crosses as a fresh one does");
}

/// A snapshot fed back restores the story; close ends an unfinished one.
fn restore_and_close(embed: &Embed) {
    let d = load(embed);
    embed.dialogue_set_fact(d, "gate", text("north")).unwrap();
    embed.dialogue_say(d, "hola").unwrap();
    wait_for(embed, 5 * SECS, finished);
    let saved = embed.dialogue_snapshot(d).unwrap();
    let graph = saved.graph.clone().unwrap();
    assert_eq!(graph.facts, [("gate".to_owned(), text("north"))].into(), "set values only, never defaults");
    let s = GraphSession { npc: Some(graph.npc.clone()), facts: graph.facts.clone(), checkpoint: Some(graph.checkpoint.clone()), window: saved.window.clone(), ..session(None) };
    let restored = embed.dialogue_snapshot(embed.dialogue_load(ir(), s).unwrap()).unwrap();
    assert_eq!((restored.graph, restored.window), (Some(graph), saved.window));

    embed.dialogue_close(d).unwrap();
    let abandoned = |e: &EmbedEvent| matches!(e, EmbedEvent::DialogueEnded { dialogue, outcome: DialogueOutcome::Abandoned, edge: None, .. } if *dialogue == d);
    wait_for(embed, 5 * SECS, abandoned);
    let ended = embed.dialogue_load(ir(), GraphSession { facts: [("alarm".to_owned(), FactValue::Boolean(true))].into(), ..session(Some("elena")) }).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(200));
    while embed.poll_event().is_some() {}
    embed.dialogue_close(ended).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(200));
    assert!(!std::iter::from_fn(|| embed.poll_event()).any(|e| matches!(e, EmbedEvent::DialogueEnded { .. })), "an ended story ends once");
}

// 28.10.26f AC11
#[test]
fn load_resume_restore_close_and_dark_rules() {
    let fake = FakeLingara::start(|_, i| if i == 0 { done_with(vec![]) } else { graphs_dark() });
    let embed = started(config(&fake.url));
    load_rules(&embed);
    restore_and_close(&embed);

    let d = load(&embed);
    let request = embed.dialogue_say(d, "hola").unwrap();
    let (event, _) = wait_for(&embed, 5 * SECS, |e| matches!(e, EmbedEvent::Error { .. }));
    let EmbedEvent::Error { error } = event else { unreachable!() };
    assert_eq!((error.kind, error.retryable, error.status, error.code.as_deref(), error.request), (ErrorKind::InvalidInput, false, Some(400), Some("invalid_request"), Some(request)));
    assert!(error.message.contains("not available to this client"), "{}", error.message);
    let snapshot = embed.dialogue_snapshot(d).unwrap();
    assert_eq!((snapshot.phase, snapshot.graph.unwrap().checkpoint.as_str(), snapshot.window.len()), (DialoguePhase::Idle, "intro", 0));
}

// 28.10.26f AC12
#[test]
fn set_fact_and_validate_refuse_cleanly() {
    let fake = FakeLingara::start(|_, _| done_with(vec![]));
    let embed = started(config(&fake.url));
    let d = load(&embed);
    let refusals = [("trst", number(1.0)), ("trust", text("high")), ("gate", text(&"x".repeat(121))), ("gate", text("   ")), ("gate", text("two\nlines"))];
    for (key, value) in refusals {
        assert_eq!(embed.dialogue_set_fact(d, key, value.clone()).unwrap_err().kind, ErrorKind::InvalidInput, "{key} = {value:?}");
    }
    // NaN has no JSON spelling and no `Number`: refused before it reaches the core.
    assert!(serde_json::Number::from_f64(f64::NAN).is_none());
    let nan = r#"{ "type": "dialogue_set_fact", "dialogue": 1, "key": "trust", "value": NaN }"#;
    assert!(serde_json::from_str::<Command>(nan).is_err());
    embed.dialogue_set_fact(d, "gate", text(&"x".repeat(120))).unwrap();

    embed.dialogue_set_fact(d, "alarm", FactValue::Boolean(true)).unwrap();
    assert_eq!(embed.dialogue_snapshot(d).unwrap().phase, DialoguePhase::Ended, "an idle fact change auto-crosses");
    assert_eq!(embed.dialogue_set_fact(d, "trust", number(1.0)).unwrap_err().kind, ErrorKind::InvalidState, "the story is over");
    let free = embed.dialogue_open(crate::types::DialogueConfig {
        npc: crate::types::Npc { name: "Mei".into(), persona: None }, scene: None, source_lang: "en".into(), target_lang: "zh".into(), level: 2,
        window: vec![], languages: vec![], pedagogy: None,
    }).unwrap();
    assert_eq!(embed.dialogue_set_fact(free, "trust", number(1.0)).unwrap_err().kind, ErrorKind::InvalidState, "free-form has no facts");

    embed.dialogue_validate(&keepers().to_string()).unwrap();
    let mut bad = keepers();
    bad["checkpoints"][0]["edges"][1]["predicate"] = json!("trst >= 0.4");
    let error = embed.dialogue_validate(&bad.to_string()).unwrap_err();
    assert_eq!(error.kind, ErrorKind::InvalidInput);
    assert!(error.message.starts_with("checkpoints[0].edges[1].predicate"), "{}", error.message);
    let next = embed.dialogue_load(ir(), session(Some("tomas"))).unwrap();
    assert_eq!(next, free + 1, "validate draws no id, so it opened no dialogue");
}
