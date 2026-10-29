#![doc = include_str!("../README.md")]

pub mod ir;
pub mod limits;
mod path;
pub mod predicate;
mod shape;
pub mod text;
mod validate;
mod validate_bounds;
mod validate_graph;

#[cfg(test)]
mod tests;

use std::fmt;

use serde_json::Value;

pub use ir::{Checkpoint, DialogueIr, Edge, FactDecl, FactType, FactValue, NpcLanguage, NpcProfile, Pedagogy, SCHEMA_V1};
pub use validate::validate;

/// Why a dialogue was refused. `path` is a dotted JSON path
/// (`checkpoints[1].edges[0].predicate`), empty for the document itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrError {
    pub path: String,
    pub message: String,
}

impl IrError {
    pub(crate) fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        IrError { path: path.into(), message: message.into() }
    }
}

impl fmt::Display for IrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.path.is_empty() { f.write_str(&self.message) } else { write!(f, "{}: {}", self.path, self.message) }
    }
}

impl std::error::Error for IrError {}

/// Something a dialogue loads with but a developer should hear about: an
/// unreachable checkpoint, or text the schema allows and the turn refuses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warning {
    pub path: String,
    pub message: String,
}

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}

/// Reads a `dialogue.v1` document from its text. Every member is held to its
/// JSON type and closed object first, so a refusal names its path; the bounds
/// and the semantic rules are [`validate`]'s.
pub fn from_json(text: &str) -> Result<DialogueIr, IrError> {
    let value: Value = serde_json::from_str(text).map_err(|err| IrError::new("", format!("not JSON: {err}")))?;
    from_value(&value)
}

/// [`from_json`] over an already-parsed value.
pub fn from_value(value: &Value) -> Result<DialogueIr, IrError> {
    shape::walk(value)?;
    // The walk has held every member to the type this deserialise reads, so
    // a failure here is a gap in the walk, still reported rather than panicked.
    serde_json::from_value(value.clone()).map_err(|err| IrError::new("", err.to_string()))
}

/// [`from_json`] then [`validate`]: a dialogue ready to run, and its warnings.
pub fn load(text: &str) -> Result<(DialogueIr, Vec<Warning>), IrError> {
    let ir = from_json(text)?;
    let warnings = validate(&ir)?;
    Ok((ir, warnings))
}

/// [`limits::ID_PATTERN`], written out so no regex is needed.
pub fn is_ir_id(value: &str) -> bool {
    let mut bytes = value.bytes();
    let first_ok = bytes.next().is_some_and(|b| b.is_ascii_lowercase());
    let rest_ok = bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
    first_ok && rest_ok && value.len() <= limits::MAX_ID_CHARS
}
