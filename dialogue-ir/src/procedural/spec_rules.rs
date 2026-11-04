//! §4's seven spec rules (ADR 3.11.26g), run by `validate` on a procedural
//! document after the shared members, each refusing with its §9 code. Text
//! faults are refusals here, not warnings: the planner quotes the spec on
//! every call and no author reviews the result.

use super::{AtomKind, ProceduralDialogue, ProceduralSpec};
use crate::limits::*;
use crate::path::{index, member};
use crate::text::one_line;
use crate::predicate::check_with_rules as check_predicate;
use crate::validate::first_repeat;
use crate::validate_bounds::{chars, count, id};
use crate::{IrError, Rule};

const AT: &str = "procedural";

pub(crate) fn check(doc: &ProceduralDialogue) -> Result<(), IrError> {
    let spec = &doc.procedural;
    texts(spec)?;
    ids(doc)?;
    pool(doc)?;
    predicates(doc)?;
    atoms(spec)?;
    horizon(spec)?;
    endings(doc)
}

fn at(name: &str) -> String {
    member(AT, name)
}

/// Rule 7 with the list bounds: every text non-blank, within its bound and on
/// one visible line; every list within its count.
fn texts(spec: &ProceduralSpec) -> Result<(), IrError> {
    line(&spec.premise, &at("premise"), MAX_PREMISE_CHARS)?;
    count(spec.disclosable.len(), &at("disclosable"), 0, MAX_DISCLOSABLE)?;
    count(spec.topics.len(), &at("topics"), 1, MAX_TOPICS)?;
    for (i, topic) in spec.topics.iter().enumerate() {
        let path = index(&at("topics"), i);
        line(&topic.about, &member(&path, "about"), MAX_ABOUT_CHARS)?;
        count(topic.discloses.len(), &member(&path, "discloses"), 0, MAX_TOPIC_DISCLOSES)?;
        if let Some(when) = &topic.when {
            chars(when, &member(&path, "when"), 1, MAX_PREDICATE_CHARS)?;
        }
    }
    count(spec.forbidden.len(), &at("forbidden"), 0, MAX_FORBIDDEN)?;
    for (i, forbidden) in spec.forbidden.iter().enumerate() {
        line(&forbidden.about, &member(&index(&at("forbidden"), i), "about"), MAX_ABOUT_CHARS)?;
    }
    count(spec.goals.len(), &at("goals"), 0, MAX_GOALS)?;
    for (i, goal) in spec.goals.iter().enumerate() {
        let path = index(&at("goals"), i);
        line(&goal.describe, &member(&path, "describe"), MAX_DESCRIBE_CHARS)?;
        if let Some(when) = &goal.when {
            line(when, &member(&path, "when"), MAX_CONDITION_CHARS)?;
        }
        if let Some(predicate) = &goal.predicate {
            chars(predicate, &member(&path, "predicate"), 1, MAX_PREDICATE_CHARS)?;
        }
    }
    Ok(())
}

/// One visible line within `max`: a refusal with the text code, then the bound.
pub(crate) fn line(text: &str, path: &str, max: usize) -> Result<(), IrError> {
    one_line(text).map_err(|fault| IrError::new(Rule::of_text(fault), path, fault.describe()))?;
    chars(text, path, 1, max)
}

/// Rule 1: topic and goal ids match the pattern and are unique; every cast id
/// a topic or a forbidden entry names exists.
fn ids(doc: &ProceduralDialogue) -> Result<(), IrError> {
    let spec = &doc.procedural;
    for (i, topic) in spec.topics.iter().enumerate() {
        id(&topic.id, &member(&index(&at("topics"), i), "id"))?;
    }
    for (i, goal) in spec.goals.iter().enumerate() {
        id(&goal.id, &member(&index(&at("goals"), i), "id"))?;
    }
    first_repeat(spec.topics.iter().enumerate().map(|(i, t)| (member(&index(&at("topics"), i), "id"), t.id.as_str())))?;
    first_repeat(spec.goals.iter().enumerate().map(|(i, g)| (member(&index(&at("goals"), i), "id"), g.id.as_str())))?;
    let casts = spec.topics.iter().enumerate().map(|(i, t)| (index(&at("topics"), i), &t.npcs));
    let casts = casts.chain(spec.forbidden.iter().enumerate().map(|(i, f)| (index(&at("forbidden"), i), &f.npcs)));
    for (path, npcs) in casts {
        let Some(npcs) = npcs else { continue };
        if let Some(k) = npcs.iter().position(|npc| doc.npc(npc).is_none()) {
            return Err(IrError::new(Rule::UnknownTarget, index(&member(&path, "npcs"), k), format!("names no cast member: \"{}\"", npcs[k])));
        }
    }
    Ok(())
}

/// Rule 2: the pool and every `discloses` name declared facts, and each
/// `discloses` stays inside the pool (`pool_escape`).
fn pool(doc: &ProceduralDialogue) -> Result<(), IrError> {
    let spec = &doc.procedural;
    let declared = |key: &str| doc.facts.iter().any(|fact| fact.key == key);
    let undeclared = |key: &String, path: String| IrError::new(Rule::UndeclaredFact, path, format!("unknown fact \"{key}\""));
    if let Some(k) = spec.disclosable.iter().position(|key| !declared(key)) {
        return Err(undeclared(&spec.disclosable[k], index(&at("disclosable"), k)));
    }
    first_repeat(spec.disclosable.iter().enumerate().map(|(k, key)| (index(&at("disclosable"), k), key.as_str())))?;
    for (i, topic) in spec.topics.iter().enumerate() {
        let path = member(&index(&at("topics"), i), "discloses");
        for (k, key) in topic.discloses.iter().enumerate() {
            if !declared(key) {
                return Err(undeclared(key, index(&path, k)));
            }
            if !spec.disclosable.contains(key) {
                return Err(IrError::new(Rule::PoolEscape, index(&path, k), format!("\"{key}\" is not in `disclosable`")));
            }
        }
    }
    Ok(())
}

/// Rule 3: every topic `when` and goal `predicate` is a well-typed K1
/// predicate; a goal holds a `predicate` or a `when`.
fn predicates(doc: &ProceduralDialogue) -> Result<(), IrError> {
    let spec = &doc.procedural;
    for (i, topic) in spec.topics.iter().enumerate() {
        if let Some(when) = &topic.when {
            check_predicate(when, &doc.facts, &member(&index(&at("topics"), i), "when"))?;
        }
    }
    for (i, goal) in spec.goals.iter().enumerate() {
        let path = index(&at("goals"), i);
        if goal.predicate.is_none() && goal.when.is_none() {
            return Err(IrError::new(Rule::MissingMember, path, "holds neither a `predicate` nor a `when`"));
        }
        if let Some(predicate) = &goal.predicate {
            check_predicate(predicate, &doc.facts, &member(&path, "predicate"))?;
        }
    }
    Ok(())
}

/// Rule 4: unique, `open` and `close` both present; `practise` only with a
/// goal, `reveal` only with a topic that discloses something.
fn atoms(spec: &ProceduralSpec) -> Result<(), IrError> {
    let path = at("atoms");
    let kinds: Vec<AtomKind> = spec.atoms.iter().map(|a| a.atom).collect();
    if let Some(k) = (0..kinds.len()).find(|&k| kinds[..k].contains(&kinds[k])) {
        return Err(IrError::new(Rule::AtomSet, index(&path, k), format!("repeats \"{}\"", kinds[k].as_str())));
    }
    for required in [AtomKind::Open, AtomKind::Close] {
        if !kinds.contains(&required) {
            return Err(IrError::new(Rule::AtomSet, path, format!("must hold \"{}\"", required.as_str())));
        }
    }
    let usable = |kind: AtomKind| match kind {
        AtomKind::Practise => !spec.goals.is_empty(),
        AtomKind::Reveal => spec.topics.iter().any(|topic| !topic.discloses.is_empty()),
        _ => true,
    };
    match kinds.iter().position(|kind| !usable(*kind)) {
        Some(k) => Err(IrError::new(Rule::AtomSet, index(&path, k), format!("\"{}\" has nothing to work on", kinds[k].as_str()))),
        None => Ok(()),
    }
}

/// Rule 5: the horizon and the termination's numbers within their ranges.
fn horizon(spec: &ProceduralSpec) -> Result<(), IrError> {
    let h = &spec.horizon;
    let t = &spec.termination;
    let least_nodes = (1 + u32::from(h.plan_depth) + u32::from(h.sketch_depth)).max(3);
    let checks = [
        ("horizon.plan_depth", u32::from(h.plan_depth), 1, u32::from(MAX_PLAN_DEPTH)),
        ("horizon.sketch_depth", u32::from(h.sketch_depth), 0, u32::from(MAX_SKETCH_DEPTH)),
        ("horizon.max_nodes", u32::from(h.max_nodes), least_nodes, u32::from(MAX_FRONTIER_NODES)),
        ("termination.max_turns", u32::from(t.max_turns), 1, u32::from(MAX_TURNS)),
        ("termination.max_beats", u32::from(t.max_beats), 1, u32::from(MAX_BEATS)),
        ("termination.stall.turns", u32::from(t.stall.turns), u32::from(MIN_STALL_TURNS), u32::from(MAX_STALL_TURNS)),
    ];
    match checks.iter().find(|(_, n, min, max)| !(min..=max).contains(&n)) {
        Some((name, _, min, max)) => Err(IrError::new(Rule::NumberRange, at(name), format!("must be from {min} to {max}"))),
        None => Ok(()),
    }
}

/// Rule 6: the termination's endings are declared (or `abandoned`), and its
/// success goal is a goal.
fn endings(doc: &ProceduralDialogue) -> Result<(), IrError> {
    let t = &doc.procedural.termination;
    let mut named = vec![("termination.stall.ending", &t.stall.ending), ("termination.on_limit", &t.on_limit)];
    if let Some(success) = &t.success {
        if doc.goal(&success.goal).is_none() {
            return Err(IrError::new(Rule::UnknownTarget, at("termination.success.goal"), format!("names no goal: \"{}\"", success.goal)));
        }
        named.insert(0, ("termination.success.ending", &success.ending));
    }
    match named.into_iter().find(|(_, ending)| !doc.is_ending(ending)) {
        Some((name, ending)) => Err(IrError::new(Rule::UnknownTarget, at(name), format!("names no ending: \"{ending}\""))),
        None => Ok(()),
    }
}
