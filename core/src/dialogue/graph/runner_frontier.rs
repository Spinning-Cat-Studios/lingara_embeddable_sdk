//! The runner's two procedural methods (ADR 3.11.26j D2, D6): a frontier
//! that is replaced, and a run its spec ends. Neither crosses an edge.

use std::sync::Arc;

use lingara_dialogue_ir::DialogueIr;

use super::GraphState;
use crate::dialogue::graph::Step;

impl GraphState {
    /// Ends an unfinished story on `outcome` with no edge: a close, or a
    /// procedural run's termination rule (ADR 3.11.26j D6).
    pub fn end_on(&mut self, outcome: &str) -> Option<Step> {
        if self.ended() {
            return None;
        }
        self.outcome = Some(outcome.to_owned());
        Some(Step::Ended { outcome: outcome.to_owned(), checkpoint: self.current().id.clone(), edge: None, trace: None })
    }

    /// Swaps in a new expansion and re-points the current checkpoint by id,
    /// keeping the facts and the outcome (ADR 3.11.26j D2). Refused, changing
    /// nothing, when `checkpoint` is not in `ir`. It never crosses an edge:
    /// the auto-run after it does, as after any fact change.
    pub fn rebase(&mut self, ir: Arc<DialogueIr>, checkpoint: &str) -> bool {
        let Some(at) = ir.checkpoints.iter().position(|c| c.id == checkpoint) else { return false };
        (self.ir, self.checkpoint) = (ir, at);
        true
    }
}
