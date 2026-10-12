//! Errors and diagnostics (D9): the one closed `ErrorKind` every binding
//! surfaces, the one mapping from the library's errors to it, and the bridge
//! that queues the core's own log lines as `Log` events.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::queue::Outbox;
use crate::types::{EmbedEvent, LogLevel, RequestId};

/// The closed code K2–K7 surface. `retryable` is a function of the kind
/// alone, so every binding agrees on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ErrorKind {
    Config,
    InvalidInput,
    TokenUnavailable,
    TokenRejected,
    Forbidden,
    Billing,
    NotFound,
    RateLimited,
    Unavailable,
    Network,
    Busy,
    InvalidState,
    UnknownId,
    Closed,
    Internal,
}

impl ErrorKind {
    /// Whether the same call may succeed later. It never makes the core
    /// retry on its own.
    pub fn retryable(self) -> bool {
        matches!(self, ErrorKind::TokenUnavailable | ErrorKind::RateLimited | ErrorKind::Unavailable | ErrorKind::Network)
    }
}

/// Every failure the core reports. `message` is English, for logs, never
/// shown to a learner, and never holds a token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[error("{kind:?}: {message}")]
pub struct EmbedError {
    pub kind: ErrorKind,
    pub message: String,
    #[serde(default)]
    pub status: Option<u16>,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub retry_after_ms: Option<u64>,
    pub retryable: bool,
    /// The command this failure answers, when it answers one.
    #[serde(default)]
    pub request: Option<RequestId>,
}

impl EmbedError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self { kind, message: message.into(), status: None, code: None, retry_after_ms: None, retryable: kind.retryable(), request: None }
    }

    pub(crate) fn invalid(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::InvalidInput, message)
    }

    pub(crate) fn closed() -> Self {
        Self::new(ErrorKind::Closed, "the embed handle is closed")
    }

    pub(crate) fn with_request(mut self, request: RequestId) -> Self {
        self.request = Some(request);
        self
    }
}

/// The kind a refused `/v1` call maps to, by status (D9's table).
fn kind_for_status(status: u16) -> ErrorKind {
    match status {
        200 | 500..=599 => ErrorKind::Unavailable,
        400 | 422 => ErrorKind::InvalidInput,
        401 => ErrorKind::TokenRejected,
        402 => ErrorKind::Billing,
        403 => ErrorKind::Forbidden,
        404 => ErrorKind::NotFound,
        410 => ErrorKind::Config,
        429 => ErrorKind::RateLimited,
        _ => ErrorKind::Internal,
    }
}

fn millis(d: Option<std::time::Duration>) -> Option<u64> {
    d.map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// The one mapping from the library's errors (D9). A status-200 `ApiError`
/// is an in-stream `error` frame; an `OAuthError` cannot occur, because the
/// core never calls a token endpoint, so it is `Internal`.
pub(crate) fn from_library(err: &lingara::Error) -> EmbedError {
    match err {
        lingara::Error::Api(e) => {
            let mut out = EmbedError::new(kind_for_status(e.status), e.message.clone());
            out.status = Some(e.status);
            out.code = Some(e.code.clone());
            out.retry_after_ms = millis(e.retry_after);
            out
        }
        lingara::Error::Maintenance(e) => {
            let mut out = EmbedError::new(ErrorKind::Unavailable, "the Lingara API is under maintenance");
            out.status = Some(503);
            out.retry_after_ms = millis(e.retry_after);
            out
        }
        lingara::Error::Transport(e) => EmbedError::new(ErrorKind::Network, format!("transport failure: {}", e.kind)),
        other => EmbedError::new(ErrorKind::Internal, other.to_string()),
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
        })
    }
}

impl From<LogLevel> for log::Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Error => log::Level::Error,
            LogLevel::Warn => log::Level::Warn,
            LogLevel::Info => log::Level::Info,
            LogLevel::Debug => log::Level::Debug,
            LogLevel::Trace => log::Level::Trace,
        }
    }
}

/// The core's diagnostics: through the `log` facade (no logger installed),
/// and, at or above `log_level`, as `Log` events for an engine with no Rust
/// logger. Never holds a token: callers pass only their own words.
#[derive(Clone)]
pub(crate) struct Diagnostics {
    pub level: LogLevel,
    pub outbox: Outbox,
}

impl Diagnostics {
    pub fn emit(&self, level: LogLevel, target: &'static str, message: String) {
        log::log!(target: target, level.into(), "{message}");
        if level <= self.level {
            self.outbox.push_later(EmbedEvent::Log { level, target: target.to_owned(), message });
        }
    }

    pub fn warn(&self, target: &'static str, message: impl Into<String>) {
        self.emit(LogLevel::Warn, target, message.into());
    }
}
