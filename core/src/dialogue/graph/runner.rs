//! The graph runner (ADR 28.10.26f D4), pure like `window.rs`: no I/O and no
//! lock. It holds where the story is and the facts the game set, and answers
//! with the [`Step`]s the session turns into events and log lines.
//!
//! An edge is *open* when it has no `predicate` or its predicate holds over
//! the facts. The open edges with a `when` are *offered* to the judge, in IR
//! order. The first open edge with no `when` is crossed alone, repeatedly;
//! load refused every cycle of such edges (D2 rule 6), so a run always ends,
//! and the `MAX_CHECKPOINTS` cap below is a defence against a bug, not a
//! designer's loop.

use std::collections::BTreeMap;
use std::sync::Arc;

use lingara_dialogue_ir::{Checkpoint, DialogueIr, Edge, FactValue, NpcProfile, limits, predicate};

use crate::dialogue::graph::{Step, facts, log};
use crate::types::{ABANDONED, CrossedVia, GraphSnapshot, LogLevel};

// What a procedural run adds to the runner (ADR 3.11.26j D2, D6): `rebase`
// and `end_on`, in a child so they reach the same private state.
#[path = "runner_frontier.rs"]
mod frontier;

pub(crate) struct GraphState {
    /// Validated.
    ir: Arc<DialogueIr>,
    /// The cast member this run speaks as.
    npc: usize,
    checkpoint: usize,
    /// The game's values; defaults are read from the IR.
    facts: BTreeMap<String, FactValue>,
    outcome: Option<String>,
}

impl GraphState {
    /// A run of `ir` (validated) as cast member `npc`, at `checkpoint`.
    pub fn new(ir: Arc<DialogueIr>, npc: usize, checkpoint: usize) -> Self {
        Self { ir, npc, checkpoint, facts: BTreeMap::new(), outcome: None }
    }

    pub fn ir(&self) -> &DialogueIr {
        &self.ir
    }

    pub fn npc(&self) -> &NpcProfile {
        &self.ir.npcs[self.npc]
    }

    pub fn current(&self) -> &Checkpoint {
        &self.ir.checkpoints[self.checkpoint]
    }

    pub fn set_facts(&self) -> &BTreeMap<String, FactValue> {
        &self.facts
    }

    pub fn ended(&self) -> bool {
        self.outcome.is_some()
    }

    fn open(&self, edge: &Edge) -> bool {
        let Some(text) = &edge.predicate else { return true };
        // A validated IR's predicates all parse; one that does not is closed.
        let Ok(parsed) = predicate::parse(text) else { return false };
        let lookup = |key: &str| predicate::value(&self.facts, &self.ir.facts, key);
        predicate::eval(&parsed, &lookup)
    }

    /// The open edges with a `when`, in IR order: what a turn sends.
    pub fn offered(&self) -> Vec<&Edge> {
        self.current().edges.iter().filter(|edge| edge.when.is_some() && self.open(edge)).collect()
    }

    /// `CheckpointReached { via: start }` where the run begins, then any auto crossings.
    pub fn start(&mut self) -> Vec<Step> {
        let mut steps = vec![Step::Reached { checkpoint: self.current().id.clone(), edge: None, via: CrossedVia::Start }];
        steps.extend(self.auto_run());
        steps
    }

    /// Crosses the first open edge with no `when`, in IR order, until none is
    /// open or the story ends.
    pub fn auto_run(&mut self) -> Vec<Step> {
        let mut steps = Vec::new();
        let mut crossed = 0;
        while !self.ended() {
            let Some(edge) = self.current().edges.iter().position(|e| e.when.is_none() && self.open(e)) else { break };
            if crossed == limits::MAX_CHECKPOINTS {
                let message = format!("an auto-run stopped after {crossed} crossings at `{}`: a cycle of edges without `when`, which load refuses, is a bug", self.current().id);
                steps.push(log(LogLevel::Error, message));
                break;
            }
            steps.push(self.cross(edge, CrossedVia::Auto));
            crossed += 1;
        }
        steps
    }

    /// Crosses the current checkpoint's `edge`-th edge.
    fn cross(&mut self, edge: usize, via: CrossedVia) -> Step {
        let ir = Arc::clone(&self.ir);
        let left = &ir.checkpoints[self.checkpoint];
        let edge = &left.edges[edge];
        if ir.is_ending(&edge.to) {
            let outcome = edge.to.clone();
            self.outcome = Some(outcome.clone());
            return Step::Ended { outcome, checkpoint: left.id.clone(), edge: Some(edge.id.clone()), trace: None };
        }
        match ir.checkpoints.iter().position(|c| c.id == edge.to) {
            Some(to) => self.checkpoint = to,
            None => return log(LogLevel::Error, format!("edge `{}` names no checkpoint `{}`, which load refuses", edge.id, edge.to)),
        }
        Step::Reached { checkpoint: edge.to.clone(), edge: Some(edge.id.clone()), via }
    }

    /// The judge's `transition { edge, to }`, applied after `done`: only for
    /// an edge the turn sent (`sent`), and always to the IR's own `to`.
    pub fn apply_judged(&mut self, sent: &[String], edge: &str, to: &str) -> Vec<Step> {
        let found = self.current().edges.iter().position(|e| e.id == edge);
        let Some(index) = found.filter(|_| !self.ended() && sent.iter().any(|s| s == edge)) else {
            return vec![log(LogLevel::Warn, format!("ignored a transition over edge `{edge}`, which this turn did not send"))];
        };
        let mut steps = Vec::new();
        let ours = &self.current().edges[index].to;
        if ours != to {
            steps.push(log(LogLevel::Warn, format!("the transition over `{edge}` named `{to}`; the dialogue's `{ours}` is used")));
        }
        steps.push(self.cross(index, CrossedVia::Judge));
        steps
    }

    /// A close of an unfinished story: `DialogueEnded { abandoned }`, with no edge.
    pub fn abandon(&mut self) -> Option<Step> {
        self.end_on(ABANDONED)
    }

    /// Stores the game's value for `key`, or says why it is refused.
    pub fn set_fact(&mut self, key: &str, value: FactValue) -> Result<(), String> {
        facts::check(&self.ir, key, &value)?;
        self.facts.insert(key.to_owned(), value);
        Ok(())
    }

    pub fn snapshot(&self) -> GraphSnapshot {
        GraphSnapshot { id: self.ir.id.clone(), npc: self.npc().id.clone(), checkpoint: self.current().id.clone(), facts: self.facts.clone(), outcome: self.outcome.clone() }
    }
}
