//! The config mirrors and the secret type (ADR 1.10.26ae D2, D7).
//!
//! `EmbedConfig`'s defaulted fields are `Option`s, skipped when `None`, so a
//! Kotlin `EmbedConfig()` names only what it changes and K1's `Default`
//! fills the rest: no second copy of a default exists.

use serde::{Deserialize, Serialize};

use crate::types::LogLevel;

/// A credential crossing into K1: a token or a header value. Kotlin lifts
/// it into `com.getlingara.embed.Secret`, whose `toString()` redacts.
///
/// Its `Serialize` writes the raw value, because `convert.rs` hands it to
/// K1's `Secret` through JSON. Nothing serialises a mirror outward except
/// `event_to_json`, and no event carries one.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SecretString(String);

impl std::fmt::Debug for SecretString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(lingara_embed::REDACTED)
    }
}

impl From<String> for SecretString {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<SecretString> for String {
    fn from(value: SecretString) -> Self {
        value.0
    }
}

uniffi::custom_type!(SecretString, String);

/// One header the Http token source sends. The value is a secret: a game
/// session is a credential too.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct Header {
    pub name: String,
    pub value: SecretString,
}

/// Where tokens come from. `Poll` raises `TokenNeeded`; `Http` POSTs `{}`
/// to the game's own URL from K1's thread.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Enum)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TokenSourceConfig {
    Poll,
    Http {
        url: String,
        #[serde(default)]
        headers: Vec<Header>,
    },
}

/// The handle's config. Every null field takes the core's default.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, uniffi::Record)]
pub struct EmbedConfig {
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// Null is `Poll`.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_source: Option<TokenSourceConfig>,
    /// The `Lingara-Version` pin.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,
    /// The tail's `types`; empty is every type.
    #[uniffi(default = [])]
    #[serde(default)]
    pub event_types: Vec<String>,
    /// A cursor saved from `status()` or a `Stream` event.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resume_cursor: Option<String>,
    /// `false` starts paused: a game that never listens pays nothing.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_on_start: Option<bool>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queue_capacity: Option<i64>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_wait_timeout_ms: Option<i64>,
    /// A heartbeat gap this long reconnects the stream (at least 1 000).
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_idle_timeout_ms: Option<i64>,
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_level: Option<LogLevel>,
    /// Appended after the kit's own User-Agent token, e.g. `my-game/1.4`.
    #[uniffi(default = None)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent_suffix: Option<String>,
}
