//! The walk [`crate::document_from_value`] runs before it deserialises: every
//! object closed (`x-` members at the top level only), every required member
//! present, every member of the JSON type the IR reads. Each refusal names its
//! path, which a serde error cannot. Lengths, counts and patterns are
//! [`crate::validate`]'s, so an IR a builder made is held to them too.
//!
//! This file holds the walk's machinery and `dialogue.v1`'s tables;
//! `shape_v2.rs` holds v2's, which reuse every v1 table they extend.

use serde_json::Value;

use crate::limits::{MAX_LEVEL, MIN_LEVEL};
use crate::path::{index, member};
use crate::{IrError, Rule};

pub(crate) type Check = fn(&Value, &str) -> Result<(), IrError>;

/// One property of an object: its name, whether it is required, its check.
pub(crate) struct Member(pub &'static str, pub bool, pub Check);

const ROOT: &[Member] = &[
    Member("schema", true, string),
    Member("id", true, string),
    Member("title", false, string),
    Member("pedagogy", false, pedagogy),
    Member("npcs", true, npcs),
    Member("facts", false, facts),
    Member("start", true, string),
    Member("checkpoints", true, checkpoints),
];
pub(crate) const NPC: &[Member] = &[
    Member("id", true, string),
    Member("name", true, string),
    Member("persona", false, string),
    Member("languages", false, languages),
];
const LANGUAGE: &[Member] = &[Member("code", true, string), Member("speaking", true, level), Member("comprehension", true, level)];
const FACT: &[Member] = &[Member("key", true, string), Member("type", true, fact_type), Member("default", false, fact_value)];
pub(crate) const CHECKPOINT: &[Member] = &[
    Member("id", true, string),
    Member("goal", true, string),
    Member("may_disclose", false, strings),
    Member("edges", false, edges),
];
const EDGE: &[Member] =
    &[Member("id", true, string), Member("to", true, string), Member("predicate", false, string), Member("when", false, string)];

/// A `dialogue.v1` document.
pub(crate) fn walk(root: &Value) -> Result<(), IrError> {
    object(root, "", ROOT, true)
}

/// Unknown members first, then each member in table order.
pub(crate) fn object(value: &Value, path: &str, members: &[Member], extensions: bool) -> Result<(), IrError> {
    let map = value.as_object().ok_or_else(|| IrError::new(Rule::Bound, path, "must be an object"))?;
    let known = |key: &str| members.iter().any(|m| m.0 == key) || (extensions && key.starts_with("x-"));
    if let Some(key) = map.keys().find(|key| !known(key)) {
        return Err(IrError::new(Rule::UnknownMember, member(path, key), "is not a member of dialogue.v1"));
    }
    for Member(name, required, check) in members {
        match map.get(*name) {
            Some(value) => check(value, &member(path, name))?,
            None if *required => return Err(IrError::new(Rule::MissingMember, member(path, name), "is required")),
            None => {}
        }
    }
    Ok(())
}

pub(crate) fn array(value: &Value, path: &str, each: impl Fn(&Value, &str) -> Result<(), IrError>) -> Result<(), IrError> {
    let items = value.as_array().ok_or_else(|| IrError::new(Rule::Bound, path, "must be an array"))?;
    items.iter().enumerate().try_for_each(|(i, item)| each(item, &index(path, i)))
}

pub(crate) fn string(value: &Value, path: &str) -> Result<(), IrError> {
    if value.is_string() { Ok(()) } else { Err(IrError::new(Rule::Bound, path, "must be a string")) }
}

pub(crate) fn strings(value: &Value, path: &str) -> Result<(), IrError> {
    array(value, path, string)
}

pub(crate) fn one_of(value: &Value, path: &str, allowed: &[&str]) -> Result<(), IrError> {
    if value.as_str().is_some_and(|s| allowed.contains(&s)) {
        Ok(())
    } else {
        Err(IrError::new(Rule::Bound, path, format!("must be one of {}", allowed.join(", "))))
    }
}

pub(crate) fn pedagogy(value: &Value, path: &str) -> Result<(), IrError> {
    one_of(value, path, &["subtle", "none"])
}

fn fact_type(value: &Value, path: &str) -> Result<(), IrError> {
    one_of(value, path, &["string", "number", "boolean"])
}

/// A `default`: a string, a number or a boolean. Whether it matches the
/// fact's `type` is rule 5, `validate`'s.
fn fact_value(value: &Value, path: &str) -> Result<(), IrError> {
    match value {
        Value::String(_) | Value::Number(_) | Value::Bool(_) => Ok(()),
        _ => Err(IrError::new(Rule::Bound, path, "must be a string, a number or a boolean")),
    }
}

/// A level is an integer from 1 to 9, checked here because the IR holds it
/// as a `u8`, which a larger number would fail to deserialise into.
fn level(value: &Value, path: &str) -> Result<(), IrError> {
    let ok = value.as_u64().is_some_and(|l| (u64::from(MIN_LEVEL)..=u64::from(MAX_LEVEL)).contains(&l));
    if ok { Ok(()) } else { Err(IrError::new(Rule::NumberRange, path, format!("must be an integer from {MIN_LEVEL} to {MAX_LEVEL}"))) }
}

fn npcs(value: &Value, path: &str) -> Result<(), IrError> {
    array(value, path, |npc, path| object(npc, path, NPC, false))
}

pub(crate) fn languages(value: &Value, path: &str) -> Result<(), IrError> {
    array(value, path, |language, path| object(language, path, LANGUAGE, false))
}

pub(crate) fn facts(value: &Value, path: &str) -> Result<(), IrError> {
    array(value, path, |fact, path| object(fact, path, FACT, false))
}

fn checkpoints(value: &Value, path: &str) -> Result<(), IrError> {
    array(value, path, |checkpoint, path| object(checkpoint, path, CHECKPOINT, false))
}

pub(crate) fn edges(value: &Value, path: &str) -> Result<(), IrError> {
    array(value, path, |edge, path| object(edge, path, EDGE, false))
}
