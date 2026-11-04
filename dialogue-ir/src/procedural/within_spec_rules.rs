//! §8's rules W1–W5 and W7–W11, one function each (ADR 3.11.26g). Paths name
//! the fragment: `beats[2].exits[0].to`.

use std::collections::HashSet;

use super::atoms::{AGAIN, shape};
use super::spec_rules::line;
use super::within_spec::Check;
use super::{Beat, PlanFragment};
use crate::limits::{MAX_ANGLE_CHARS, MAX_BEAT_ID_CHARS, MAX_EXITS_PER_BEAT};
use crate::path::{beat, index, member};
use crate::{IrError, Rule};


fn err(rule: Rule, path: impl Into<String>, message: impl Into<String>) -> IrError {
    IrError::new(rule, path, message)
}

/// The beats with their paths.
fn beats<'a>(check: &'a Check) -> impl Iterator<Item = (String, &'a Beat)> {
    check.fragment.beats.iter().enumerate().map(|(i, b)| (beat(i), b))
}

/// W1: `beats[0]` is the anchor, at depth 0 and not a sketch, and `anchor`
/// names it.
pub(super) fn anchor(check: &Check) -> Result<(), IrError> {
    let Some(first) = check.fragment.beats.first() else {
        return Err(err(Rule::PlanAnchor, "beats", "holds no anchor"));
    };
    if first.id != check.fragment.anchor {
        return Err(err(Rule::PlanAnchor, "anchor", format!("must name beats[0], \"{}\"", first.id)));
    }
    if first.depth != 0 || first.sketch {
        return Err(err(Rule::PlanAnchor, beat(0), "the anchor is at depth 0 and is not a sketch"));
    }
    Ok(())
}

/// W1's second half: the same beat as the old frontier's (rebased), or on a
/// first plan a fresh Open named `‹prefix›_0`.
pub(super) fn anchor_matches(check: &Check, frontier: Option<&PlanFragment>) -> Result<(), IrError> {
    let first = &check.fragment.beats[0];
    match frontier {
        Some(old) => match old.beat(&first.id) {
            Some(was) if was.same_instance(first) => Ok(()),
            Some(_) => Err(err(Rule::PlanAnchor, beat(0), "differs from the frontier's beat of that id")),
            None => Err(err(Rule::PlanAnchor, beat(0), "is not a beat of the frontier")),
        },
        None => {
            let fresh = format!("{}_0", check.prefix.as_deref().unwrap_or_default());
            let ok = first.atom == super::AtomKind::Open && first.id == fresh;
            if ok { Ok(()) } else { Err(err(Rule::PlanAnchor, beat(0), format!("a first plan anchors on an Open named \"{fresh}\""))) }
        }
    }
}

/// W2: every atom is one the spec allows.
pub(super) fn atoms(check: &Check) -> Result<(), IrError> {
    let allowed: Vec<_> = check.doc.procedural.atoms.iter().map(|a| a.atom).collect();
    match beats(check).find(|(_, b)| !allowed.contains(&b.atom)) {
        Some((path, b)) => Err(err(Rule::PlanAtom, member(&path, "atom"), format!("\"{}\" is not in the spec's atoms", b.atom.as_str()))),
        None => Ok(()),
    }
}

/// W3: each beat's parameters are exactly its atom's set, sketches included.
pub(super) fn params(check: &Check) -> Result<(), IrError> {
    for (path, b) in beats(check) {
        let s = shape(b.atom);
        for (name, wanted, has) in [("topic", s.topic, b.topic.is_some()), ("goal", s.goal, b.goal.is_some()), ("ending", s.ending, b.ending.is_some())] {
            if wanted != has {
                let message = if wanted { "is required by this atom" } else { "is not a parameter of this atom" };
                return Err(err(Rule::PlanParams, member(&path, name), message));
            }
        }
    }
    Ok(())
}

/// W4: topics (beats' and detours') are spec topics this NPC may raise, a
/// Reveal's topic discloses something, goals are spec goals, endings declared.
pub(super) fn references(check: &Check) -> Result<(), IrError> {
    let doc = check.doc;
    let raisable = |id: &str| doc.topic(id).is_some_and(|t| t.npcs.as_ref().is_none_or(|n| n.iter().any(|x| x == check.npc)));
    for (path, b) in beats(check) {
        let topics = b.topic.iter().map(|t| (member(&path, "topic"), t));
        let detours = b.exits.iter().enumerate().filter_map(|(k, e)| Some((member(&index(&member(&path, "exits"), k), "detour"), e.detour.as_ref()?)));
        if let Some((at, t)) = topics.chain(detours).find(|(_, t)| !raisable(t)) {
            return Err(err(Rule::PlanReference, at, format!("\"{t}\" is not a topic this NPC may raise")));
        }
        if b.atom == super::AtomKind::Reveal && b.topic.as_ref().and_then(|t| doc.topic(t)).is_some_and(|t| t.discloses.is_empty()) {
            return Err(err(Rule::PlanReference, member(&path, "topic"), "a Reveal's topic discloses nothing"));
        }
        if let Some(goal) = b.goal.as_ref().filter(|g| doc.goal(g).is_none()) {
            return Err(err(Rule::PlanReference, member(&path, "goal"), format!("names no goal: \"{goal}\"")));
        }
        if let Some(ending) = b.ending.as_ref().filter(|e| !doc.is_ending(e)) {
            return Err(err(Rule::PlanReference, member(&path, "ending"), format!("names no ending: \"{ending}\"")));
        }
    }
    Ok(())
}

/// W5: a topic with a `when` appears only if the core reported it open.
pub(super) fn topics_open(check: &Check, open: &[&str]) -> Result<(), IrError> {
    let closed = |id: &String| check.doc.topic(id).is_some_and(|t| t.when.is_some()) && !open.contains(&id.as_str());
    for (path, b) in beats(check) {
        let named = b.topic.iter().map(|t| (member(&path, "topic"), t));
        let detours = b.exits.iter().enumerate().filter_map(|(k, e)| Some((member(&index(&member(&path, "exits"), k), "detour"), e.detour.as_ref()?)));
        if let Some((at, t)) = named.chain(detours).find(|(_, t)| closed(t)) {
            return Err(err(Rule::PlanTopicClosed, at, format!("\"{t}\" is not open")));
        }
    }
    Ok(())
}

/// W7: depths within the horizon, sketches exactly at `plan_depth + 1` and
/// with no exits, and no more beats than `max_nodes`.
pub(super) fn horizon(check: &Check) -> Result<(), IrError> {
    let h = &check.doc.procedural.horizon;
    let deepest = h.plan_depth + h.sketch_depth;
    if check.fragment.beats.len() > usize::from(h.max_nodes) {
        return Err(err(Rule::PlanHorizon, "beats", format!("holds {} beats; at most {}", check.fragment.beats.len(), h.max_nodes)));
    }
    for (path, b) in beats(check) {
        if b.depth > deepest {
            return Err(err(Rule::PlanHorizon, member(&path, "depth"), format!("is deeper than {deepest}")));
        }
        if b.sketch != (b.depth == h.plan_depth + 1) {
            return Err(err(Rule::PlanHorizon, member(&path, "sketch"), "a beat is a sketch exactly at depth plan_depth + 1"));
        }
        if b.sketch && !b.exits.is_empty() {
            return Err(err(Rule::PlanHorizon, member(&path, "exits"), "a sketch has no exits"));
        }
    }
    Ok(())
}

/// W8: a tree. Every non-anchor beat is the target of exactly one exit, from
/// a beat one level shallower, and no exit names an unknown beat.
pub(super) fn tree(check: &Check) -> Result<(), IrError> {
    let fragment = check.fragment;
    let mut parents = vec![0usize; fragment.beats.len()];
    for (path, b) in beats(check) {
        for (k, exit) in b.exits.iter().enumerate() {
            let at = member(&index(&member(&path, "exits"), k), "to");
            let Some(t) = fragment.beats.iter().position(|x| x.id == exit.to) else {
                return Err(err(Rule::PlanShape, at, format!("names no beat: \"{}\"", exit.to)));
            };
            if t == 0 || fragment.beats[t].depth != b.depth + 1 {
                return Err(err(Rule::PlanShape, at, "an exit leads one level deeper, never to the anchor"));
            }
            parents[t] += 1;
        }
    }
    match (1..parents.len()).find(|&i| parents[i] != 1) {
        Some(i) => Err(err(Rule::PlanShape, beat(i), format!("is the target of {} exits; exactly one", parents[i]))),
        None => Ok(()),
    }
}

/// W9: non-anchor ids carry the prefix and the beat-id pattern; every id is
/// unique and none ends in `_again`.
pub(super) fn ids(check: &Check) -> Result<(), IrError> {
    let mut seen = HashSet::new();
    for (i, (path, b)) in beats(check).enumerate() {
        let at = member(&path, "id");
        if !seen.insert(b.id.as_str()) {
            return Err(err(Rule::PlanIds, at, format!("repeats the id \"{}\"", b.id)));
        }
        if b.id.ends_with(AGAIN) {
            return Err(err(Rule::PlanIds, at, "a beat id never ends in \"_again\""));
        }
        let prefixed = check.prefix.as_ref().is_none_or(|p| b.id.starts_with(&format!("{p}_")));
        if i > 0 && !(prefixed && is_beat_id(&b.id)) {
            return Err(err(Rule::PlanIds, at, "must start with the request's prefix and match ^[a-z][a-z0-9_]{0,23}$"));
        }
    }
    Ok(())
}

fn is_beat_id(id: &str) -> bool {
    crate::is_ir_id(id) && id.len() <= MAX_BEAT_ID_CHARS
}

/// W10: a planned non-Close beat has one done exit and at most
/// min(its atom's cap, the stance's) detours, each onto a beat on the
/// detour's topic; a Close has no exits.
pub(super) fn exits(check: &Check) -> Result<(), IrError> {
    let cap_by_stance = check.stance.max_detours();
    for (path, b) in beats(check).filter(|(_, b)| !b.sketch) {
        let at = member(&path, "exits");
        let s = shape(b.atom);
        let done = b.exits.iter().filter(|e| e.detour.is_none()).count();
        let detours: Vec<_> = b.exits.iter().filter(|e| e.detour.is_some()).collect();
        if !s.done_exit && !b.exits.is_empty() {
            return Err(err(Rule::PlanExits, at, "a Close has no exits"));
        }
        if s.done_exit && done != 1 {
            return Err(err(Rule::PlanExits, at, format!("holds {done} done exits; exactly one")));
        }
        let cap = s.max_detours.min(cap_by_stance);
        if detours.len() > cap || b.exits.len() > MAX_EXITS_PER_BEAT {
            return Err(err(Rule::PlanExits, at, format!("holds {} detours; at most {cap} for this atom and stance", detours.len())));
        }
        let off_topic = detours.iter().find(|e| check.fragment.beat(&e.to).is_some_and(|t| t.topic != e.detour));
        if let Some(e) = off_topic {
            return Err(err(Rule::PlanExits, at, format!("the detour to \"{}\" leads to a beat on another topic", e.to)));
        }
    }
    Ok(())
}

/// W11: every `angle` is 1–60 characters on one visible line.
pub(super) fn text(check: &Check) -> Result<(), IrError> {
    for (path, b) in beats(check) {
        if let Some(angle) = &b.angle {
            line(angle, &member(&path, "angle"), MAX_ANGLE_CHARS).map_err(|e| err(Rule::PlanText, e.path, e.message))?;
        }
    }
    Ok(())
}
