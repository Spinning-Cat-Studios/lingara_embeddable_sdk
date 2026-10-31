//! `EmbedConfig` and its validation (D3). It holds no token, derives
//! `Default`, and deserialises with every field defaulted, so a binding's
//! JSON config names only what it changes.

use serde::{Deserialize, Serialize};

use crate::error::{EmbedError, ErrorKind};
use crate::limits;
use crate::secret::Secret;
use crate::types::LogLevel;

/// One header the Http token source sends. The value is a secret: a game
/// session is a credential too.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Header {
    pub name: String,
    pub value: Secret,
}

/// Where tokens come from (D4). `Poll` raises `TokenNeeded` and waits for
/// `provide_token`; `Http` POSTs `{}` to the integrator's URL.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum TokenSourceConfig {
    #[default]
    Poll,
    Http {
        url: String,
        #[serde(default)]
        headers: Vec<Header>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct EmbedConfig {
    pub base_url: String,
    pub token_source: TokenSourceConfig,
    /// The `Lingara-Version` pin.
    pub api_version: Option<String>,
    /// The tail's `types`; empty is every type.
    pub event_types: Vec<String>,
    /// A cursor saved from `status()` or a `Stream` event.
    pub resume_cursor: Option<String>,
    /// `false` starts paused: a game that never listens pays nothing.
    pub stream_on_start: bool,
    pub queue_capacity: u32,
    pub token_wait_timeout_ms: u32,
    /// A heartbeat gap this long reconnects the stream (at least 1 000).
    pub stream_idle_timeout_ms: u32,
    pub log_level: LogLevel,
    /// Appended after `lingara-embed/<version>`, e.g. `my-game/1.4`.
    pub user_agent_suffix: Option<String>,
    /// The platform embed version this build was registered against
    /// (`MAJOR.MINOR.PATCH`), sent to the token endpoint as
    /// `Lingara-Embed-Version` (ADR 30.10.26p D1). Never sent to Lingara.
    pub embed_version: Option<String>,
}

impl Default for EmbedConfig {
    fn default() -> Self {
        Self {
            base_url: lingara::DEFAULT_BASE_URL.to_owned(),
            token_source: TokenSourceConfig::Poll,
            api_version: None,
            event_types: Vec::new(),
            resume_cursor: None,
            stream_on_start: true,
            queue_capacity: limits::DEFAULT_QUEUE_CAPACITY,
            token_wait_timeout_ms: limits::DEFAULT_TOKEN_WAIT_TIMEOUT_MS,
            stream_idle_timeout_ms: limits::DEFAULT_STREAM_IDLE_TIMEOUT_MS,
            log_level: LogLevel::Warn,
            user_agent_suffix: None,
            embed_version: None,
        }
    }
}

fn config_error(message: impl Into<String>) -> EmbedError {
    EmbedError::new(ErrorKind::Config, message)
}

/// An absolute `http` or `https` URL.
pub(crate) fn check_url(option: &str, url: &str) -> Result<(), EmbedError> {
    match reqwest::Url::parse(url) {
        Ok(u) if matches!(u.scheme(), "http" | "https") => Ok(()),
        Ok(_) => Err(config_error(format!("{option} must be an http or https URL"))),
        Err(e) => Err(config_error(format!("{option} is not a valid URL: {e}"))),
    }
}

/// V1's grammar for a declared version (Backend 30.10.26o §6): three
/// dot-separated numbers of at most six digits, no leading zero, no `v`,
/// at most 20 characters. So `v1.0`, `1.0` and `01.0.0` are refused.
pub(crate) fn check_embed_version(version: &str) -> Result<(), EmbedError> {
    let number = |part: &str| (1..=6).contains(&part.len()) && part.bytes().all(|b| b.is_ascii_digit()) && (part == "0" || !part.starts_with('0'));
    let parts: Vec<&str> = version.split('.').collect();
    if version.len() <= 20 && parts.len() == 3 && parts.iter().all(|p| number(p)) {
        return Ok(());
    }
    Err(config_error("embed_version must be MAJOR.MINOR.PATCH (no leading zeros, no `v`, at most 20 characters)"))
}

impl EmbedConfig {
    /// D9's `Config` cases, checked before `start` spawns anything.
    pub fn validate(&self) -> Result<(), EmbedError> {
        check_url("base_url", &self.base_url)?;
        if let TokenSourceConfig::Http { url, .. } = &self.token_source {
            check_url("token_source.url", url)?;
        }
        if self.api_version.as_deref() == Some("") {
            return Err(config_error("api_version must not be empty"));
        }
        if let Some(version) = &self.embed_version {
            check_embed_version(version)?;
        }
        if self.queue_capacity == 0 {
            return Err(config_error("queue_capacity must be at least 1"));
        }
        if self.stream_idle_timeout_ms < limits::MIN_STREAM_IDLE_TIMEOUT_MS {
            return Err(config_error(format!("stream_idle_timeout_ms must be at least {}", limits::MIN_STREAM_IDLE_TIMEOUT_MS)));
        }
        Ok(())
    }
}
