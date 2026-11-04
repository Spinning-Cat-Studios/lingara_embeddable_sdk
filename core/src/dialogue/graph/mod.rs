//! A dialogue that walks a developer's `dialogue.v1` or `dialogue.v2` graph (ADR 28.10.26f, 3.11.26g).
//! It rides the free-form session (`dialogue/mod.rs`): the same turn task,
//! the same window, one more piece of state. `runner` is the pure story,
//! `facts` what goes in and out of it, `session` the three commands,
//! `stored` a graph loaded by its store id (ADR 28.10.26i), and [`Step`]
//! what the runner answers.

mod facts;
pub(crate) mod runner;
mod session;
mod stored;

pub(crate) use facts::turn_fields;
pub(crate) use runner::GraphState;
pub(crate) use stored::{DialogueCache, Pending};

use lingara_dialogue_ir::{DialogueIr, Document, Warning};

use crate::shared::Shared;
use crate::types::{CrossedVia, DialogueId, EmbedEvent, LogLevel};

/// The target every graph log line carries.
pub(crate) const TARGET: &str = "lingara_embed::dialogue";

/// What one move of the story tells the game, in order.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Step {
    Reached { checkpoint: String, edge: Option<String>, via: CrossedVia },
    /// `checkpoint` is the one the story left.
    Ended { outcome: String, checkpoint: String, edge: Option<String> },
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
            Step::Ended { outcome, checkpoint, edge } => shared.outbox.push_later(EmbedEvent::DialogueEnded { dialogue, outcome, checkpoint, edge }),
            Step::Log { level, message } => shared.diag.emit(level, TARGET, message),
        }
    }
}

/// What a procedural document is refused with until the runtime that walks
/// one lands (ADR 3.11.26g §9; A·K2 removes it). It names no version.
pub(crate) const PROCEDURAL_NOT_YET: &str = "this kit cannot run procedural dialogues yet: upgrade lingara-embed";

/// K1's `load` for the core: either schema of static graph, and a procedural
/// document refused by [`PROCEDURAL_NOT_YET`] rather than by the crate's
/// "use load_document".
pub(crate) fn load_ir(ir_json: &str) -> Result<(DialogueIr, Vec<Warning>), String> {
    match lingara_dialogue_ir::load_document(ir_json).map_err(|e| e.to_string())? {
        (Document::Static(ir), warnings) => Ok((ir, warnings)),
        (Document::Procedural(_), _) => Err(PROCEDURAL_NOT_YET.into()),
    }
}

/// The one line a run logs when its graph carries a stance this kit does not
/// send yet (ADR 3.11.26g §9): `disposition` and `steer_to` load, and A·K2
/// sends them.
pub(crate) fn stance_notice(ir: &DialogueIr) -> Option<Step> {
    let stance = ir.npcs.iter().any(|n| n.disposition.is_some()) || ir.checkpoints.iter().any(|c| c.steer_to.is_some());
    stance.then(|| log(LogLevel::Info, "this dialogue's steering stance (`disposition`, `steer_to`) loads but is not sent yet".into()))
}
