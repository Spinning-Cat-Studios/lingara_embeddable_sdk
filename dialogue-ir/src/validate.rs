//! `validate` (ADR 28.10.26f D2): the schema **plus** Backend ADR
//! 28.10.26e §10's seven semantic rules, and nothing more ("refuse nothing
//! the schema allows"). The rules are numbered as §10 numbers them, so a
//! citation means the same rule in both documents.

use std::collections::HashSet;

use crate::ir::{DialogueIr, Edge, FactType, FactValue};
use crate::limits::{MAX_IR_BYTES, OUTCOMES};
use crate::path::{edge, index, member};
use crate::text::one_line;
use crate::validate_graph::{unreachable, when_less_cycle};
use crate::{IrError, Warning, predicate, validate_bounds};

/// Every rule in order, the first refusal winning; then the warnings: an
/// unreachable checkpoint, and text the schema allows but a turn refuses.
pub fn validate(ir: &DialogueIr) -> Result<Vec<Warning>, IrError> {
    validate_bounds::check(ir)?;
    unique_ids(ir)?;
    targets(ir)?;
    disclosed_facts(ir)?;
    predicates(ir)?;
    default_types(ir)?;
    if let Some(path) = when_less_cycle(ir) {
        return Err(IrError::new(path, "closes a cycle of edges without `when`, which would loop with no turn"));
    }
    size(ir)?;
    Ok(warnings(ir))
}

/// Every edge with its path, in document order.
fn edges(ir: &DialogueIr) -> impl Iterator<Item = (String, &Edge)> {
    ir.checkpoints.iter().enumerate().flat_map(|(i, c)| c.edges.iter().enumerate().map(move |(j, e)| (edge(i, j), e)))
}

/// The second occurrence of the first repeat, named by its path.
fn first_repeat<'a>(ids: impl Iterator<Item = (String, &'a str)>) -> Result<(), IrError> {
    let mut seen = HashSet::new();
    for (path, id) in ids {
        if !seen.insert(id) {
            return Err(IrError::new(path, format!("repeats the id \"{id}\"")));
        }
    }
    Ok(())
}

/// Rule 1: unique within each list, and edge ids unique across the whole
/// dialogue, because a crossing is keyed by edge id alone.
fn unique_ids(ir: &DialogueIr) -> Result<(), IrError> {
    first_repeat(ir.npcs.iter().enumerate().map(|(i, n)| (member(&index("npcs", i), "id"), n.id.as_str())))?;
    first_repeat(ir.facts.iter().enumerate().map(|(i, f)| (member(&index("facts", i), "key"), f.key.as_str())))?;
    first_repeat(ir.checkpoints.iter().enumerate().map(|(i, c)| (member(&index("checkpoints", i), "id"), c.id.as_str())))?;
    first_repeat(edges(ir).map(|(path, e)| (member(&path, "id"), e.id.as_str())))
}

/// Rule 2: `start` and every non-outcome `to` name a checkpoint. (A
/// checkpoint named after an outcome is rule 0's, as the schema says it.)
fn targets(ir: &DialogueIr) -> Result<(), IrError> {
    if ir.checkpoint(&ir.start).is_none() {
        return Err(IrError::new("start", format!("names no checkpoint: \"{}\"", ir.start)));
    }
    let dangling = edges(ir).find(|(_, e)| !OUTCOMES.contains(&e.to.as_str()) && ir.checkpoint(&e.to).is_none());
    match dangling {
        Some((path, e)) => Err(IrError::new(member(&path, "to"), format!("names no checkpoint or outcome: \"{}\"", e.to))),
        None => Ok(()),
    }
}

/// Rule 3, for `may_disclose`. The facts a predicate reads are checked with
/// the predicate itself (rule 4), which names the predicate's path.
fn disclosed_facts(ir: &DialogueIr) -> Result<(), IrError> {
    for (i, checkpoint) in ir.checkpoints.iter().enumerate() {
        if let Some(k) = checkpoint.may_disclose.iter().position(|key| ir.fact(key).is_none()) {
            let path = index(&member(&index("checkpoints", i), "may_disclose"), k);
            return Err(IrError::new(path, format!("unknown fact \"{}\"", checkpoint.may_disclose[k])));
        }
    }
    Ok(())
}

/// Rules 3 and 4: every predicate parses **and** type-checks against the
/// declarations (`predicate::check`), so it reads only declared facts.
fn predicates(ir: &DialogueIr) -> Result<(), IrError> {
    for (path, e) in edges(ir) {
        if let Some(text) = &e.predicate {
            predicate::check(text, &ir.facts).map_err(|err| IrError::new(member(&path, "predicate"), err.to_string()))?;
        }
    }
    Ok(())
}

/// Rule 5: a fact's `default` matches its `type`.
fn default_types(ir: &DialogueIr) -> Result<(), IrError> {
    for (i, fact) in ir.facts.iter().enumerate() {
        let Some(default) = &fact.default else { continue };
        if default.fact_type() != fact.kind {
            let path = member(&index("facts", i), "default");
            return Err(IrError::new(path, format!("must be a {} to match the fact's type", type_name(fact.kind))));
        }
    }
    Ok(())
}

fn type_name(kind: FactType) -> &'static str {
    match kind {
        FactType::String => "string",
        FactType::Number => "number",
        FactType::Boolean => "boolean",
    }
}

/// Rule 7, on the compact `serde_json` form of the parsed IR: K2's normal
/// form, which differs from the JCS form only where a number is spelt
/// differently (`2.0` against `2`). K2 moves this onto its canonical writer.
fn size(ir: &DialogueIr) -> Result<(), IrError> {
    let bytes = serde_json::to_vec(ir).map_err(|err| IrError::new("", err.to_string()))?.len();
    if bytes <= MAX_IR_BYTES {
        return Ok(());
    }
    Err(IrError::new("", format!("is {bytes} bytes in normal form; at most {MAX_IR_BYTES}")))
}

fn warnings(ir: &DialogueIr) -> Vec<Warning> {
    let mut out: Vec<Warning> = unreachable(ir)
        .into_iter()
        .map(|i| Warning { path: index("checkpoints", i), message: "is unreachable from `start`".into() })
        .collect();
    let mut text = |path: String, value: &str| {
        if let Err(fault) = one_line(value) {
            out.push(Warning { path, message: fault.describe().into() });
        }
    };
    for (i, npc) in ir.npcs.iter().enumerate() {
        text(member(&index("npcs", i), "name"), &npc.name);
        npc.persona.iter().for_each(|persona| text(member(&index("npcs", i), "persona"), persona));
    }
    for (i, fact) in ir.facts.iter().enumerate() {
        if let Some(FactValue::String(default)) = &fact.default {
            text(member(&index("facts", i), "default"), default);
        }
    }
    for (i, checkpoint) in ir.checkpoints.iter().enumerate() {
        text(member(&index("checkpoints", i), "goal"), &checkpoint.goal);
    }
    for (path, e) in edges(ir) {
        e.when.iter().for_each(|when| text(member(&path, "when"), when));
    }
    out
}
