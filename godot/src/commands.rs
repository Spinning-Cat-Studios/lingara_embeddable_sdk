//! A flat method's GDScript arguments into the K1 `Command` K1's flat
//! method of the same name runs (ADR 1.10.26ab D3). Each method is one
//! `build` call: its name is the command's `type` tag, so the flat and
//! generic paths cannot disagree.

use lingara_embed::{Command, EmbedError, ErrorKind};
use serde_json::{Map, Value};

use crate::convert::arg;

/// A narrowing field, checked on its own so a refusal names it: GDScript's
/// `int` is 64-bit signed, and K1 takes `u64` ids, `u32` options and
/// lifetimes, and `u8` levels.
fn narrowed(field: &str, value: Value) -> Result<Value, EmbedError> {
    match field {
        // `dialogue_load`'s `session` is K1's `GraphSession`, not a practice id.
        "session" if value.is_object() => Ok(value),
        "session" | "dialogue" => arg::<u64>(field, value.clone()).map(Value::from),
        "option" | "expires_in_secs" => arg::<u32>(field, value.clone()).map(Value::from),
        "level" => arg::<u8>(field, value.clone()).map(Value::from),
        _ => Ok(value),
    }
}

/// `{"type": kind, …fields}` through `serde_json::from_value`.
pub fn build(kind: &str, fields: Vec<(&str, Value)>) -> Result<Command, EmbedError> {
    let mut object = Map::new();
    object.insert("type".into(), kind.into());
    for (field, value) in fields {
        object.insert(field.into(), narrowed(field, value)?);
    }
    arg(kind, Value::Object(object))
}

/// `execute(command: Dictionary)`: any `Command` in K1's JSON shape.
pub fn from_dictionary(command: Value) -> Result<Command, EmbedError> {
    match command.get("type").and_then(Value::as_str) {
        Some(_) => arg("command", command),
        None => Err(EmbedError::new(ErrorKind::InvalidInput, "command: a Command needs its `type` tag")),
    }
}

/// `set_token_endpoint`'s `{name: value}` `Dictionary` as K1's header list.
/// Each value is a run-time secret: it is never stored in a property.
pub fn headers(headers: Value) -> Result<Value, EmbedError> {
    match headers {
        Value::Null => Ok(Value::Array(Vec::new())),
        Value::Object(map) => {
            let mut list = Vec::with_capacity(map.len());
            for (name, value) in map {
                let Value::String(value) = value else {
                    return Err(EmbedError::new(ErrorKind::InvalidInput, format!("headers: the value of {name} must be a String")));
                };
                list.push(serde_json::json!({ "name": name, "value": value }));
            }
            Ok(Value::Array(list))
        }
        _ => Err(EmbedError::new(ErrorKind::InvalidInput, "headers: must be a Dictionary of name to value")),
    }
}

/// `dialogue_set_scene`: an empty scene clears it.
pub fn scene(scene: &str) -> Value {
    if scene.is_empty() { Value::Null } else { scene.into() }
}

/// The session a practice command moves, so its returned snapshot is also
/// emitted as `practice_changed`. `practice_snapshot` only reads.
pub fn session_of(command: &Command) -> Option<u64> {
    match command {
        Command::PracticeAnswer { session, .. }
        | Command::PracticeChoose { session, .. }
        | Command::PracticeReveal { session }
        | Command::PracticeSelfGrade { session, .. }
        | Command::PracticeNext { session }
        | Command::PracticeFinish { session } => Some(*session),
        _ => None,
    }
}
