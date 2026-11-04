//! What `dialogue.v2` adds to `validate` (ADR 3.11.26g §3, §9): the steering
//! target, the stance warnings, and the procedural kind's shared members
//! before its spec rules.

use crate::disposition::{Drive, Stance};
use crate::ir::{DialogueIr, SCHEMA_V2};
use crate::path::{index, member};
use crate::procedural::{ProceduralDialogue, check_spec, shared_view};
use crate::validate::{default_types, header_text, unique_header_ids};
use crate::validate_bounds::size;
use crate::{Document, IrError, Rule, Warning, WarningCode, validate_bounds};

/// §3: `steer_to` names an edge of the same checkpoint that has a `when`.
pub(crate) fn steer_targets(ir: &DialogueIr) -> Result<(), IrError> {
    for (i, checkpoint) in ir.checkpoints.iter().enumerate() {
        let Some(steer) = &checkpoint.steer_to else { continue };
        let ok = checkpoint.edges.iter().any(|edge| &edge.id == steer && edge.when.is_some());
        if !ok {
            let message = format!("names no edge of this checkpoint with a `when`: \"{steer}\"");
            return Err(IrError::new(Rule::SteerTarget, member(&index("checkpoints", i), "steer_to"), message));
        }
    }
    Ok(())
}

/// §3's warnings: a stance that contradicts itself, and a `steer_to` no cast
/// member drives hard enough to act on (warned once, at the first).
pub(crate) fn stance_warnings(ir: &DialogueIr) -> Vec<Warning> {
    let mut out = contradictions(ir);
    let all_low = ir.npcs.iter().all(|npc| Stance::of(npc.disposition.as_ref()).drive == Drive::Low);
    let first_steer = ir.checkpoints.iter().position(|checkpoint| checkpoint.steer_to.is_some());
    if let (true, Some(i)) = (all_low, first_steer) {
        let message = "no cast member's initiative is above Low, so no run acts on it";
        out.push(Warning::new(WarningCode::SteerWeak, member(&index("checkpoints", i), "steer_to"), message));
    }
    out
}

fn contradictions(ir: &DialogueIr) -> Vec<Warning> {
    let contradicts = |npc: &crate::NpcProfile| npc.disposition.is_some() && Stance::of(npc.disposition.as_ref()).contradicts();
    ir.npcs
        .iter()
        .enumerate()
        .filter(|(_, npc)| contradicts(npc))
        .map(|(i, _)| {
            let path = member(&index("npcs", i), "disposition");
            Warning::new(WarningCode::StanceContradiction, path, "railroad wins: detours are declined")
        })
        .collect()
}

/// A procedural document: the shared rules (cast, facts, endings), then §4's
/// spec rules, then the canonical size; its warnings are the shared ones.
pub(crate) fn procedural(doc: &ProceduralDialogue) -> Result<Vec<Warning>, IrError> {
    if doc.schema != SCHEMA_V2 {
        return Err(IrError::new(Rule::KindMismatch, "schema", format!("a procedural dialogue is always \"{SCHEMA_V2}\"")));
    }
    let view = shared_view(doc);
    validate_bounds::header(&view)?;
    unique_header_ids(&view)?;
    default_types(&doc.facts)?;
    check_spec(doc)?;
    size(Document::Procedural(doc.clone()).to_canonical_json().len())?;
    let mut warnings = contradictions(&view);
    header_text(&view, &mut warnings);
    Ok(warnings)
}
