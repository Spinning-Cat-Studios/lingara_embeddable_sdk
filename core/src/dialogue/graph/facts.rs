//! Facts on the way in and on the way out (ADR 28.10.26f D5, D6), pure.
//!
//! In: a value the game sets is declared, of its declared type, finite, and,
//! for a string, text the turn would accept (N2 §2: 1–120 characters, one
//! visible line). Out: a turn sends only the current checkpoint's
//! `may_disclose` keys that have a sendable value, never a `predicate`, and
//! the offered edges in IR order.

use lingara_dialogue_ir::{DialogueIr, FactValue, limits, predicate, text};
use serde_json::{Map, Value, json};

use crate::dialogue::graph::runner::GraphState;

/// Why `value` cannot be `key`'s, if it cannot.
pub(crate) fn check(ir: &DialogueIr, key: &str, value: &FactValue) -> Result<(), String> {
    let Some(decl) = ir.fact(key) else { return Err(format!("no fact `{key}` is declared")) };
    if value.fact_type() != decl.kind {
        return Err(format!("fact `{key}` is declared {:?}, not {:?}", decl.kind, value.fact_type()));
    }
    match value {
        FactValue::Number(n) if !n.as_f64().is_some_and(f64::is_finite) => Err(format!("fact `{key}` must be a finite number")),
        FactValue::String(s) => sendable_text(s).map_err(|why| format!("fact `{key}` {why}")),
        _ => Ok(()),
    }
}

fn sendable_text(value: &str) -> Result<(), String> {
    text::one_line(value).map_err(|fault| fault.describe().to_owned())?;
    let n = value.chars().count();
    if n > limits::MAX_FACT_STRING_CHARS {
        return Err(format!("is {n} characters, more than the turn's {}", limits::MAX_FACT_STRING_CHARS));
    }
    Ok(())
}

/// The value a turn sends for `key`: the set one, else the default; `None`
/// when unset, or when a string default would be refused on the wire.
fn sendable<'a>(state: &'a GraphState, key: &str) -> Option<&'a FactValue> {
    let value = predicate::value(state.set_facts(), &state.ir().facts, key)?;
    match value {
        FactValue::String(s) if sendable_text(s).is_err() => None,
        _ => Some(value),
    }
}

/// Adds the graph's fields to a turn's `body` (N1's `pedagogy`, N2 §2's
/// `checkpoint`, `edges` and `facts`) and answers the edge ids it sent.
pub(crate) fn turn_fields(state: &GraphState, body: &mut Value) -> Vec<String> {
    let checkpoint = state.current();
    body["pedagogy"] = json!(state.ir().pedagogy);
    body["checkpoint"] = json!({ "id": checkpoint.id, "goal": checkpoint.goal, "may_disclose": checkpoint.may_disclose });
    let offered = state.offered();
    if !offered.is_empty() {
        body["edges"] = offered.iter().map(|edge| json!({ "id": edge.id, "to": edge.to, "when": edge.when })).collect();
    }
    let facts: Map<String, Value> = checkpoint.may_disclose.iter().filter_map(|key| sendable(state, key).map(|value| (key.clone(), json!(value)))).collect();
    if !facts.is_empty() {
        body["facts"] = Value::Object(facts);
    }
    offered.iter().map(|edge| edge.id.clone()).collect()
}
