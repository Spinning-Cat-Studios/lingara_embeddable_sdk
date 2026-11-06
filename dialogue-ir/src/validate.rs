//! `validate` (ADR 28.10.26f D2): the schema **plus** Backend ADR
//! 28.10.26e §10's seven semantic rules, and nothing more ("refuse nothing
//! the schema allows"). The rules are numbered as §10 numbers them, so a
//! citation means the same rule in both documents. `dialogue.v2`'s additions
//! (ADR 3.11.26g §3, §9, §11.1) are `validate_v2.rs`'s.

use std::collections::HashSet;

use crate::ir::{DialogueIr, Edge, FactDecl, FactType, FactValue};
use crate::path::{edge, index, member};
use crate::predicate::check_with_rules;
use crate::text::one_line;
use crate::validate_bounds::size;
use crate::validate_graph::{no_ending_reachable, unreachable, when_less_cycle};
use crate::{Document, IrError, Rule, Warning, WarningCode, validate_bounds, validate_v2};

/// How text the schema allows but a turn refuses is treated (§9).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Strictness {
    /// A warning: what a kit's load does.
    #[default]
    Load,
    /// A `text_blank` / `text_multiline` refusal: what a save does.
    Save,
}

/// Either kind, at either strictness: the one entry point.
pub fn validate_with(doc: &Document, strictness: Strictness) -> Result<Vec<Warning>, IrError> {
    let warnings = match doc {
        Document::Static(ir) => validate(ir)?,
        Document::Procedural(doc) => validate_v2::procedural(doc)?,
    };
    if strictness == Strictness::Save {
        if let Some((w, fault)) = warnings.iter().find_map(|w| if let WarningCode::Text(f) = w.code { Some((w, f)) } else { None }) {
            return Err(IrError::new(Rule::of_text(fault), w.path.clone(), w.message.clone()));
        }
    }
    Ok(warnings)
}

/// Every rule in order on a static graph, the first refusal winning; then the
/// warnings: what is unreachable, the stance, and text a turn refuses.
pub fn validate(ir: &DialogueIr) -> Result<Vec<Warning>, IrError> {
    validate_bounds::check(ir)?;
    unique_ids(ir)?;
    targets(ir)?;
    disclosed_facts(ir)?;
    predicates(ir)?;
    default_types(&ir.facts)?;
    if let Some(path) = when_less_cycle(ir) {
        return Err(IrError::new(Rule::WhenLessCycle, path, "closes a cycle of edges without `when`, which would loop with no turn"));
    }
    validate_v2::steer_targets(ir)?;
    size(ir.to_canonical_json().len())?;
    Ok(warnings(ir))
}

/// Every edge with its path, in document order.
fn edges(ir: &DialogueIr) -> impl Iterator<Item = (String, &Edge)> {
    ir.checkpoints.iter().enumerate().flat_map(|(i, c)| c.edges.iter().enumerate().map(move |(j, e)| (edge(i, j), e)))
}

/// The second occurrence of the first repeat, named by its path.
pub(crate) fn first_repeat<'a>(ids: impl Iterator<Item = (String, &'a str)>) -> Result<(), IrError> {
    let mut seen = HashSet::new();
    for (path, id) in ids {
        if !seen.insert(id) {
            return Err(IrError::new(Rule::DuplicateId, path, format!("repeats the id \"{id}\"")));
        }
    }
    Ok(())
}

/// Rule 1 for the members both kinds share: the cast, the facts, the endings.
pub(crate) fn unique_header_ids(ir: &DialogueIr) -> Result<(), IrError> {
    first_repeat(ir.npcs.iter().enumerate().map(|(i, n)| (member(&index("npcs", i), "id"), n.id.as_str())))?;
    first_repeat(ir.facts.iter().enumerate().map(|(i, f)| (member(&index("facts", i), "key"), f.key.as_str())))?;
    first_repeat(ir.endings.iter().enumerate().map(|(i, e)| (member(&index("endings", i), "id"), e.id.as_str())))
}

/// Rule 1: unique within each list, and edge ids unique across the whole
/// dialogue, because a crossing is keyed by edge id alone.
fn unique_ids(ir: &DialogueIr) -> Result<(), IrError> {
    unique_header_ids(ir)?;
    first_repeat(ir.checkpoints.iter().enumerate().map(|(i, c)| (member(&index("checkpoints", i), "id"), c.id.as_str())))?;
    first_repeat(edges(ir).map(|(path, e)| (member(&path, "id"), e.id.as_str())))
}

/// Rule 2: `start` names a checkpoint, and every `to` a checkpoint, a
/// declared ending or `abandoned` (§11.1). (A checkpoint named after an
/// ending is rule 0's, `outcome_name`.)
fn targets(ir: &DialogueIr) -> Result<(), IrError> {
    if ir.checkpoint(&ir.start).is_none() {
        return Err(IrError::new(Rule::UnknownTarget, "start", format!("names no checkpoint: \"{}\"", ir.start)));
    }
    let dangling = edges(ir).find(|(_, e)| !ir.is_ending(&e.to) && ir.checkpoint(&e.to).is_none());
    match dangling {
        Some((path, e)) => {
            Err(IrError::new(Rule::UnknownTarget, member(&path, "to"), format!("names no checkpoint or ending: \"{}\"", e.to)))
        }
        None => Ok(()),
    }
}

/// Rule 3, for `may_disclose`. The facts a predicate reads are checked with
/// the predicate itself (rule 4), which names the predicate's path.
fn disclosed_facts(ir: &DialogueIr) -> Result<(), IrError> {
    for (i, checkpoint) in ir.checkpoints.iter().enumerate() {
        if let Some(k) = checkpoint.may_disclose.iter().position(|key| ir.fact(key).is_none()) {
            let path = index(&member(&index("checkpoints", i), "may_disclose"), k);
            return Err(IrError::new(Rule::UndeclaredFact, path, format!("unknown fact \"{}\"", checkpoint.may_disclose[k])));
        }
    }
    Ok(())
}

/// Rules 3 and 4 for every edge's predicate.
fn predicates(ir: &DialogueIr) -> Result<(), IrError> {
    for (path, e) in edges(ir) {
        if let Some(text) = &e.predicate {
            check_with_rules(text, &ir.facts, &member(&path, "predicate"))?;
        }
    }
    Ok(())
}

/// Rule 5: a fact's `default` matches its `type`.
pub(crate) fn default_types(facts: &[FactDecl]) -> Result<(), IrError> {
    for (i, fact) in facts.iter().enumerate() {
        let Some(default) = &fact.default else { continue };
        if default.fact_type() != fact.kind {
            let path = member(&index("facts", i), "default");
            return Err(IrError::new(Rule::DefaultType, path, format!("must be a {} to match the fact's type", type_name(fact.kind))));
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

fn warnings(ir: &DialogueIr) -> Vec<Warning> {
    let mut out: Vec<Warning> = unreachable(ir)
        .into_iter()
        .map(|i| Warning::new(WarningCode::CheckpointUnreachable, index("checkpoints", i), "is unreachable from `start`"))
        .collect();
    if no_ending_reachable(ir) {
        out.push(Warning::new(WarningCode::OutcomeUnreachable, "start", "no ending reachable"));
    }
    out.extend(validate_v2::stance_warnings(ir));
    header_text(ir, &mut out);
    for (i, checkpoint) in ir.checkpoints.iter().enumerate() {
        text(&mut out, member(&index("checkpoints", i), "goal"), &checkpoint.goal);
        checkpoint.label.iter().for_each(|label| text(&mut out, member(&index("checkpoints", i), "label"), label));
    }
    for (path, e) in edges(ir) {
        e.when.iter().for_each(|when| text(&mut out, member(&path, "when"), when));
    }
    out
}

/// The one-line warnings over the members both kinds share.
pub(crate) fn header_text(ir: &DialogueIr, out: &mut Vec<Warning>) {
    for (i, npc) in ir.npcs.iter().enumerate() {
        text(out, member(&index("npcs", i), "name"), &npc.name);
        npc.persona.iter().for_each(|persona| text(out, member(&index("npcs", i), "persona"), persona));
    }
    for (i, fact) in ir.facts.iter().enumerate() {
        if let Some(FactValue::String(default)) = &fact.default {
            text(out, member(&index("facts", i), "default"), default);
        }
    }
    for (i, ending) in ir.endings.iter().enumerate() {
        ending.label.iter().for_each(|label| text(out, member(&index("endings", i), "label"), label));
    }
}

fn text(out: &mut Vec<Warning>, path: String, value: &str) {
    if let Err(fault) = one_line(value) {
        out.push(Warning::new(WarningCode::Text(fault), path, fault.describe()));
    }
}
