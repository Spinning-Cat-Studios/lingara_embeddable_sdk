//! K1 ⇄ mirror (ADR 1.10.26ae D2), through `serde_json::Value` in K1's own
//! JSON shape, as the Godot binding moves GDScript values: the mirrors carry
//! K1's serde attributes, so one generic pair covers every type.
//!
//! - Inward, a value out of K1's range (a negative id, a level of 300) is
//!   refused by K1's own deserialiser as `invalid_input` naming the field:
//!   nothing is cast, nothing panics.
//! - Outward, a K1 value the mirrors do not know (a variant of a newer core)
//!   fails to deserialise and becomes `EmbedEvent::Unknown`, carrying K1's
//!   whole JSON. The tripwire test holds that it never happens in this
//!   workspace.

use lingara_embed as core;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::types::{EmbedError, EmbedEvent, EmbedException, ErrorKind};

/// A mirror into its K1 type.
pub(crate) fn to_core<M: Serialize, K: DeserializeOwned>(field: &str, mirror: &M) -> Result<K, EmbedException> {
    let value = serde_json::to_value(mirror).map_err(|e| invalid(field, e))?;
    serde_json::from_value(value).map_err(|e| invalid(field, e))
}

/// A K1 value into its mirror, or K1's JSON when the mirror cannot hold it.
pub(crate) fn from_core<K: Serialize, M: DeserializeOwned>(value: &K) -> Result<M, Value> {
    let json = serde_json::to_value(value).unwrap_or(Value::Null);
    serde_json::from_value(json.clone()).map_err(|_| json)
}

/// A K1 value the caller must have, such as a method's snapshot: one the
/// mirrors cannot hold is an `internal` error naming it, never a panic.
pub(crate) fn known<K: Serialize, M: DeserializeOwned>(value: &K) -> Result<M, EmbedException> {
    from_core(value).map_err(|json| failed(core::ErrorKind::Internal, format!("a core value this wrapper does not know: {json}")))
}

pub(crate) fn event(value: &core::EmbedEvent) -> EmbedEvent {
    from_core(value).unwrap_or_else(unknown_from_json)
}

/// A variant of a newer core: its `type` tag (empty when it has none) and
/// its whole JSON.
pub fn unknown_from_json(json: Value) -> EmbedEvent {
    let type_tag = json.get("type").and_then(Value::as_str).unwrap_or_default().to_owned();
    EmbedEvent::Unknown { type_tag, json: json.to_string() }
}

/// K1's error. A kind the mirror does not know reads `internal`, with
/// K1's message kept, so an error is always reportable.
pub(crate) fn error(value: &core::EmbedError) -> EmbedError {
    from_core(value).unwrap_or_else(|_| EmbedError {
        kind: ErrorKind::Internal,
        message: value.message.clone(),
        status: value.status.map(i32::from),
        code: value.code.clone(),
        retry_after_ms: value.retry_after_ms.map(|ms| i64::try_from(ms).unwrap_or(i64::MAX)),
        retryable: value.retryable,
        request: value.request.and_then(|r| i64::try_from(r).ok()),
    })
}

pub(crate) fn fail(value: core::EmbedError) -> EmbedException {
    EmbedException::Failed { error: error(&value) }
}

fn failed(kind: core::ErrorKind, message: String) -> EmbedException {
    fail(core::EmbedError::new(kind, message))
}

pub(crate) fn invalid(field: &str, problem: impl std::fmt::Display) -> EmbedException {
    failed(core::ErrorKind::InvalidInput, format!("{field}: {problem}"))
}

/// A Kotlin `Long` or `Int` into K1's unsigned field, or `invalid_input`.
pub(crate) fn narrow<T: TryFrom<i64>>(field: &str, value: impl Into<i64>) -> Result<T, EmbedException> {
    let value = value.into();
    T::try_from(value).map_err(|_| invalid(field, format!("{value} is out of range")))
}

/// One of K1's ids out to Kotlin. K1 caps them at `i64::MAX`, so this
/// refuses nothing K1 issues.
pub(crate) fn id_out(id: u64) -> Result<i64, EmbedException> {
    i64::try_from(id).map_err(|_| failed(core::ErrorKind::Internal, format!("id {id} is beyond i64")))
}
