//! A dialogue that walks a developer's `dialogue.v1` or `dialogue.v2` graph (ADR 28.10.26f, 3.11.26g).
//! It rides the free-form session (`dialogue/mod.rs`): the same turn task,
//! the same window, one more piece of state. `runner` is the pure story,
//! `facts` what goes in and out of it, `session` the three commands,
//! `stored` a graph loaded by its store id (ADR 28.10.26i), and [`Step`]
//! what the runner answers.

mod facts;
pub(crate) mod procedural;
pub(crate) mod runner;
mod session;
mod stored;

pub(crate) use facts::turn_fields;
pub(crate) use procedural::ProceduralRun;
pub(crate) use runner::GraphState;
pub(crate) use stored::{DialogueCache, Pending};

use lingara_dialogue_ir::{Document, Warning};

use crate::shared::Shared;
use crate::types::{CrossedVia, DialogueId, EmbedEvent, LogLevel, PlanKeptReason, PlanOutcome, TracedBeat};

/// The target every graph log line carries.
pub(crate) const TARGET: &str = "lingara_embed::dialogue";

/// What one move of the story tells the game, in order.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Step {
    Reached { checkpoint: String, edge: Option<String>, via: CrossedVia },
    /// `checkpoint` is the one the story left; a procedural run adds its trace.
    Ended { outcome: String, checkpoint: String, edge: Option<String>, trace: Option<Vec<TracedBeat>> },
    /// A procedural run's plan landed (ADR 3.11.26j D9).
    Planned { outcome: PlanOutcome, checkpoints: Vec<String>, reason: Option<PlanKeptReason> },
    Log { level: LogLevel, message: String },
}

pub(crate) fn log(level: LogLevel, message: String) -> Step {
    Step::Log { level, message }
}

/// Queues `steps` in order: the two events, and each log line through the
/// core's diagnostics.
pub(crate) fn emit(shared: &Shared, dialogue: DialogueId, steps: Vec<Step>) {
    for step in steps {
        match step {
            Step::Reached { checkpoint, edge, via } => shared.outbox.push_later(EmbedEvent::CheckpointReached { dialogue, checkpoint, edge, via }),
            Step::Ended { outcome, checkpoint, edge, trace } => shared.outbox.push_later(EmbedEvent::DialogueEnded { dialogue, outcome, checkpoint, edge, trace }),
            Step::Planned { outcome, checkpoints, reason } => shared.outbox.push_later(EmbedEvent::DialoguePlanned { dialogue, outcome, checkpoints, reason }),
            Step::Log { level, message } => shared.diag.emit(level, TARGET, message),
        }
    }
}

/// K1's `load` for the core, over either kind (ADR 3.11.26g §1): a static
/// graph of either schema, or a procedural document, which this kit runs
/// since ADR 3.11.26j.
pub(crate) fn load_ir(ir_json: &str) -> Result<(Document, Vec<Warning>), String> {
    lingara_dialogue_ir::load_document(ir_json).map_err(|e| e.to_string())
}
