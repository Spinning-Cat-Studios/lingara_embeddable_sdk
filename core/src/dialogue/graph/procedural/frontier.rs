//! D5: an answer is applied only when it is fresh, re-reads into the crate's
//! own `PlanFragment`, and passes `within_spec` and `expand` here too, because
//! it crosses a trust boundary and a kit may meet a server of another crate
//! version. Anything else keeps the frontier the run has, or, on `start`,
//! falls back to the crate's `seed`. A replacement is the whole new frontier,
//! rebased onto the checkpoint the run sits on, never merged.

use std::sync::Arc;

use lingara_dialogue_ir::DialogueIr;
use lingara_dialogue_ir::procedural::{GAME_WORDING, PlanFragment, expand, seed, within_spec};

use super::plan::{Asked, Landed};
use super::{ProceduralRun, beat_of};
use crate::dialogue::graph::{GraphState, Step, log};
use crate::error::EmbedError;
use crate::types::{LogLevel, PlanKeptReason, PlanOutcome};

/// A refused answer: the reason the event carries, and the line the log does.
type Refused = (PlanKeptReason, Option<Step>);

fn kept(reason: PlanKeptReason) -> Step {
    Step::Planned { outcome: PlanOutcome::Kept, checkpoints: Vec::new(), reason: Some(reason) }
}

fn accepted(ir: &DialogueIr) -> Step {
    Step::Planned { outcome: PlanOutcome::Accepted, checkpoints: ir.checkpoints.iter().map(|c| c.id.clone()).collect(), reason: None }
}

/// The server's reason word, as the event spells it.
fn server_reason(word: &str) -> PlanKeptReason {
    match word {
        "timeout" => PlanKeptReason::Timeout,
        "unparseable" => PlanKeptReason::Unparseable,
        "rejected" => PlanKeptReason::Rejected,
        _ => PlanKeptReason::Unavailable,
    }
}

impl ProceduralRun {
    /// D5 over a running frontier: (1) fresh, (2) within spec, (3) replace,
    /// rebase and auto-run, through D6's crossing rules.
    pub(super) fn apply(&mut self, graph: &mut GraphState, asked: Asked, landed: Landed, steps: &mut Vec<Step>) {
        let here = graph.current().id.clone();
        let (fragment, ir) = match self.check(&asked, landed, Some(beat_of(&here))) {
            Ok(checked) => checked,
            Err((reason, line)) => {
                steps.extend(line);
                steps.push(kept(reason));
                return;
            }
        };
        let planned = accepted(&ir);
        if !graph.rebase(Arc::new(ir), &here) {
            steps.push(log(LogLevel::Warn, format!("a planned frontier was refused: it has no checkpoint `{here}` to stay on")));
            steps.push(kept(PlanKeptReason::Rejected));
            return;
        }
        self.frontier = fragment;
        self.progress.frontier_seq = self.progress.frontier_seq.saturating_add(1);
        self.progress.since_crossing = 0;
        steps.push(planned);
        let mut crossed = graph.auto_run();
        let moved = self.observe(&here, &mut crossed);
        steps.extend(crossed);
        let ending = self.after_crossing(graph, &moved);
        self.end(graph, ending, steps);
    }

    /// The first frontier (D1): a valid `planned` answer, else the seed. The
    /// run is still loading, so only the anchor id is checked for freshness.
    pub(super) fn first(&mut self, asked: Asked, landed: Landed) -> Result<(DialogueIr, Vec<Step>), EmbedError> {
        let (fragment, ir, steps) = match self.check(&asked, landed, None) {
            Ok((fragment, ir)) => {
                let planned = accepted(&ir);
                (fragment, ir, vec![planned])
            }
            Err((reason, line)) => {
                let fragment = seed(&self.doc, &self.npc, &asked.prefix);
                // The seed passes `within_spec` with no frontier; its one
                // refusal is the wording's, which `GAME_WORDING` passes.
                let ir = expand(&self.doc, &self.npc, &fragment).map_err(|e| EmbedError::invalid(e.to_string()))?;
                (fragment, ir, line.into_iter().chain([kept(reason)]).collect())
            }
        };
        self.frontier = fragment;
        self.progress.frontier_seq = 1;
        Ok((ir, steps))
    }

    /// D5's checks, in order, against what the call was asked with.
    fn check(&mut self, asked: &Asked, landed: Landed, current: Option<&str>) -> Result<(PlanFragment, DialogueIr), Refused> {
        let value = match landed {
            Landed::Planned(value) => value,
            Landed::Kept(word) => return Err((server_reason(&word), None)),
            Landed::TimedOut => return Err((PlanKeptReason::Timeout, Some(log(LogLevel::Warn, format!("a plan did not answer within {}s", super::plan::PLAN_WAIT.as_secs()))))),
            Landed::Failed(error) => return Err(self.failed(&error)),
        };
        // Stale only because the run moved while the call was in flight, so
        // the trigger that crossing dropped is asked next (D3). A wrong anchor
        // alone is the planner's fault, and refused like any other: were it
        // stale, a server answering from the wrong beat would be asked again,
        // a unit each time.
        let anchor = asked.anchor();
        if current.is_some_and(|beat| beat != anchor) {
            return Err((PlanKeptReason::Stale, None));
        }
        let rejected = |why: String| (PlanKeptReason::Rejected, Some(log(LogLevel::Warn, format!("a planned fragment was rejected: {why}"))));
        let fragment: PlanFragment = serde_json::from_value(value).map_err(|e| rejected(format!("it does not read as a fragment: {e}")))?;
        if fragment.anchor != anchor {
            return Err(rejected(format!("it is anchored on `{}`, not `{anchor}`", fragment.anchor)));
        }
        let open: Vec<&str> = asked.open_topics.iter().map(String::as_str).collect();
        let old = asked.beat.is_some().then_some(&self.frontier);
        within_spec(&self.doc, &self.npc, old, &open, &asked.prefix, &fragment, &GAME_WORDING).map_err(|e| rejected(format!("rule {}: {e}", e.rule.as_str())))?;
        // `within_spec` answers no graph, so the one the run walks is expanded
        // again; W13 has shown it passes `validate`.
        let ir = expand(&self.doc, &self.npc, &fragment).map_err(|e| rejected(format!("rule {}: {e}", e.rule.as_str())))?;
        Ok((fragment, ir))
    }

    /// A call that failed: always `unavailable`. A `400`, `403` or `404`
    /// would answer every later call the same, so it turns planning off (D3).
    fn failed(&mut self, error: &EmbedError) -> Refused {
        let repeats = matches!(error.status, Some(400 | 403 | 404));
        self.plans_off |= repeats;
        let tail = if repeats { "; planning is off for the rest of this run" } else { "" };
        (PlanKeptReason::Unavailable, Some(log(LogLevel::Warn, format!("a plan failed ({error}){tail}"))))
    }
}
