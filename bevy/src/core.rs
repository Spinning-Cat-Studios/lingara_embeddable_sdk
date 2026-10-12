//! The one seam between the plugin and `lingara-embed` (D12): a private
//! `Core` trait the `Lingara` resource holds. K1's `Embed` implements it;
//! the unit tests' scripted fake implements it too. Every command goes
//! through K1's own `execute(Command)`, so the seam is four methods wide.

use std::sync::Arc;

use bevy::ecs::resource::Resource;
use lingara_embed::{Command, CommandOutcome, Embed, EmbedConfig, EmbedError, EmbedEvent, EmbedStatus, EmbedTokenSource, ErrorKind};

pub(crate) trait Core: Send + Sync + 'static {
    fn drain(&self, max: u32) -> Vec<EmbedEvent>;
    fn status(&self) -> EmbedStatus;
    fn close(&self);
    fn execute(&self, command: Command) -> Result<CommandOutcome, EmbedError>;
    /// The real handle, for `Lingara::core()`. The test fake has none.
    fn embed(&self) -> Option<&Embed> {
        None
    }
}

impl Core for Embed {
    fn drain(&self, max: u32) -> Vec<EmbedEvent> {
        Embed::drain(self, max)
    }

    fn status(&self) -> EmbedStatus {
        Embed::status(self)
    }

    fn close(&self) {
        Embed::close(self);
    }

    fn execute(&self, command: Command) -> Result<CommandOutcome, EmbedError> {
        Embed::execute(self, command)
    }

    fn embed(&self) -> Option<&Embed> {
        Some(self)
    }
}

/// The live handle. Present from a successful start until the core's one
/// `Closed` has been drained. Dropping it is K1's `close()`, never a join, so
/// quitting a game never waits on the network.
#[derive(Resource)]
pub struct Lingara {
    core: Box<dyn Core>,
    closing: bool,
}

impl Lingara {
    pub(crate) fn new(core: Box<dyn Core>) -> Self {
        Self { core, closing: false }
    }

    /// The escape hatch: a K1 call this plugin does not wrap
    /// (`stream_pause`, `practice_snapshot`, `dialogue_snapshot`, `execute`),
    /// with the ids `NpcTranscript::id()` and `PracticeSession::id()` expose.
    /// Always `Some` in a game.
    pub fn core(&self) -> Option<&Embed> {
        self.core.embed()
    }

    pub(crate) fn handle(&self) -> &dyn Core {
        self.core.as_ref()
    }

    /// False once `close()` was asked for: the resource stays until `Closed`.
    pub fn is_live(&self) -> bool {
        !self.closing
    }

    pub(crate) fn close(&mut self) {
        if !self.closing {
            self.closing = true;
            self.core.close();
        }
    }

    /// One command, refused locally once the handle is closing.
    pub(crate) fn run(&self, command: Command) -> Result<CommandOutcome, EmbedError> {
        if self.closing {
            return Err(closed());
        }
        self.core.execute(command)
    }
}

/// What every inbound Message meets while no handle is live (D3).
pub(crate) fn closed() -> EmbedError {
    EmbedError::new(ErrorKind::Closed, "no Lingara handle is live: send LingaraConnect first")
}

/// The id a command answered with.
pub(crate) fn id_of(outcome: CommandOutcome) -> Result<u64, EmbedError> {
    match outcome {
        CommandOutcome::Id { id } => Ok(id),
        other => Err(EmbedError::new(ErrorKind::Internal, format!("expected an id, got {other:?}"))),
    }
}

/// How a handle is made: K1's `Embed::start` in a game, a fake in the tests.
pub(crate) type StartFn = Arc<dyn Fn(EmbedConfig, Option<Arc<dyn EmbedTokenSource>>) -> Result<Box<dyn Core>, EmbedError> + Send + Sync>;

pub(crate) fn start_embed() -> StartFn {
    Arc::new(|config, source| {
        let embed = match source {
            Some(source) => Embed::start_with_source(config, source)?,
            None => Embed::start(config)?,
        };
        Ok(Box::new(embed) as Box<dyn Core>)
    })
}
