//! The one seam between the binding and `lingara-embed` (ADR 1.10.26ab
//! D2, D10): K1's `Embed` implements it, and so does the tests' fake. Every
//! command goes through K1's own `execute(Command)`, so it is five methods
//! wide.

use lingara_embed::{Command, CommandOutcome, Embed, EmbedError, EmbedEvent, EmbedStatus};

pub trait Core: Send + Sync + 'static {
    fn drain(&self, max: u32) -> Vec<EmbedEvent>;
    fn status(&self) -> EmbedStatus;
    fn close(&self);
    fn close_and_wait(&self, timeout_ms: u32) -> bool;
    fn execute(&self, command: Command) -> Result<CommandOutcome, EmbedError>;
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

    fn close_and_wait(&self, timeout_ms: u32) -> bool {
        Embed::close_and_wait(self, timeout_ms)
    }

    fn execute(&self, command: Command) -> Result<CommandOutcome, EmbedError> {
        Embed::execute(self, command)
    }
}
