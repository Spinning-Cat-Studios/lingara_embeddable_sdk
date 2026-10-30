//! The pure graph runner (ADR 28.10.26f D4), and the two session rules it
//! needs a live dialogue to show: auto-crossing waits for a turn, and an
//! ended story refuses `say`.

use std::sync::Arc;

use serde_json::{Value, json};

use crate::dialogue::graph::{GraphState, Step};
use crate::error::ErrorKind;
use crate::tests::fake_lingara::{After, FakeLingara, Reply, frame};
use crate::tests::{SECS, config, started, wait_for};
use crate::types::{CrossedVia, DialogueOutcome, DialoguePhase, DialogueSource, EmbedEvent, FactValue, GraphSession, LogLevel};

/// The intro offers three judged edges and one game-only edge to `failure`.
fn story() -> Value {
    json!({
        "schema": "dialogue.v1", "id": "story", "npcs": [{ "id": "elena", "name": "Elena" }],
        "facts": [{ "key": "trust", "type": "number", "default": 0 }, { "key": "alarm", "type": "boolean", "default": false },
                  { "key": "ready", "type": "boolean" }],
        "start": "intro",
        "checkpoints": [
            { "id": "intro", "goal": "Ask who", "may_disclose": [], "edges": [
                { "id": "alarm", "to": "failure", "predicate": "alarm" },
                { "id": "low", "to": "ask", "predicate": "trust < 0.4", "when": "the player is rude" },
                { "id": "high", "to": "ask", "predicate": "trust >= 0.4", "when": "the player is kind" },
                { "id": "any", "to": "ask", "when": "the player says who they are" },
                { "id": "auto", "to": "hall", "predicate": "ready" }] },
            { "id": "hall", "goal": "Walk", "edges": [{ "id": "on", "to": "ask" }] },
            { "id": "ask", "goal": "Admit", "edges": [{ "id": "gate", "to": "success", "when": "asks" }] }
        ]
    })
}

fn state(ir: &Value) -> GraphState {
    let ir = lingara_dialogue_ir::from_value(ir).expect("the IR reads");
    GraphState::new(Arc::new(ir), 0, 0)
}

fn number(n: f64) -> FactValue {
    FactValue::Number(serde_json::Number::from_f64(n).unwrap())
}

fn offered(s: &GraphState) -> Vec<String> {
    s.offered().iter().map(|e| e.id.clone()).collect()
}

fn reached(checkpoint: &str, edge: Option<&str>, via: CrossedVia) -> Step {
    Step::Reached { checkpoint: checkpoint.into(), edge: edge.map(Into::into), via }
}

fn is_log(step: &Step, want: LogLevel) -> bool {
    matches!(step, Step::Log { level, .. } if *level == want)
}

// 28.10.26f AC6
#[test]
fn only_open_judged_edges_are_offered_in_order() {
    let mut s = state(&story());
    assert_eq!(offered(&s), ["low", "any"], "a failing predicate is not offered, and `alarm`/`auto` carry no `when`");
    s.set_fact("trust", number(0.5)).unwrap();
    assert_eq!(offered(&s), ["high", "any"], "IR order, whatever the facts");
}

// 28.10.26f AC7
#[test]
fn auto_edges_cross_in_order_and_wait_for_a_turn() {
    let mut s = state(&story());
    assert_eq!(s.start(), [reached("intro", None, CrossedVia::Start)], "nothing is open without a `when` yet");
    s.set_fact("ready", FactValue::Boolean(true)).unwrap();
    let want = [reached("hall", Some("auto"), CrossedVia::Auto), reached("ask", Some("on"), CrossedVia::Auto)];
    assert_eq!(s.auto_run(), want, "the first open edge, then again from where it lands");

    // During a turn the fact is stored, and crossing waits for the turn's end.
    let hold = |_: &_, _| Reply::Sse(vec![frame("delta", json!({ "text": "..." }))], After::Hold);
    let fake = FakeLingara::start(hold);
    let embed = started(config(&fake.url));
    let session = GraphSession { npc: None, source_lang: "en".into(), target_lang: "es".into(), level: 2, scene: None, facts: Default::default(), checkpoint: None, window: vec![] };
    let d = embed.dialogue_load(DialogueSource::Ir { ir_json: story().to_string() }, session).unwrap();
    embed.dialogue_say(d, "hola").unwrap();
    wait_for(&embed, 5 * SECS, |e| matches!(e, EmbedEvent::NpcTurn { .. }));
    embed.dialogue_set_fact(d, "ready", FactValue::Boolean(true)).unwrap();
    assert_eq!(embed.dialogue_snapshot(d).unwrap().graph.unwrap().checkpoint, "intro", "no crossing under a streaming reply");
    embed.dialogue_cancel(d).unwrap();
    let (_, before) = wait_for(&embed, 5 * SECS, |e| matches!(e, EmbedEvent::CheckpointReached { checkpoint, .. } if checkpoint == "ask"));
    assert!(before.iter().any(|e| matches!(e, EmbedEvent::CheckpointReached { checkpoint, via: CrossedVia::Auto, .. } if checkpoint == "hall")));
}

// 28.10.26f AC8
#[test]
fn unsent_edges_are_ignored_and_outcomes_end() {
    let mut s = state(&story());
    let steps = s.apply_judged(&["any".into()], "high", "ask");
    assert!(steps.len() == 1 && is_log(&steps[0], LogLevel::Warn), "an edge the turn did not send: {steps:?}");
    assert_eq!(s.current().id, "intro");

    let steps = s.apply_judged(&["any".into()], "any", "elsewhere");
    assert!(is_log(&steps[0], LogLevel::Warn), "a differing `to` is logged");
    assert_eq!(steps[1], reached("ask", Some("any"), CrossedVia::Judge), "and the IR's `to` is used");

    let steps = s.apply_judged(&["gate".into()], "gate", "success");
    assert_eq!(steps, [Step::Ended { outcome: DialogueOutcome::Success, checkpoint: "ask".into(), edge: Some("gate".into()) }]);
    assert!(s.ended() && s.abandon().is_none());

    // A live dialogue whose story ends on load refuses a line.
    let fake = FakeLingara::start(|_, _| Reply::Json(500, json!({})));
    let embed = started(config(&fake.url));
    let facts = [("alarm".to_owned(), FactValue::Boolean(true))].into();
    let session = GraphSession { npc: None, source_lang: "en".into(), target_lang: "es".into(), level: 2, scene: None, facts, checkpoint: None, window: vec![] };
    let d = embed.dialogue_load(DialogueSource::Ir { ir_json: story().to_string() }, session).unwrap();
    let ended = |e: &EmbedEvent| matches!(e, EmbedEvent::DialogueEnded { outcome: DialogueOutcome::Failure, checkpoint, edge: Some(edge), .. } if checkpoint == "intro" && edge == "alarm");
    wait_for(&embed, 5 * SECS, ended);
    assert_eq!(embed.dialogue_snapshot(d).unwrap().phase, DialoguePhase::Ended);
    assert_eq!(embed.dialogue_say(d, "hola").unwrap_err().kind, ErrorKind::InvalidState);
}

// 28.10.26f AC24
#[test]
fn an_auto_run_ends_and_its_cap_is_a_bug() {
    let mut s = state(&story());
    s.set_fact("ready", FactValue::Boolean(true)).unwrap();
    let steps = s.start();
    assert_eq!(steps.len(), 3, "start, then two auto crossings, then no open edge without a `when`");

    // `from_value` reads without `validate`, so a when-less cycle gets through.
    let cycle = json!({
        "schema": "dialogue.v1", "id": "loop", "npcs": [{ "id": "a", "name": "A" }], "start": "a",
        "checkpoints": [{ "id": "a", "goal": "A", "edges": [{ "id": "ab", "to": "b" }] }, { "id": "b", "goal": "B", "edges": [{ "id": "ba", "to": "a" }] }]
    });
    assert!(lingara_dialogue_ir::load(&cycle.to_string()).is_err(), "load refuses it (D2 rule 6)");
    let steps = state(&cycle).start();
    let crossings = steps.iter().filter(|s| matches!(s, Step::Reached { via: CrossedVia::Auto, .. })).count();
    assert_eq!(crossings, lingara_dialogue_ir::limits::MAX_CHECKPOINTS);
    assert!(is_log(steps.last().unwrap(), LogLevel::Error), "the cap stops the run and logs at error");
}
