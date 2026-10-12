//! `EmbedHandle`, the one UniFFI object (ADR 1.10.26ae D2): K1's `Embed`,
//! its flat methods one for one. The commands are spread over this file
//! (lifecycle, polling, tokens and the stream), `practice.rs` and
//! `dialogue.rs`, each its own `#[uniffi::export] impl` block.
//!
//! K1's `close` and `close_and_wait` are `shutdown` and `shutdown_and_wait`
//! here: UniFFI gives every object its own `close()`, which frees the Rust
//! side, and an exported `close` would clash with it.

use std::sync::Arc;

use lingara_embed as core;

use crate::convert::{event, fail, known, narrow, to_core};
use crate::types::{Command, CommandOutcome, EmbedConfig, EmbedEvent, EmbedException, EmbedStatus, Header, SecretString};

pub(crate) type Outcome<T> = Result<T, EmbedException>;

/// One player's connection to Lingara. The game reaches it through
/// `LingaraEmbed`; every method may be called from any thread.
#[derive(uniffi::Object)]
pub struct EmbedHandle {
    pub(crate) embed: core::Embed,
}

#[uniffi::export]
impl EmbedHandle {
    /// Validates `config`, starts the core's I/O thread and, unless
    /// `stream_on_start` is false, opens the stream.
    #[uniffi::constructor]
    pub fn start(config: EmbedConfig) -> Outcome<Arc<Self>> {
        let config: core::EmbedConfig = to_core("config", &config)?;
        core::Embed::start(config).map(|embed| Arc::new(Self { embed })).map_err(fail)
    }

    pub fn status(&self) -> Outcome<EmbedStatus> {
        known(&self.embed.status())
    }

    /// Signals the I/O thread and returns at once: one `Closed` follows.
    pub fn shutdown(&self) {
        self.embed.close();
    }

    /// `shutdown`, then joins the I/O thread for at most `timeout_ms`; true
    /// when it exited in time.
    pub fn shutdown_and_wait(&self, timeout_ms: i64) -> bool {
        let timeout = u32::try_from(timeout_ms.max(0)).unwrap_or(u32::MAX);
        self.embed.close_and_wait(timeout)
    }

    pub fn poll_event(&self) -> Option<EmbedEvent> {
        self.embed.poll_event().as_ref().map(event)
    }

    /// At most `max` events, in queue order, in one crossing.
    pub fn drain(&self, max: i64) -> Vec<EmbedEvent> {
        let max = u32::try_from(max.max(0)).unwrap_or(u32::MAX);
        self.embed.drain(max).iter().map(event).collect()
    }

    /// Any command, dispatched exactly as its flat method.
    pub fn execute(&self, command: Command) -> Outcome<CommandOutcome> {
        let command: core::Command = to_core("command", &command)?;
        known(&self.embed.execute(command).map_err(fail)?)
    }

    /// The game's token, valid until `expires_at` (RFC 3339).
    pub fn provide_token(&self, token: SecretString, expires_at: String) -> Outcome<()> {
        self.embed.provide_token(String::from(token), expires_at).map_err(fail)
    }

    /// The game's token, valid for `expires_in_secs` from now.
    pub fn provide_token_for(&self, token: SecretString, expires_in_secs: i64) -> Outcome<()> {
        let secs = narrow("expires_in_secs", expires_in_secs)?;
        self.embed.provide_token_for(String::from(token), secs).map_err(fail)
    }

    /// The game cannot get a token: every waiting call fails.
    pub fn token_unavailable(&self, reason: String) -> Outcome<()> {
        self.embed.token_unavailable(&reason).map_err(fail)
    }

    /// Switches to, or re-points, the Http token source.
    pub fn set_token_endpoint(&self, url: String, headers: Vec<Header>) -> Outcome<()> {
        let headers: Vec<core::Header> = to_core("headers", &headers)?;
        self.embed.set_token_endpoint(&url, headers).map_err(fail)
    }

    /// Drops the stream's connection, keeping its cursor.
    pub fn stream_pause(&self) -> Outcome<()> {
        self.embed.stream_pause().map_err(fail)
    }

    /// Reopens the stream from the kept cursor.
    pub fn stream_resume(&self) -> Outcome<()> {
        self.embed.stream_resume().map_err(fail)
    }
}
