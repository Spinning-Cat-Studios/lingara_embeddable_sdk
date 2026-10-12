//! How a `poll_until` matches an event, exactly as the core's harness does:
//! the event must *contain* the case's `match` object, compared after
//! dropping nulls. An object matches when each of its keys does; an array
//! when it is equal in length and each element matches; anything else when
//! it is equal.

use serde_json::Value;

pub fn without_nulls(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(map.iter().filter(|(_, v)| !v.is_null()).map(|(k, v)| (k.clone(), without_nulls(v))).collect()),
        Value::Array(items) => Value::Array(items.iter().map(without_nulls).collect()),
        other => other.clone(),
    }
}

pub fn contains(actual: &Value, wanted: &Value) -> bool {
    match (actual, wanted) {
        (_, Value::Null) => true,
        (Value::Object(a), Value::Object(w)) => w.iter().all(|(k, v)| v.is_null() || a.get(k).is_some_and(|got| contains(got, v))),
        (Value::Array(a), Value::Array(w)) => a.len() == w.len() && a.iter().zip(w).all(|(got, v)| contains(got, v)),
        (a, w) => a == w,
    }
}
