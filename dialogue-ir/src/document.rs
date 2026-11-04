//! Either kind of dialogue (ADR 3.11.26g §1): what [`load_document`] answers.
//! [`crate::from_json`] and [`crate::load`] keep answering a static graph and
//! refuse a procedural one by name.

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::ir::{DialogueIr, SCHEMA_V2};
use crate::procedural::ProceduralDialogue;
use crate::validate::{Strictness, validate_with};
use crate::{IrError, Rule, Warning, parse_json, shape, shape_v2, version};

/// A dialogue of either kind. Unboxed on purpose: a caller matches on it once
/// per load, and §1 publishes this shape.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum Document {
    Static(DialogueIr),
    Procedural(ProceduralDialogue),
}

impl Document {
    /// The canonical text of either kind; a procedural document is always
    /// `dialogue.v2` (§9).
    pub fn to_canonical_json(&self) -> String {
        match self {
            Document::Static(ir) => ir.to_canonical_json(),
            Document::Procedural(doc) => crate::canonical::to_string(&crate::canonical::procedural_normal_form(doc)),
        }
    }

    /// The document's own id.
    pub fn id(&self) -> &str {
        match self {
            Document::Static(ir) => &ir.id,
            Document::Procedural(doc) => &doc.id,
        }
    }
}

/// Reads either kind from an already-parsed value: the shape walk of the
/// schema it declares, then the types. A v1 document is opened as v2 (§2).
pub fn document_from_value(value: &Value) -> Result<Document, IrError> {
    if value.get("schema").and_then(Value::as_str) != Some(SCHEMA_V2) {
        shape::walk(value)?;
        return Ok(Document::Static(version::read_v1(typed(value.clone())?)));
    }
    let procedural = shape_v2::walk(value)?;
    // `kind` is not a field: the variant carries it, and the canonical writer
    // writes it back.
    let mut body = value.clone();
    body.as_object_mut().map(|map| map.remove("kind"));
    if procedural { Ok(Document::Procedural(typed(body)?)) } else { Ok(Document::Static(typed(body)?)) }
}

/// The walk has held every member to the type this deserialise reads, so a
/// failure here is a gap in the walk, still reported rather than panicked.
fn typed<T: DeserializeOwned>(value: Value) -> Result<T, IrError> {
    serde_json::from_value(value).map_err(|err| IrError::new(Rule::Bound, "", err.to_string()))
}

/// Reads and validates either kind: a document ready to run or to plan, and
/// its warnings.
pub fn load_document(text: &str) -> Result<(Document, Vec<Warning>), IrError> {
    let doc = document_from_value(&parse_json(text)?)?;
    let warnings = validate_with(&doc, Strictness::Load)?;
    Ok((doc, warnings))
}
