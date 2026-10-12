//! A scripted `Core`: queued events, every call recorded, practice
//! commands answered with a fixed snapshot. Its state is shared, so a test
//! still reads it after the pump owns the box.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard};

use lingara_embed::{CloseReason, Command, CommandOutcome, EmbedError, EmbedEvent, EmbedStatus, PracticePhase, PracticeSnapshot, StreamState};

use crate::pump::Core;

#[derive(Default)]
pub struct State {
    pub queue: VecDeque<EmbedEvent>,
    pub drains: Vec<u32>,
    pub closes: usize,
    /// Every `close_and_wait` timeout, in order.
    pub joins: Vec<u32>,
    pub commands: Vec<Command>,
    /// The phase practice commands answer with.
    pub phase: Option<PracticePhase>,
}

#[derive(Clone, Default)]
pub struct FakeCore(pub Arc<Mutex<State>>);

impl FakeCore {
    pub fn state(&self) -> MutexGuard<'_, State> {
        self.0.lock().expect("the fake's state")
    }
}

pub fn snapshot(phase: PracticePhase) -> PracticeSnapshot {
    PracticeSnapshot {
        phase,
        plan: None,
        set_index: 0,
        set_count: 1,
        question_index: 0,
        question_count: 1,
        question: None,
        last_result: None,
        failure: None,
    }
}

impl Core for FakeCore {
    fn drain(&self, max: u32) -> Vec<EmbedEvent> {
        let mut state = self.state();
        state.drains.push(max);
        let n = (max as usize).min(state.queue.len());
        state.queue.drain(..n).collect()
    }

    fn status(&self) -> EmbedStatus {
        EmbedStatus { stream: StreamState::Paused, cursor: None, needs_token: false, closed: false }
    }

    fn close(&self) {
        let mut state = self.state();
        state.closes += 1;
        state.queue.push_back(EmbedEvent::Closed { reason: CloseReason::Requested });
    }

    fn close_and_wait(&self, timeout_ms: u32) -> bool {
        self.state().joins.push(timeout_ms);
        true
    }

    fn execute(&self, command: Command) -> Result<CommandOutcome, EmbedError> {
        let mut state = self.state();
        state.commands.push(command.clone());
        Ok(match (&command, state.phase) {
            (Command::PracticeStart { .. }, _) => CommandOutcome::Id { id: 7 },
            (_, Some(phase)) => CommandOutcome::Practice { snapshot: Some(snapshot(phase)) },
            _ => CommandOutcome::Done {},
        })
    }
}
