//! Rule 0 (ADR 28.10.26f D2): the schema's values on a typed IR, so a
//! dialogue a builder made meets the bounds a parsed one does. Members and
//! their JSON types are `shape.rs`'s; every number here is `limits.rs`'s.

use crate::ir::{Checkpoint, DialogueIr, Disposition, Edge, Ending, FactDecl, FactType, FactValue, NpcLanguage, NpcProfile, SCHEMA_V1, SCHEMA_V2};
use crate::limits::*;
use crate::path::{index, member};
use crate::{IrError, Rule, is_ir_id};

/// Every bound, in document order. The first refusal wins.
pub(crate) fn check(ir: &DialogueIr) -> Result<(), IrError> {
    header(ir)?;
    id(&ir.start, "start")?;
    count(ir.checkpoints.len(), "checkpoints", 1, MAX_CHECKPOINTS)?;
    ir.checkpoints.iter().enumerate().try_for_each(|(i, c)| checkpoint(ir, c, &index("checkpoints", i)))
}

/// The members both kinds share (ADR 3.11.26g §9): the schema, the id and
/// title, the cast, the facts and the endings.
pub(crate) fn header(ir: &DialogueIr) -> Result<(), IrError> {
    if ir.schema != SCHEMA_V1 && ir.schema != SCHEMA_V2 {
        return Err(IrError::new(Rule::Bound, "schema", format!("must be \"{SCHEMA_V1}\" or \"{SCHEMA_V2}\"")));
    }
    id(&ir.id, "id")?;
    if let Some(title) = &ir.title {
        chars(title, "title", 1, MAX_TITLE_CHARS)?;
    }
    count(ir.npcs.len(), "npcs", 1, MAX_NPCS)?;
    ir.npcs.iter().enumerate().try_for_each(|(i, npc)| npc_profile(npc, &index("npcs", i)))?;
    count(ir.facts.len(), "facts", 0, MAX_FACTS)?;
    ir.facts.iter().enumerate().try_for_each(|(i, fact)| fact_decl(fact, &index("facts", i)))?;
    count(ir.endings.len(), "endings", 1, MAX_ENDINGS)?;
    ir.endings.iter().enumerate().try_for_each(|(i, ending)| self::ending(ending, &index("endings", i)))
}

/// §11.1: an id, an optional label, and `abandoned` never declared.
fn ending(ending: &Ending, path: &str) -> Result<(), IrError> {
    let id_path = member(path, "id");
    id(&ending.id, &id_path)?;
    if ending.id == ABANDONED {
        return Err(IrError::new(Rule::OutcomeName, id_path, "\"abandoned\" is built in, so it is never declared"));
    }
    match &ending.label {
        Some(label) => chars(label, &member(path, "label"), 1, MAX_TITLE_CHARS),
        None => Ok(()),
    }
}

/// §3: both members finite and within 0 to 1.
fn disposition(disposition: &Disposition, path: &str) -> Result<(), IrError> {
    for (name, value) in [("suggestibility", &disposition.suggestibility), ("initiative", &disposition.initiative)] {
        if !value.as_f64().is_some_and(|x| x.is_finite() && (0.0..=1.0).contains(&x)) {
            return Err(IrError::new(Rule::NumberRange, member(path, name), "must be a number from 0 to 1"));
        }
    }
    Ok(())
}

fn npc_profile(npc: &NpcProfile, path: &str) -> Result<(), IrError> {
    id(&npc.id, &member(path, "id"))?;
    chars(&npc.name, &member(path, "name"), 1, MAX_NPC_NAME_CHARS)?;
    if let Some(persona) = &npc.persona {
        chars(persona, &member(path, "persona"), 1, MAX_NPC_PERSONA_CHARS)?;
    }
    if let Some(d) = &npc.disposition {
        disposition(d, &member(path, "disposition"))?;
    }
    let Some(languages) = &npc.languages else { return Ok(()) };
    let path = member(path, "languages");
    count(languages.len(), &path, 1, MAX_NPC_LANGUAGES)?;
    languages.iter().enumerate().try_for_each(|(i, language)| npc_language(language, &index(&path, i)))
}

fn npc_language(language: &NpcLanguage, path: &str) -> Result<(), IrError> {
    if !is_language_code(&language.code) {
        let message = format!("must be a language tag such as \"es\" or \"zh-Hant\", at most {MAX_LANGUAGE_CODE_CHARS} characters");
        return Err(IrError::new(Rule::Bound, member(path, "code"), message));
    }
    for (name, level) in [("speaking", language.speaking), ("comprehension", language.comprehension)] {
        if !(MIN_LEVEL..=MAX_LEVEL).contains(&level) {
            return Err(IrError::new(Rule::NumberRange, member(path, name), format!("must be from {MIN_LEVEL} to {MAX_LEVEL}")));
        }
    }
    Ok(())
}

/// `^[a-z]{2,3}(-[A-Za-z0-9]{2,8}){0,2}$`, at most 16 characters (N1).
fn is_language_code(code: &str) -> bool {
    let mut parts = code.split('-');
    let primary = parts.next().unwrap_or_default();
    let primary_ok = (2..=3).contains(&primary.len()) && primary.bytes().all(|b| b.is_ascii_lowercase());
    let subtags: Vec<&str> = parts.collect();
    let subtag_ok = |s: &&str| (2..=8).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_alphanumeric());
    primary_ok && subtags.len() <= 2 && subtags.iter().all(subtag_ok) && code.len() <= MAX_LANGUAGE_CODE_CHARS
}

/// The key, and a `string` fact's string default held to the schema's
/// 1–120. A default of the wrong type is rule 5's.
fn fact_decl(fact: &FactDecl, path: &str) -> Result<(), IrError> {
    id(&fact.key, &member(path, "key"))?;
    match (&fact.kind, &fact.default) {
        (FactType::String, Some(FactValue::String(default))) => chars(default, &member(path, "default"), 1, MAX_FACT_STRING_CHARS),
        _ => Ok(()),
    }
}

/// A checkpoint's id shares one namespace with the endings and `abandoned`
/// (§11.1): v1's "a checkpoint id is not an outcome", for any ending set.
fn checkpoint(ir: &DialogueIr, checkpoint: &Checkpoint, path: &str) -> Result<(), IrError> {
    let id_path = member(path, "id");
    id(&checkpoint.id, &id_path)?;
    if ir.is_ending(&checkpoint.id) {
        let message = format!("\"{}\" is an ending, so it cannot name a checkpoint", checkpoint.id);
        return Err(IrError::new(Rule::OutcomeName, id_path, message));
    }
    chars(&checkpoint.goal, &member(path, "goal"), 1, MAX_GOAL_CHARS)?;
    if let Some(steer) = &checkpoint.steer_to {
        id(steer, &member(path, "steer_to"))?;
    }
    let disclose = member(path, "may_disclose");
    count(checkpoint.may_disclose.len(), &disclose, 0, MAX_DISCLOSE)?;
    for (k, key) in checkpoint.may_disclose.iter().enumerate() {
        id(key, &index(&disclose, k))?;
        if checkpoint.may_disclose[..k].contains(key) {
            return Err(IrError::new(Rule::DuplicateId, index(&disclose, k), format!("repeats \"{key}\"")));
        }
    }
    let edges = member(path, "edges");
    count(checkpoint.edges.len(), &edges, 0, MAX_EDGES_PER_CHECKPOINT)?;
    checkpoint.edges.iter().enumerate().try_for_each(|(j, e)| edge(e, &index(&edges, j)))
}

fn edge(edge: &Edge, path: &str) -> Result<(), IrError> {
    id(&edge.id, &member(path, "id"))?;
    id(&edge.to, &member(path, "to"))?;
    if let Some(predicate) = &edge.predicate {
        chars(predicate, &member(path, "predicate"), 1, MAX_PREDICATE_CHARS)?;
    }
    if let Some(when) = &edge.when {
        chars(when, &member(path, "when"), 1, MAX_CONDITION_CHARS)?;
    }
    Ok(())
}

pub(crate) fn id(value: &str, path: &str) -> Result<(), IrError> {
    if is_ir_id(value) { Ok(()) } else { Err(IrError::new(Rule::IdPattern, path, format!("\"{value}\" must match {ID_PATTERN}"))) }
}

/// Rule 7, on the canonical form (ADR 28.10.26g D2), `x-` members
/// included: the form `x-maxBytes` is defined over.
pub(crate) fn size(bytes: usize) -> Result<(), IrError> {
    if bytes <= MAX_IR_BYTES {
        return Ok(());
    }
    Err(IrError::new(Rule::Bound, "", format!("is {bytes} bytes in canonical form; at most {MAX_IR_BYTES}")))
}

/// A length in Unicode scalar values, as JSON Schema counts it, untrimmed.
pub(crate) fn chars(value: &str, path: &str, min: usize, max: usize) -> Result<(), IrError> {
    let n = value.chars().count();
    if (min..=max).contains(&n) { Ok(()) } else { Err(IrError::new(Rule::Bound, path, format!("is {n} characters; it must be {min} to {max}"))) }
}

pub(crate) fn count(n: usize, path: &str, min: usize, max: usize) -> Result<(), IrError> {
    if (min..=max).contains(&n) { Ok(()) } else { Err(IrError::new(Rule::Bound, path, format!("holds {n} entries; it must hold {min} to {max}"))) }
}
