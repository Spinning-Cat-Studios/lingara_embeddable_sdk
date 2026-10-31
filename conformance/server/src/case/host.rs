//! An EK case's `host:` script (K1 D12): what the harness does through its
//! binding's surface, in K1's JSON vocabulary, which every binding already
//! maps. Every step and every field is closed. The server never runs these
//! steps; it serves them to the harness and judges what the kit then sends.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One step. Each is a one-key map in the YAML (`- start: {…}`).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum HostStep {
    /// `Embed::start` with `config`, then `provide_token` if a token is given.
    Start(Start),
    /// `ProvideToken` (`expires_at`) or `ProvideTokenFor` (`expires_in_secs`).
    ProvideToken(ProvideToken),
    /// A `Command` in its JSON shape, through `execute` or the flat method.
    Command(Value),
    /// Drain until an event of type `event` containing `match` arrives,
    /// failing at `within_ms`.
    PollUntil(PollUntil),
    /// Drain for `for_ms`, failing on a match.
    ExpectNoEvent(ExpectNoEvent),
    /// Do not drain for `for_ms` (EK9's backpressure case).
    StopPolling(StopPolling),
    /// `close`, then drain to `Closed`; an ABI harness frees instead.
    Close(super::Empty),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Start {
    /// `none`, or the token the harness provides at once.
    pub initial_token: String,
    #[serde(default)]
    pub config: StartConfig,
}

/// The `EmbedConfig` fields a case may set. `base_url` is always the
/// server's, and the token source is always the poll source.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StartConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queue_capacity: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_agent_suffix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_types: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resume_cursor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream_on_start: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream_idle_timeout_ms: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log_level: Option<String>,
}

/// Exactly one of the two lifetimes (`shape_problems` checks it).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProvideToken {
    pub token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_in_secs: Option<u32>,
    /// The command's `update_available` (ADR 30.10.26p D5), passed through
    /// as written: EK13's malformed case relies on it never being checked here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update_available: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PollUntil {
    /// An `EmbedEvent` `type` tag: `stream`, `npc_turn`, …
    pub event: String,
    /// A JSON object the event must contain, compared after dropping nulls.
    #[serde(default, rename = "match", skip_serializing_if = "Option::is_none")]
    pub matches: Option<Value>,
    pub within_ms: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectNoEvent {
    pub event: String,
    #[serde(default, rename = "match", skip_serializing_if = "Option::is_none")]
    pub matches: Option<Value>,
    pub for_ms: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StopPolling {
    pub for_ms: u64,
}

/// The rules serde cannot state about one step.
pub fn step_problems(id: &str, n: usize, step: &HostStep) -> Option<String> {
    match step {
        HostStep::ProvideToken(p) if p.expires_at.is_some() == p.expires_in_secs.is_some() => {
            Some(format!("{id}: host[{n}].provide_token names exactly one of expires_at and expires_in_secs"))
        }
        HostStep::Command(c) if !c.get("type").is_some_and(Value::is_string) => Some(format!("{id}: host[{n}].command is a Command with a `type`")),
        _ => None,
    }
}
