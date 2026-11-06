//! A procedural run (ADR 3.11.26j): a graph run over a frontier. The frontier
//! is the last `PlanFragment` the core accepted, and the unchanged runner
//! walks its expansion; what is new is only who writes the next few
//! checkpoints, and when. `plan` decides when to ask and runs the one call in
//! flight, `frontier` checks and applies an answer, `termination` ends the run
//! on its spec's rules, and `load` opens or restores one. Pure except for the
//! plan task, which follows `turn.rs`'s shape: one task, its answer applied
//! under the dialogue's lock.
//!
//! The runner itself stays in `Dialogue::graph`, as for a static graph, so a
//! turn, a fact and a snapshot read it unchanged; this holds the rest of the
//! run beside it.

mod frontier;
pub(crate) mod load;
pub(crate) mod plan;
mod termination;

use std::sync::Arc;

use lingara_dialogue_ir::procedural::{PlanFragment, ProceduralDialogue};
use serde_json::Value;

use crate::dialogue::graph::{GraphState, Step, log};
use crate::dialogue::window::Window;
use crate::types::{CrossedVia, LogLevel, ProceduralProgress, ProceduralSnapshot, TracedBeat, VisitedBeat};

use plan::{Asked, Landed};

/// The planner reads the last 16 beats crossed (3.11.26i's bound on the wire).
const VISITED_MAX: usize = 16;

/// Practise's second checkpoint; 3.11.26g W9 forbids a beat id ending in it.
const AGAIN: &str = "_again";

/// The beat a checkpoint belongs to: its id, with a trailing `_again` removed (D2).
pub(crate) fn beat_of(checkpoint: &str) -> &str {
    checkpoint.strip_suffix(AGAIN).unwrap_or(checkpoint)
}

/// The one plan a run may have outstanding (D3).
enum Slot {
    Idle,
    InFlight(Asked),
    /// An answer that landed during a turn, applied at its settle point.
    Held(Asked, Landed),
}

pub(crate) struct ProceduralRun {
    doc: Arc<ProceduralDialogue>,
    /// The document as the plan body sends it, in canonical form.
    document: Value,
    /// The cast member this run speaks as.
    npc: String,
    frontier: PlanFragment,
    /// `visited`, `trace` and the counters, in the shape a snapshot saves.
    progress: ProceduralProgress,
    /// A plan was already asked from the current beat (D3's once per beat).
    asked_here: bool,
    /// A beat crossing whose `crossed` trigger has not fired yet.
    crossed: bool,
    /// A `400`, `403` or `404` turned planning off for this run (D3).
    plans_off: bool,
    slot: Slot,
}

/// What one settle point did to the run.
#[derive(Default)]
struct Moved {
    crossed: bool,
    beat_crossed: bool,
}

impl ProceduralRun {
    fn new(doc: Arc<ProceduralDialogue>, npc: String, frontier: PlanFragment, progress: ProceduralProgress) -> Self {
        let document = canonical(&doc);
        Self { doc, document, npc, frontier, progress, asked_here: false, crossed: false, plans_off: false, slot: Slot::Idle }
    }

    pub fn snapshot(&self) -> ProceduralSnapshot {
        ProceduralSnapshot { frontier: serde_json::to_string(&self.frontier).unwrap_or_default(), progress: self.progress.clone() }
    }

    /// D3's settle point after a turn (`finished`) or an idle change, over the
    /// `steps` of its crossing and auto-run, `before` being the checkpoint it
    /// started on: steps (1)–(5). Answers the plan body to send, if any.
    pub fn settle(&mut self, graph: &mut GraphState, before: &str, steps: &mut Vec<Step>, finished: bool, window: &Window) -> Option<Value> {
        if finished {
            self.progress.turns = self.progress.turns.saturating_add(1);
        }
        let moved = self.observe(before, steps);
        if finished && !moved.crossed {
            self.progress.since_crossing = self.progress.since_crossing.saturating_add(1);
        }
        let ending = self.after_crossing(graph, &moved).or_else(|| if finished { self.after_turn() } else { None });
        if self.end(graph, ending, steps) {
            return None;
        }
        // Only a held answer is taken; a call still in flight stays in its slot.
        match std::mem::replace(&mut self.slot, Slot::Idle) {
            Slot::Held(asked, landed) => self.apply(graph, asked, landed, steps),
            other => self.slot = other,
        }
        self.next_plan(graph, window)
    }

    /// An answer that landed while no turn is in flight: (4), then (3) and (5).
    /// During a turn it is held for the settle point instead.
    pub fn land(&mut self, graph: &mut GraphState, landed: Landed, turning: bool, window: &Window) -> (Vec<Step>, Option<Value>) {
        let Slot::InFlight(asked) = std::mem::replace(&mut self.slot, Slot::Idle) else { return (Vec::new(), None) };
        if graph.ended() {
            return (Vec::new(), None);
        }
        if turning {
            self.slot = Slot::Held(asked, landed);
            return (Vec::new(), None);
        }
        let mut steps = Vec::new();
        self.apply(graph, asked, landed, &mut steps);
        if graph.ended() {
            return (steps, None);
        }
        (steps, self.next_plan(graph, window))
    }

    /// A close of an unfinished run: `abandoned`, with its trace.
    pub fn abandon(&mut self, graph: &mut GraphState) -> Option<Step> {
        let mut steps: Vec<Step> = graph.abandon().into_iter().collect();
        let before = graph.current().id.clone();
        self.observe(&before, &mut steps);
        steps.pop()
    }

    /// Ends the run on `ending`, if a rule chose one, and drops any held answer.
    fn end(&mut self, graph: &mut GraphState, ending: Option<String>, steps: &mut Vec<Step>) -> bool {
        if let Some(ending) = ending {
            let before = graph.current().id.clone();
            let mut ended: Vec<Step> = graph.end_on(&ending).into_iter().collect();
            self.observe(&before, &mut ended);
            steps.extend(ended);
        }
        if graph.ended() {
            self.slot = Slot::Idle;
        }
        graph.ended()
    }

    /// Books every crossing in `steps` (D2, D13) and stamps each ending with
    /// the trace.
    fn observe(&mut self, before: &str, steps: &mut Vec<Step>) -> Moved {
        let mut moved = Moved::default();
        let mut here = before.to_owned();
        let mut refused = Vec::new();
        for step in steps.iter_mut() {
            match step {
                Step::Reached { checkpoint, edge: Some(edge), via } => {
                    moved.crossed = true;
                    self.progress.since_crossing = 0;
                    if beat_of(checkpoint) != beat_of(&here) {
                        moved.beat_crossed = true;
                        refused.extend(self.cross_beat(beat_of(&here).to_owned(), edge.clone(), beat_of(checkpoint).to_owned(), *via));
                    }
                    here.clone_from(checkpoint);
                }
                Step::Ended { edge, trace, .. } => {
                    self.close_entry(edge.clone());
                    *trace = Some(self.progress.trace.clone());
                }
                _ => {}
            }
        }
        steps.extend(refused);
        moved
    }

    fn cross_beat(&mut self, left: String, edge: String, entered: String, via: CrossedVia) -> Option<Step> {
        if let Some(beat) = self.frontier.beat(&left) {
            self.progress.visited.push(VisitedBeat { atom: beat.atom, topic: beat.topic.clone() });
            if self.progress.visited.len() > VISITED_MAX {
                self.progress.visited.remove(0);
            }
        }
        self.progress.beats = self.progress.beats.saturating_add(1);
        (self.asked_here, self.crossed) = (false, true);
        self.close_entry(Some(edge));
        self.enter(&entered, via)
    }

    /// Appends the entry for a beat the run enters (D13). An append past
    /// `max_beats + 1` can only follow a broken D6: it is refused, and says so.
    fn enter(&mut self, beat: &str, via: CrossedVia) -> Option<Step> {
        let bound = usize::from(self.doc.procedural.termination.max_beats) + 1;
        debug_assert!(self.progress.trace.len() < bound, "the trace outgrew max_beats + 1: termination is broken");
        if self.progress.trace.len() >= bound {
            return Some(log(LogLevel::Error, format!("a trace entry for `{beat}` was refused: the trace already holds max_beats + 1 ({bound})")));
        }
        let found = self.frontier.beat(beat);
        self.progress.trace.push(TracedBeat {
            id: beat.to_owned(),
            atom: found.map(|b| b.atom),
            topic: found.and_then(|b| b.topic.clone()),
            goal: found.and_then(|b| b.goal.clone()),
            ending: found.and_then(|b| b.ending.clone()),
            left_by_edge: None,
            via,
            entered_turn: self.progress.turns,
            left_turn: None,
            plan_seq: self.progress.frontier_seq,
        });
        None
    }

    /// Writes the current entry's way out, once.
    fn close_entry(&mut self, edge: Option<String>) {
        let turns = self.progress.turns;
        if let Some(entry) = self.progress.trace.last_mut().filter(|e| e.left_turn.is_none()) {
            (entry.left_by_edge, entry.left_turn) = (edge, Some(turns));
        }
    }

}

/// The document as a plan body carries it: the crate's canonical text, read back as JSON.
fn canonical(doc: &ProceduralDialogue) -> Value {
    let text = lingara_dialogue_ir::Document::Procedural(doc.clone()).to_canonical_json();
    serde_json::from_str(&text).unwrap_or(Value::Null)
}
