//! The node's behaviour with no Godot type in it (ADR 1.10.26ab D2, D3):
//! the pump, the token state, `last_error`, and the signals a call or a
//! frame owes the game. `embed_node.rs` emits `take_signals()` before every
//! `#[func]` returns, so a refusal's `error_raised` is seen before its
//! sentinel.

use std::sync::Arc;

use lingara_embed::{Command, CommandOutcome, EmbedConfig, EmbedError};
use serde_json::{Map, Value};

use crate::commands;
use crate::convert::{self, Finished, SignalCall};
use crate::pump::{Core, Library, Pump};
use crate::token::{Endpoint, Tokens};

/// What a method returns to GDScript, and so what its refusal looks like.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Returns {
    /// An id, `-1` when refused.
    Id,
    /// `true`, `false` when refused.
    Bool,
    /// Nothing.
    Unit,
    /// A practice snapshot, `{}` when refused; also emitted for this session.
    Practice(Option<u64>),
    /// A dialogue snapshot, `{}` when refused.
    Dialogue,
}

/// How a handle is made: K1's `Embed::start` in a game, a fake in tests.
pub type StartFn = Box<dyn Fn(EmbedConfig) -> Result<Box<dyn Core>, EmbedError> + Send + Sync>;

pub struct Binding {
    pump: Pump,
    start_fn: StartFn,
    pub tokens: Tokens,
    last_error: Option<EmbedError>,
    finished: Finished,
    pending: Vec<SignalCall>,
}

fn empty() -> Value {
    Value::Object(Map::new())
}

impl Binding {
    pub fn new(library: Arc<Library>, start_fn: StartFn) -> Self {
        Self { pump: Pump::new(library), start_fn, tokens: Tokens::default(), last_error: None, finished: Finished::default(), pending: Vec::new() }
    }

    /// The signals owed, in order. The node emits them before it returns.
    pub fn take_signals(&mut self) -> Vec<SignalCall> {
        std::mem::take(&mut self.pending)
    }

    pub fn is_running(&self) -> bool {
        self.pump.is_running()
    }

    /// The last refusal's `EmbedError`; `{}` after a call that succeeded.
    pub fn last_error(&self) -> Value {
        self.last_error.as_ref().map(convert::json).unwrap_or_else(empty)
    }

    pub fn status(&self) -> Value {
        self.pump.status().as_ref().map(convert::json).unwrap_or_else(empty)
    }

    fn refuse(&mut self, error: EmbedError) {
        self.pending.push(SignalCall::new("error_raised", vec![convert::json(&error)]));
        self.last_error = Some(error);
    }

    /// `start()`: builds K1's handle from `config`. `None` when refused;
    /// `Some(true)` when another node's handle was already live (D2's warning).
    pub fn start(&mut self, config: Result<EmbedConfig, EmbedError>) -> Option<bool> {
        self.last_error = None;
        match config.and_then(|c| (self.start_fn)(c)) {
            Ok(core) => {
                let another = self.pump.install(core);
                self.pending.push(SignalCall::new("started", Vec::new()));
                Some(another)
            }
            Err(e) => {
                self.refuse(e);
                None
            }
        }
    }

    pub fn stop(&mut self) {
        self.pump.stop();
    }

    /// One frame's events as signals.
    pub fn frame(&mut self, max: u32) {
        for event in self.pump.frame(max) {
            let calls = convert::signals_for(&event, &mut self.finished);
            self.pending.extend(calls);
        }
    }

    fn run(&mut self, command: Result<Command, EmbedError>) -> Result<CommandOutcome, EmbedError> {
        self.last_error = None;
        let core = self.pump.live().ok_or_else(convert::not_running)?;
        core.execute(command?)
    }

    /// A flat method: the command, its answer as GDScript sees it, and on a
    /// refusal D3's sentinel plus `error_raised` and `last_error`.
    pub fn call(&mut self, command: Result<Command, EmbedError>, returns: Returns) -> Value {
        match (self.run(command), returns) {
            (Err(e), _) => {
                self.refuse(e);
                sentinel(returns)
            }
            (Ok(CommandOutcome::Id { id }), _) => Value::from(id),
            (Ok(CommandOutcome::Practice { snapshot: Some(s) }), Returns::Practice(session)) => {
                if let Some(session) = session {
                    let calls = self.finished.practice(session, &s);
                    self.pending.extend(calls);
                }
                convert::json(&s)
            }
            (Ok(CommandOutcome::Dialogue { snapshot: Some(s) }), _) => convert::json(&s),
            (Ok(_), Returns::Bool) => Value::Bool(true),
            (Ok(_), Returns::Unit) => Value::Null,
            (Ok(_), _) => empty(),
        }
    }

    /// `execute(command)`: `{"ok": …}` or `{"error": …}`; a refusal also
    /// emits `error_raised` and sets `last_error`, as the flat path does.
    pub fn execute(&mut self, command: Value) -> Value {
        let command = commands::from_dictionary(command);
        let session = command.as_ref().ok().and_then(commands::session_of);
        let result = self.run(command);
        match &result {
            Err(e) => self.refuse(e.clone()),
            Ok(CommandOutcome::Practice { snapshot: Some(s) }) if session.is_some() => {
                let calls = self.finished.practice(session.unwrap_or_default(), s);
                self.pending.extend(calls);
            }
            Ok(_) => {}
        }
        convert::execute_answer(&result)
    }

    /// `set_token_endpoint`: kept before `start()`, K1's command after.
    pub fn set_token_endpoint(&mut self, url: &str, headers: Value) -> bool {
        self.last_error = None;
        match self.tokens.set_endpoint(self.pump.is_running(), url, headers) {
            Ok(Endpoint::Kept) => true,
            Ok(Endpoint::Send(command)) => self.call(Ok(command), Returns::Bool) == Value::Bool(true),
            Err(e) => {
                self.refuse(e);
                false
            }
        }
    }
}

/// D3's refusal values: `-1`, `false`, nothing, or `{}`.
pub fn sentinel(returns: Returns) -> Value {
    match returns {
        Returns::Id => Value::from(-1),
        Returns::Bool => Value::Bool(false),
        Returns::Unit => Value::Null,
        Returns::Practice(_) | Returns::Dialogue => empty(),
    }
}
