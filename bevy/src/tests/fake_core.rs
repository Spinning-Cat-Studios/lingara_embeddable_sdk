//! The scripted fake `Core` (D12): it records every call, answers commands
//! from a script, and drains whatever a test queued. It never blocks, so a
//! test with nothing queued is a core whose I/O is stalled indefinitely.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, MutexGuard};

use bevy::app::App;
use bevy::ecs::message::{Message, Messages};
use lingara_embed::{
    Command, CommandOutcome, EmbedConfig, EmbedError, EmbedEvent, EmbedStatus, ErrorKind, PracticePhase, PracticeSnapshot, StreamState,
};

use crate::config::LingaraConfig;
use crate::core::{Core, StartFn};
use crate::plugin::LingaraPlugin;

#[derive(Default)]
pub struct State {
    pub queue: VecDeque<EmbedEvent>,
    pub commands: Vec<Command>,
    pub drains: Vec<u32>,
    pub closes: u32,
    pub starts: Vec<EmbedConfig>,
    pub start_error: Option<EmbedError>,
    /// A command's `type` → the error it is refused with, every time.
    pub refuse: HashMap<String, EmbedError>,
    /// What the practice calls answer, in order; then `Unanswered`.
    pub snapshots: VecDeque<PracticeSnapshot>,
    pub stream: Option<StreamState>,
    next_id: u64,
}

#[derive(Clone, Default)]
pub struct Fake(pub Arc<Mutex<State>>);

pub fn snapshot(phase: PracticePhase) -> PracticeSnapshot {
    PracticeSnapshot {
        phase,
        plan: None,
        set_index: 0,
        set_count: 1,
        question_index: 0,
        question_count: 2,
        question: None,
        last_result: None,
        failure: None,
    }
}

pub fn error(kind: ErrorKind) -> EmbedError {
    EmbedError::new(kind, "scripted")
}

/// A command's serde tag: `dialogue_open`, `practice_start`, …
pub fn kind(command: &Command) -> String {
    serde_json::to_value(command).ok().and_then(|v| v["type"].as_str().map(str::to_owned)).unwrap_or_default()
}

impl Fake {
    pub fn state(&self) -> MutexGuard<'_, State> {
        self.0.lock().unwrap()
    }

    pub fn push(&self, event: EmbedEvent) {
        self.state().queue.push_back(event);
    }

    pub fn refuse(&self, command: &str, kind: ErrorKind) {
        self.state().refuse.insert(command.into(), error(kind));
    }

    /// The commands of one kind, in order.
    pub fn sent(&self, command: &str) -> Vec<Command> {
        self.state().commands.iter().filter(|c| kind(c) == command).cloned().collect()
    }

    pub fn starter(&self) -> StartFn {
        let fake = self.clone();
        Arc::new(move |config, _source| {
            let mut state = fake.state();
            state.starts.push(config);
            match state.start_error.clone() {
                Some(error) => Err(error),
                None => Ok(Box::new(fake.clone()) as Box<dyn Core>),
            }
        })
    }
}

impl Core for Fake {
    fn drain(&self, max: u32) -> Vec<EmbedEvent> {
        let mut state = self.state();
        state.drains.push(max);
        let n = state.queue.len().min(max as usize);
        state.queue.drain(..n).collect()
    }

    fn status(&self) -> EmbedStatus {
        let state = self.state();
        EmbedStatus { stream: state.stream.clone().unwrap_or(StreamState::Paused), cursor: None, needs_token: false, closed: state.closes > 0 }
    }

    fn close(&self) {
        self.state().closes += 1;
    }

    fn execute(&self, command: Command) -> Result<CommandOutcome, EmbedError> {
        let mut state = self.state();
        state.commands.push(command.clone());
        if let Some(error) = state.refuse.get(&kind(&command)) {
            return Err(error.clone());
        }
        Ok(match command {
            Command::SendContext { .. } | Command::PracticeStart { .. } | Command::DialogueOpen { .. } | Command::DialogueLoad { .. } | Command::DialogueSay { .. } => {
                state.next_id += 1;
                CommandOutcome::Id { id: state.next_id }
            }
            Command::PracticeSnapshot { .. } => CommandOutcome::Practice { snapshot: Some(snapshot(PracticePhase::Requesting)) },
            c if kind(&c).starts_with("practice_") => {
                let next = state.snapshots.pop_front().unwrap_or_else(|| snapshot(PracticePhase::Unanswered));
                CommandOutcome::Practice { snapshot: Some(next) }
            }
            _ => CommandOutcome::Done {},
        })
    }
}

/// An app with the plugin over `fake`, and nothing else.
pub fn app(config: LingaraConfig, fake: &Fake) -> App {
    let mut app = App::new();
    app.add_plugins(LingaraPlugin::with_start(config, fake.starter()));
    app
}

/// Every `M` written since the last call.
pub fn read<M: Message>(app: &mut App) -> Vec<M> {
    app.world_mut().resource_mut::<Messages<M>>().drain().collect()
}

pub fn write<M: Message>(app: &mut App, message: M) {
    app.world_mut().write_message(message);
}
