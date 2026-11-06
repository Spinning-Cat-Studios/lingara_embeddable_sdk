//! `dialogue.v2`'s shape walk (ADR 3.11.26g §1): one per kind, chosen by
//! `kind`, after refusing a member of the other kind (`kind_mismatch`). The
//! procedural spec's tables are `procedural/spec_shape.rs`'s.

use serde_json::Value;

use crate::procedural::walk_spec;
use crate::shape::{CHECKPOINT, Member, NPC, array, facts, object, one_of, pedagogy, string};
use crate::{IrError, Rule};

const STATIC: &[Member] = &[
    Member("schema", true, string),
    Member("kind", false, kind),
    Member("id", true, string),
    Member("title", false, string),
    Member("pedagogy", false, pedagogy),
    Member("endings", true, endings),
    Member("npcs", true, npcs),
    Member("facts", false, facts),
    Member("start", true, string),
    Member("checkpoints", true, checkpoints),
];
const PROCEDURAL_ROOT: &[Member] = &[
    Member("schema", true, string),
    Member("kind", true, kind),
    Member("id", true, string),
    Member("title", false, string),
    Member("pedagogy", false, pedagogy),
    Member("endings", true, endings),
    Member("npcs", true, npcs),
    Member("facts", false, facts),
    Member("procedural", true, walk_spec),
];
const ENDING: &[Member] = &[Member("id", true, string), Member("label", false, string)];
const DISPOSITION: &[Member] = &[Member("suggestibility", true, number), Member("initiative", true, number)];

/// Whether a v2 document is procedural, after its walk.
pub(crate) fn walk(root: &Value) -> Result<bool, IrError> {
    let map = root.as_object().ok_or_else(|| IrError::new(Rule::Bound, "", "must be an object"))?;
    if let Some(kind) = map.get("kind") {
        self::kind(kind, "kind")?;
    }
    let procedural = map.get("kind").and_then(Value::as_str) == Some("procedural");
    let foreign: &[&str] = if procedural { &["start", "checkpoints"] } else { &["procedural"] };
    if let Some(name) = foreign.iter().find(|name| map.contains_key(**name)) {
        let kind = if procedural { "procedural" } else { "static" };
        return Err(IrError::new(Rule::KindMismatch, *name, format!("is not a member of a {kind} dialogue")));
    }
    let table = if procedural { PROCEDURAL_ROOT } else { STATIC };
    object(root, "", table, true).map_err(v2_words)?;
    Ok(procedural)
}

/// The machinery's unknown-member message names v1; a v2 walk says v2.
fn v2_words(mut err: IrError) -> IrError {
    if err.rule == Rule::UnknownMember {
        err.message = "is not a member of dialogue.v2".into();
    }
    err
}

fn kind(value: &Value, path: &str) -> Result<(), IrError> {
    one_of(value, path, &["static", "procedural"])
}

/// Any JSON number; its range is `validate`'s.
pub(crate) fn number(value: &Value, path: &str) -> Result<(), IrError> {
    if value.is_number() { Ok(()) } else { Err(IrError::new(Rule::Bound, path, "must be a number")) }
}

/// An integer within `0..=max`, checked here because the IR holds it in a
/// small unsigned type that a larger number would fail to deserialise into.
fn integer(value: &Value, path: &str, max: u64) -> Result<(), IrError> {
    let ok = value.as_u64().is_some_and(|n| n <= max);
    if ok { Ok(()) } else { Err(IrError::new(Rule::NumberRange, path, format!("must be an integer from 0 to {max}"))) }
}

/// An integer a `u8` holds.
pub(crate) fn small(value: &Value, path: &str) -> Result<(), IrError> {
    integer(value, path, u64::from(u8::MAX))
}

/// An integer a `u16` holds.
pub(crate) fn medium(value: &Value, path: &str) -> Result<(), IrError> {
    integer(value, path, u64::from(u16::MAX))
}

/// Each item of an array, an object of `table`.
pub(crate) fn each(table: &'static [Member]) -> impl Fn(&Value, &str) -> Result<(), IrError> {
    move |item, path| object(item, path, table, false)
}

fn endings(value: &Value, path: &str) -> Result<(), IrError> {
    array(value, path, each(ENDING))
}

/// v1's NPC members plus `disposition`.
fn npcs(value: &Value, path: &str) -> Result<(), IrError> {
    array(value, path, |npc, path| {
        let mut probe = npc.clone();
        let disposition = probe.as_object_mut().and_then(|map| map.remove("disposition"));
        object(&probe, path, NPC, false)?;
        disposition.map_or(Ok(()), |d| object(&d, &crate::path::member(path, "disposition"), DISPOSITION, false))
    })
}

/// v1's checkpoint members plus `steer_to` and `label`.
fn checkpoints(value: &Value, path: &str) -> Result<(), IrError> {
    array(value, path, |checkpoint, path| {
        let mut probe = checkpoint.clone();
        let steer = probe.as_object_mut().and_then(|map| map.remove("steer_to"));
        let label = probe.as_object_mut().and_then(|map| map.remove("label"));
        object(&probe, path, CHECKPOINT, false)?;
        steer.map_or(Ok(()), |s| string(&s, &crate::path::member(path, "steer_to")))?;
        label.map_or(Ok(()), |l| string(&l, &crate::path::member(path, "label")))
    })
}
