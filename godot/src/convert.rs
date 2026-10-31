//! K1's events into signal calls, and GDScript arguments into K1's
//! `Command`s (ADR 1.10.26ab D3), over `serde_json::Value` with no Godot
//! type anywhere, so `cargo test` reaches every mapping. `variant.rs` is the
//! one place a `Value` becomes a `Variant`.

use std::collections::HashSet;

use lingara_embed::{EmbedError, EmbedEvent, ErrorKind, NpcTurnUpdate, PracticePhase, PracticeSnapshot};
use serde::de::DeserializeOwned;
use serde_json::{Map, Value, json};

/// One `emit_signal`, its arguments in D3's order.
#[derive(Debug, Clone, PartialEq)]
pub struct SignalCall {
    pub name: &'static str,
    pub args: Vec<Value>,
}

impl SignalCall {
    pub fn new(name: &'static str, args: Vec<Value>) -> Self {
        Self { name, args }
    }
}

/// A GDScript argument through `serde_json::from_value`: a negative id, a
/// level of 300 or a malformed `Dictionary` is K1's `invalid_input` naming
/// the field. Nothing is cast or wrapped.
pub fn arg<T: DeserializeOwned>(field: &str, value: Value) -> Result<T, EmbedError> {
    serde_json::from_value(value).map_err(|e| EmbedError::new(ErrorKind::InvalidInput, format!("{field}: {e}")))
}

/// `value` in K1's JSON vocabulary. K1's types always serialise.
pub fn json<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

/// An `Option` of K1 data as a `Dictionary`: `{}` when absent.
fn dict_or_empty<T: serde::Serialize>(value: Option<&T>) -> Value {
    value.map(json).unwrap_or_else(|| Value::Object(Map::new()))
}

/// The sessions whose `practice_finished` has fired: once per session, on
/// D3's terminal rule.
#[derive(Debug, Default)]
pub struct Finished(HashSet<u64>);

/// `reported`, `ended`, `failed`, or a `report_failed` that cannot be re-sent.
pub fn is_terminal(snapshot: &PracticeSnapshot) -> bool {
    match snapshot.phase {
        PracticePhase::Reported | PracticePhase::Ended | PracticePhase::Failed => true,
        PracticePhase::ReportFailed => snapshot.failure.as_ref().is_none_or(|f| !f.retryable),
        _ => false,
    }
}

impl Finished {
    /// `practice_changed`, then `practice_finished` the first time the
    /// session's snapshot is terminal.
    pub fn practice(&mut self, session: u64, snapshot: &PracticeSnapshot) -> Vec<SignalCall> {
        let args = vec![Value::from(session), json(snapshot)];
        let mut calls = vec![SignalCall::new("practice_changed", args.clone())];
        if is_terminal(snapshot) && self.0.insert(session) {
            calls.push(SignalCall::new("practice_finished", args));
        }
        calls
    }
}

/// `core_event` with the whole event, then its typed signal (D3). A variant
/// newer than this binding reaches GDScript through `core_event` alone.
pub fn signals_for(event: &EmbedEvent, finished: &mut Finished) -> Vec<SignalCall> {
    let mut calls = vec![SignalCall::new("core_event", vec![json(event)])];
    calls.extend(typed(event, finished));
    calls
}

fn typed(event: &EmbedEvent, finished: &mut Finished) -> Vec<SignalCall> {
    let one = |name, args| vec![SignalCall::new(name, args)];
    match event {
        EmbedEvent::TokenNeeded { request, reason } => one("token_needed", vec![Value::from(*request), json(reason)]),
        EmbedEvent::Stream { state, cursor } => one("stream_changed", vec![json(state), Value::from(cursor.clone().unwrap_or_default())]),
        EmbedEvent::Lingara { event } => {
            let data = serde_json::from_str::<Value>(&event.data_json).ok().filter(Value::is_object);
            let args = vec![event.id.clone().into(), event.event_type.clone().into(), event.created_at.clone().into(), dict_or_empty(data.as_ref())];
            one("event_received", args)
        }
        EmbedEvent::WorldSent { request, event_id, reaction } => one("world_sent", vec![Value::from(*request), event_id.clone().into(), dict_or_empty(reaction.as_ref())]),
        EmbedEvent::Practice { session, snapshot } => finished.practice(*session, snapshot),
        EmbedEvent::NpcTurn { dialogue, request, update } => one_turn(*dialogue, *request, update),
        // A graph dialogue's story (ADR 28.10.26f D8): an absent edge is "".
        EmbedEvent::CheckpointReached { dialogue, checkpoint, edge, via } => {
            one("checkpoint_reached", vec![Value::from(*dialogue), checkpoint.clone().into(), edge.clone().unwrap_or_default().into(), json(via)])
        }
        EmbedEvent::DialogueEnded { dialogue, outcome, checkpoint, edge } => {
            one("dialogue_ended", vec![Value::from(*dialogue), json(outcome), checkpoint.clone().into(), edge.clone().unwrap_or_default().into()])
        }
        // A stored dialogue's load (ADR 28.10.26i D5).
        EmbedEvent::DialogueLoaded { dialogue, dialogue_id, rev } => one("dialogue_loaded", vec![Value::from(*dialogue), dialogue_id.clone().into(), Value::from(*rev)]),
        EmbedEvent::DialogueLoadFailed { dialogue, error } => one("dialogue_load_failed", vec![Value::from(*dialogue), json(error)]),
        EmbedEvent::Error { error } => one("error_raised", vec![json(error)]),
        EmbedEvent::Log { level, target, message } => one("log_received", vec![level.to_string().into(), target.clone().into(), message.clone().into()]),
        // ADR 30.10.26p D4: once per handle, for the developer; null is "".
        EmbedEvent::UpdateAvailable { latest, deprecated, message } => {
            one("update_available", vec![latest.clone().unwrap_or_default().into(), Value::from(*deprecated), message.clone().unwrap_or_default().into()])
        }
        EmbedEvent::Closed { reason } => one("closed", vec![json(reason)]),
        _ => Vec::new(),
    }
}

/// K1's five turn updates onto their five signals. B3 sends no `started`;
/// `opened` is K1's own, from the headers arriving.
fn one_turn(dialogue: u64, request: u64, update: &NpcTurnUpdate) -> Vec<SignalCall> {
    let d = Value::from(dialogue);
    let call = match update {
        NpcTurnUpdate::Opened => SignalCall::new("npc_turn_opened", vec![d, Value::from(request)]),
        NpcTurnUpdate::Fragment { text } => SignalCall::new("npc_fragment", vec![d, text.clone().into()]),
        NpcTurnUpdate::Notice { code, message } => SignalCall::new("npc_notice", vec![d, code.clone().into(), message.clone().into()]),
        NpcTurnUpdate::Finished { text } => SignalCall::new("npc_replied", vec![d, text.clone().into()]),
        NpcTurnUpdate::Cancelled => SignalCall::new("npc_turn_cancelled", vec![d]),
        _ => return Vec::new(),
    };
    vec![call]
}

/// The refusal a method meets while no handle is live.
pub fn not_running() -> EmbedError {
    EmbedError::new(ErrorKind::Closed, "Lingara is not running: call start() first")
}

/// `execute`'s answer: `{"ok": <CommandOutcome>}` or `{"error": <EmbedError>}`.
pub fn execute_answer(result: &Result<lingara_embed::CommandOutcome, EmbedError>) -> Value {
    match result {
        Ok(outcome) => json!({ "ok": json(outcome) }),
        Err(error) => json!({ "error": json(error) }),
    }
}
