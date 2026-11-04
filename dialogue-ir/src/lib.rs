#![doc = include_str!("../README.md")]

// The public name stays `build`; the files live under `builder/` because
// scs-snapshot drops any path with a `build` component (publish/snapshot.toml).
#[path = "builder/mod.rs"]
pub mod build;
pub mod canonical;
mod document;
pub mod ir;
mod levels;
pub mod limits;
mod path;
pub mod predicate;
pub mod procedural;
mod rule;
mod shape;
pub mod text;
mod validate;
mod validate_bounds;
mod validate_graph;

// What `dialogue.v2` adds (ADR 3.11.26g), in its own folder; the module
// paths stay flat, as `build` does above.
#[path = "v2/disposition.rs"]
pub mod disposition;
#[path = "v2/shape.rs"]
mod shape_v2;
#[path = "v2/validate.rs"]
mod validate_v2;
#[path = "v2/version.rs"]
pub mod version;

#[cfg(test)]
mod tests;

use std::fmt;

use serde_json::Value;

pub use document::{Document, document_from_value, load_document};
pub use ir::{
    Checkpoint, DialogueIr, DialogueKind, Disposition, Edge, Ending, FactDecl, FactType, FactValue, NpcLanguage, NpcProfile,
    Pedagogy, SCHEMA_V1, SCHEMA_V2,
};
pub use levels::Level;
pub use rule::{Rule, WarningCode};
pub use validate::{Strictness, validate, validate_with};

/// Why a dialogue was refused. `path` is a dotted JSON path
/// (`checkpoints[1].edges[0].predicate`), empty for the document itself;
/// `rule` is the check, as Backend's wire spells it (ADR 3.11.26g §9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrError {
    pub rule: Rule,
    pub path: String,
    pub message: String,
}

impl IrError {
    pub(crate) fn new(rule: Rule, path: impl Into<String>, message: impl Into<String>) -> Self {
        IrError { rule, path: path.into(), message: message.into() }
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
    pub code: WarningCode,
    pub path: String,
    pub message: String,
}

impl Warning {
    pub(crate) fn new(code: WarningCode, path: impl Into<String>, message: impl Into<String>) -> Self {
        Warning { code, path: path.into(), message: message.into() }
    }
}

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}

/// Reads a static graph, `dialogue.v1` or `dialogue.v2`, from its text. Every
/// member is held to its JSON type and closed object first, so a refusal names
/// its path; the bounds and the semantic rules are [`validate`]'s. A procedural
/// document is refused by name: read it with [`load_document`].
pub fn from_json(text: &str) -> Result<DialogueIr, IrError> {
    from_value(&parse_json(text)?)
}

/// [`from_json`] over an already-parsed value.
pub fn from_value(value: &Value) -> Result<DialogueIr, IrError> {
    match document_from_value(value)? {
        Document::Static(ir) => Ok(ir),
        Document::Procedural(_) => Err(IrError::new(Rule::KindMismatch, "kind", "is a procedural dialogue: use load_document")),
    }
}

pub(crate) fn parse_json(text: &str) -> Result<Value, IrError> {
    serde_json::from_str(text).map_err(|err| IrError::new(Rule::Bound, "", format!("not JSON: {err}")))
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
