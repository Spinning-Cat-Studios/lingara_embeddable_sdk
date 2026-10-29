//! Rule 0 (ADR 28.10.26f D2): the schema's values on a typed IR, so a
//! dialogue a builder made meets the bounds a parsed one does. Members and
//! their JSON types are `shape.rs`'s; every number here is `limits.rs`'s.

use crate::ir::{Checkpoint, DialogueIr, Edge, FactDecl, FactType, FactValue, NpcLanguage, NpcProfile, SCHEMA_V1};
use crate::limits::*;
use crate::path::{index, member};
use crate::{IrError, is_ir_id};

/// Every bound, in document order. The first refusal wins.
pub(crate) fn check(ir: &DialogueIr) -> Result<(), IrError> {
    if ir.schema != SCHEMA_V1 {
        return Err(IrError::new("schema", format!("must be \"{SCHEMA_V1}\"")));
    }
    id(&ir.id, "id")?;
    if let Some(title) = &ir.title {
        chars(title, "title", 1, MAX_TITLE_CHARS)?;
    }
    count(ir.npcs.len(), "npcs", 1, MAX_NPCS)?;
    ir.npcs.iter().enumerate().try_for_each(|(i, npc)| npc_profile(npc, &index("npcs", i)))?;
    count(ir.facts.len(), "facts", 0, MAX_FACTS)?;
    ir.facts.iter().enumerate().try_for_each(|(i, fact)| fact_decl(fact, &index("facts", i)))?;
    id(&ir.start, "start")?;
    count(ir.checkpoints.len(), "checkpoints", 1, MAX_CHECKPOINTS)?;
    ir.checkpoints.iter().enumerate().try_for_each(|(i, c)| checkpoint(c, &index("checkpoints", i)))
}

fn npc_profile(npc: &NpcProfile, path: &str) -> Result<(), IrError> {
    id(&npc.id, &member(path, "id"))?;
    chars(&npc.name, &member(path, "name"), 1, MAX_NPC_NAME_CHARS)?;
    if let Some(persona) = &npc.persona {
        chars(persona, &member(path, "persona"), 1, MAX_NPC_PERSONA_CHARS)?;
    }
    let Some(languages) = &npc.languages else { return Ok(()) };
    let path = member(path, "languages");
    count(languages.len(), &path, 1, MAX_NPC_LANGUAGES)?;
    languages.iter().enumerate().try_for_each(|(i, language)| npc_language(language, &index(&path, i)))
}

fn npc_language(language: &NpcLanguage, path: &str) -> Result<(), IrError> {
    if !is_language_code(&language.code) {
        let message = format!("must be a language tag such as \"es\" or \"zh-Hant\", at most {MAX_LANGUAGE_CODE_CHARS} characters");
        return Err(IrError::new(member(path, "code"), message));
    }
    for (name, level) in [("speaking", language.speaking), ("comprehension", language.comprehension)] {
        if !(MIN_LEVEL..=MAX_LEVEL).contains(&level) {
            return Err(IrError::new(member(path, name), format!("must be from {MIN_LEVEL} to {MAX_LEVEL}")));
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

fn checkpoint(checkpoint: &Checkpoint, path: &str) -> Result<(), IrError> {
    let id_path = member(path, "id");
    id(&checkpoint.id, &id_path)?;
    if OUTCOMES.contains(&checkpoint.id.as_str()) {
        return Err(IrError::new(id_path, format!("\"{}\" is an outcome, so it cannot name a checkpoint", checkpoint.id)));
    }
    chars(&checkpoint.goal, &member(path, "goal"), 1, MAX_GOAL_CHARS)?;
    let disclose = member(path, "may_disclose");
    count(checkpoint.may_disclose.len(), &disclose, 0, MAX_DISCLOSE)?;
    for (k, key) in checkpoint.may_disclose.iter().enumerate() {
        id(key, &index(&disclose, k))?;
        if checkpoint.may_disclose[..k].contains(key) {
            return Err(IrError::new(index(&disclose, k), format!("repeats \"{key}\"")));
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

fn id(value: &str, path: &str) -> Result<(), IrError> {
    if is_ir_id(value) { Ok(()) } else { Err(IrError::new(path, format!("\"{value}\" must match {ID_PATTERN}"))) }
}

/// A length in Unicode scalar values, as JSON Schema counts it, untrimmed.
fn chars(value: &str, path: &str, min: usize, max: usize) -> Result<(), IrError> {
    let n = value.chars().count();
    if (min..=max).contains(&n) { Ok(()) } else { Err(IrError::new(path, format!("is {n} characters; it must be {min} to {max}"))) }
}

fn count(n: usize, path: &str, min: usize, max: usize) -> Result<(), IrError> {
    if (min..=max).contains(&n) { Ok(()) } else { Err(IrError::new(path, format!("holds {n} entries; it must hold {min} to {max}"))) }
}
