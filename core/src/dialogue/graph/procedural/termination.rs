//! D6: the spec's `termination`, enforced here because the core holds the
//! counts and the server holds nothing. The runner already ends a run on an
//! ending edge; these are the rules after it, in the table's order.

use lingara_dialogue_ir::predicate;

use super::{Moved, ProceduralRun};
use crate::dialogue::graph::GraphState;

impl ProceduralRun {
    /// After a crossing, or any idle change: the success goal, then
    /// `max_beats`. A goal with only a `when` has no predicate to read; its
    /// Practise beat's judged edge reaches it instead.
    pub(super) fn after_crossing(&self, graph: &GraphState, moved: &Moved) -> Option<String> {
        if graph.ended() {
            return None;
        }
        let termination = &self.doc.procedural.termination;
        if let Some(success) = termination.success.as_ref().filter(|s| self.goal_holds(graph, &s.goal)) {
            return Some(success.ending.clone());
        }
        (moved.crossed && self.progress.beats >= termination.max_beats).then(|| termination.on_limit.clone())
    }

    /// After a finished turn: `max_turns`, then the stall rule (turns since
    /// the last crossing or accepted replan).
    pub(super) fn after_turn(&self) -> Option<String> {
        let termination = &self.doc.procedural.termination;
        if self.progress.turns >= termination.max_turns {
            return Some(termination.on_limit.clone());
        }
        (self.progress.since_crossing >= u16::from(termination.stall.turns)).then(|| termination.stall.ending.clone())
    }

    /// Whether goal `id` has a predicate that holds over the run's facts.
    pub(super) fn goal_holds(&self, graph: &GraphState, id: &str) -> bool {
        let goal = self.doc.procedural.goals.iter().find(|g| g.id == id);
        goal.and_then(|g| g.predicate.as_deref()).is_some_and(|text| holds(graph, text))
    }
}

/// A K1 predicate over the facts the game set and their defaults; one that
/// does not parse, which validation refused, is false.
pub(super) fn holds(graph: &GraphState, text: &str) -> bool {
    let Ok(parsed) = predicate::parse(text) else { return false };
    let lookup = |key: &str| predicate::value(graph.set_facts(), &graph.ir().facts, key);
    predicate::eval(&parsed, &lookup)
}
