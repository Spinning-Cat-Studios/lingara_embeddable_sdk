//! The procedural spec's shape walk (ADR 3.11.26g §4): every object closed,
//! every required member present, every member of the JSON type the spec
//! reads. Its numbers and words are `spec_rules.rs`'s.

use serde_json::Value;

use crate::shape::{Member, array, object, one_of, string, strings};
use crate::shape_v2::{each, medium, number, small};
use crate::IrError;

const SPEC: &[Member] = &[
    Member("premise", true, string),
    Member("disclosable", false, strings),
    Member("topics", true, topics),
    Member("forbidden", false, forbidden),
    Member("goals", false, goals),
    Member("atoms", true, atoms),
    Member("horizon", false, horizon),
    Member("termination", true, termination),
];
const TOPIC: &[Member] = &[
    Member("id", true, string),
    Member("about", true, string),
    Member("npcs", false, strings),
    Member("when", false, string),
    Member("discloses", false, strings),
];
const FORBIDDEN: &[Member] = &[Member("about", true, string), Member("npcs", false, strings)];
const GOAL: &[Member] =
    &[Member("id", true, string), Member("describe", true, string), Member("predicate", false, string), Member("when", false, string)];
const ATOM_USE: &[Member] = &[Member("atom", true, atom), Member("prefer", false, number)];
const HORIZON: &[Member] =
    &[Member("plan_depth", false, small), Member("sketch_depth", false, small), Member("max_nodes", false, small)];
const TERMINATION: &[Member] = &[
    Member("max_turns", true, medium),
    Member("max_beats", true, medium),
    Member("success", false, success),
    Member("stall", false, stall),
    Member("on_limit", false, string),
];
const SUCCESS: &[Member] = &[Member("goal", true, string), Member("ending", true, string)];
const STALL: &[Member] = &[Member("turns", false, small), Member("ending", false, string)];

/// A document's `procedural` member.
pub(crate) fn walk_spec(value: &Value, path: &str) -> Result<(), IrError> {
    object(value, path, SPEC, false)
}

fn atom(value: &Value, path: &str) -> Result<(), IrError> {
    one_of(value, path, &["open", "elicit", "reveal", "practise", "close"])
}

fn topics(value: &Value, path: &str) -> Result<(), IrError> {
    array(value, path, each(TOPIC))
}

fn forbidden(value: &Value, path: &str) -> Result<(), IrError> {
    array(value, path, each(FORBIDDEN))
}

fn goals(value: &Value, path: &str) -> Result<(), IrError> {
    array(value, path, each(GOAL))
}

fn atoms(value: &Value, path: &str) -> Result<(), IrError> {
    array(value, path, each(ATOM_USE))
}

fn horizon(value: &Value, path: &str) -> Result<(), IrError> {
    object(value, path, HORIZON, false)
}

fn termination(value: &Value, path: &str) -> Result<(), IrError> {
    object(value, path, TERMINATION, false)
}

fn success(value: &Value, path: &str) -> Result<(), IrError> {
    object(value, path, SUCCESS, false)
}

fn stall(value: &Value, path: &str) -> Result<(), IrError> {
    object(value, path, STALL, false)
}
