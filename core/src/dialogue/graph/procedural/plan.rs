//! When a run asks the planner, and the one call it may have in flight
//! (ADR 3.11.26j D3, D4). A plan spends a metered unit whatever the answer,
//! so the core asks only at start, after a beat crossing, or on an exhausted
//! beat, at most once per beat, never retries, and turns planning off for a
//! refusal that would repeat. The body carries which topics and goals hold,
//! never a fact's value.

use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};

use super::termination::holds;
use super::{ProceduralRun, Slot, beat_of};
use crate::api::PlanAnswer;
use crate::dialogue::graph::{GraphState, emit};
use crate::dialogue::window::Window;
use crate::error::EmbedError;
use crate::runtime;
use crate::shared::Shared;
use crate::types::DialogueId;

/// How long the core waits for any plan, the first included: the route's
/// 10 s `PLAN_TIMEOUT` plus transport (D1). A constant, not a config field:
/// no game has asked to tune it. On expiry the call is `kept { timeout }`.
pub(crate) const PLAN_WAIT: Duration = Duration::from_secs(15);

/// What a call in flight was asked with: what D5 checks its answer against.
pub(crate) struct Asked {
    /// The beat it was asked from; `None` on `start`.
    pub beat: Option<String>,
    pub prefix: String,
    pub open_topics: Vec<String>,
}

impl Asked {
    /// The anchor a fresh answer has: the beat asked from, or `‹prefix›_0` on `start`.
    pub fn anchor(&self) -> String {
        self.beat.clone().unwrap_or_else(|| format!("{}_0", self.prefix))
    }
}

/// How a call ended, as the run reads it.
pub(crate) enum Landed {
    Planned(Value),
    Kept(String),
    Failed(EmbedError),
    TimedOut,
}

impl ProceduralRun {
    /// D3's triggers, once per beat and one call at a time: `exhausted` wins
    /// over `crossed`. A run with planning off, or ended, asks nothing.
    pub(super) fn next_plan(&mut self, graph: &GraphState, window: &Window) -> Option<Value> {
        if graph.ended() || self.plans_off || self.asked_here || !matches!(self.slot, Slot::Idle) {
            return None;
        }
        let reason = if self.exhausted(graph) {
            "exhausted"
        } else if self.crossed {
            "crossed"
        } else {
            return None;
        };
        Some(self.ask(reason, graph, window))
    }

    /// The current checkpoint is a sketch, or every exit of its beat leads to one.
    pub(super) fn exhausted(&self, graph: &GraphState) -> bool {
        let Some(beat) = self.frontier.beat(beat_of(&graph.current().id)) else { return false };
        let sketch = |id: &str| self.frontier.beat(id).is_some_and(|b| b.sketch);
        beat.sketch || (!beat.exits.is_empty() && beat.exits.iter().all(|exit| sketch(&exit.to)))
    }

    /// Books one call and answers its body (D4): the prefix advances whatever
    /// the answer, so a later call never reuses one. On `start`, `graph` is
    /// the seed's expansion with the session's facts set, read for its facts
    /// alone (D1), and the body has no `current` or `frontier`.
    pub(super) fn ask(&mut self, reason: &str, graph: &GraphState, window: &Window) -> Value {
        let prefix = format!("p{}", self.progress.next_prefix.max(1));
        self.progress.next_prefix = self.progress.next_prefix.max(1).saturating_add(1);
        self.progress.plans = self.progress.plans.saturating_add(1);
        (self.asked_here, self.crossed) = (true, false);
        let open_topics = self.open_topics(graph);
        let mut body = json!({
            "document": self.document,
            "npc": self.npc,
            "reason": reason,
            "id_prefix": prefix,
            "open_topics": open_topics,
            "goals_met": self.goals_met(graph),
            "visited": self.progress.visited,
            "turns": self.progress.turns,
            "beats": self.progress.beats,
            "history": window.entries(),
        });
        let beat = (reason != "start").then(|| beat_of(&graph.current().id).to_owned());
        if let Some(current) = &beat {
            body["current"] = json!(current);
            body["frontier"] = json!(self.frontier);
        }
        self.slot = Slot::InFlight(Asked { beat, prefix, open_topics });
        body
    }

    /// The topics whose `when` holds now and whose `npcs` admits the run NPC.
    fn open_topics(&self, graph: &GraphState) -> Vec<String> {
        let admits = |npcs: &Option<Vec<String>>| npcs.as_ref().is_none_or(|ids| ids.contains(&self.npc));
        let open = |when: &Option<String>| when.as_deref().is_none_or(|text| holds(graph, text));
        self.doc.procedural.topics.iter().filter(|t| admits(&t.npcs) && open(&t.when)).map(|t| t.id.clone()).collect()
    }

    fn goals_met(&self, graph: &GraphState) -> Vec<String> {
        self.doc.procedural.goals.iter().filter(|g| self.goal_holds(graph, &g.id)).map(|g| g.id.clone()).collect()
    }
}

/// One plan call, under the core's own [`PLAN_WAIT`], its answer applied
/// under the dialogue's lock. A late answer is dropped with the timer.
pub(crate) async fn run(shared: Arc<Shared>, dialogue: DialogueId, body: Value) {
    let landed = match tokio::time::timeout(PLAN_WAIT, shared.api.plan_dialogue(body)).await {
        Ok(Ok(PlanAnswer::Planned { fragment })) => Landed::Planned(fragment),
        Ok(Ok(PlanAnswer::Kept { reason })) => Landed::Kept(reason),
        Ok(Err(error)) => Landed::Failed(error),
        Err(_) => Landed::TimedOut,
    };
    let Some((steps, next)) = super::load::land(&shared, dialogue, landed) else { return };
    emit(&shared, dialogue, steps);
    if let Some(body) = next {
        spawn(&shared, dialogue, body);
    }
}

/// Starts the plan task for `body`.
pub(crate) fn spawn(shared: &Arc<Shared>, dialogue: DialogueId, body: Value) {
    runtime::spawn(shared, run(Arc::clone(shared), dialogue, body));
}
