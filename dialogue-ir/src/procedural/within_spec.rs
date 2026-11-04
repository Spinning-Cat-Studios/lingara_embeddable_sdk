//! The within-spec check (ADR 3.11.26g §8): the check 3.11.26i runs before it
//! answers a plan, and the one A·K2 runs again before it accepts. Each rule
//! refuses with its own code, because Backend counts rejections by rule. W12
//! (the wording) runs first; W6 and W13 run on the expansion, last.

use super::within_spec_rules as rules;
use super::{AtomWording, PlanFragment, ProceduralDialogue, expand_with};
use crate::disposition::Stance;
use crate::path::{index, member};
use crate::{IrError, Rule, validate};

/// What one check reads.
pub(super) struct Check<'a> {
    pub doc: &'a ProceduralDialogue,
    pub npc: &'a str,
    pub stance: Stance,
    pub fragment: &'a PlanFragment,
    /// `None` when there is nothing to check the ids against (a restored
    /// frontier that is the anchor alone).
    pub prefix: Option<String>,
}

/// A planned fragment: against the old frontier (`None` on a first plan), the
/// topics the core reports open, and the id prefix of this request.
// code-health: allow-arity — §8's signature: Backend (3.11.26i) and the core (A·K2) call it with exactly these seven inputs
pub fn within_spec(
    doc: &ProceduralDialogue,
    npc: &str,
    frontier: Option<&PlanFragment>,
    open_topics: &[&str],
    prefix: &str,
    fragment: &PlanFragment,
    wording: &AtomWording,
) -> Result<(), IrError> {
    wording.check()?;
    let check = Check { doc, npc, stance: stance(doc, npc), fragment, prefix: Some(prefix.into()) };
    rules::anchor(&check)?;
    rules::anchor_matches(&check, frontier)?;
    rules::atoms(&check)?;
    rules::params(&check)?;
    rules::references(&check)?;
    rules::topics_open(&check, open_topics)?;
    structure(&check, wording)
}

/// A saved frontier restored from a snapshot (§6): every rule but W1's
/// comparison with an old frontier and W5. The prefix is read from the saved
/// beats.
pub fn within_spec_restored(doc: &ProceduralDialogue, npc: &str, saved: &PlanFragment, wording: &AtomWording) -> Result<(), IrError> {
    wording.check()?;
    let check = Check { doc, npc, stance: stance(doc, npc), fragment: saved, prefix: saved_prefix(saved) };
    rules::anchor(&check)?;
    rules::atoms(&check)?;
    rules::params(&check)?;
    rules::references(&check)?;
    structure(&check, wording)
}

fn stance(doc: &ProceduralDialogue, npc: &str) -> Stance {
    Stance::of(doc.npc(npc).and_then(|n| n.disposition.as_ref()))
}

/// Everything before the last `_` of the first non-anchor beat's id.
fn saved_prefix(saved: &PlanFragment) -> Option<String> {
    let first = saved.beats.get(1)?;
    first.id.rfind('_').map(|at| first.id[..at].to_string())
}

/// W7–W11, then W6 and W13 on the expansion.
fn structure(check: &Check, wording: &AtomWording) -> Result<(), IrError> {
    rules::horizon(check)?;
    rules::tree(check)?;
    rules::ids(check)?;
    rules::exits(check)?;
    rules::text(check)?;
    let expanded = expand_with(check.doc, check.npc, check.fragment, wording)?;
    let pool = &check.doc.procedural.disclosable;
    for (i, checkpoint) in expanded.checkpoints.iter().enumerate() {
        if let Some(k) = checkpoint.may_disclose.iter().position(|key| !pool.contains(key)) {
            let path = index(&member(&index("checkpoints", i), "may_disclose"), k);
            return Err(IrError::new(Rule::PoolEscape, path, "discloses a fact outside `disclosable`"));
        }
    }
    validate(&expanded).map(|_| ())
}
