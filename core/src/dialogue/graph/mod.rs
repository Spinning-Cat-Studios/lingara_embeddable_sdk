//! A dialogue that walks a developer's `dialogue.v1` graph (ADR 28.10.26f).
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

use crate::shared::Shared;
use crate::types::{CrossedVia, DialogueId, DialogueOutcome, EmbedEvent, LogLevel};

/// The target every graph log line carries.
pub(crate) const TARGET: &str = "lingara_embed::dialogue";

/// What one move of the story tells the game, in order.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Step {
    Reached { checkpoint: String, edge: Option<String>, via: CrossedVia },
    /// `checkpoint` is the one the story left.
    Ended { outcome: DialogueOutcome, checkpoint: String, edge: Option<String> },
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
