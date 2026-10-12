//! An ES case's own keys (ADR 1.10.26af D11): the `kit:` jobs a harness
//! performs through its server kit, the harness's player directory before
//! and after, and the per-delivery router and handler. Every key is closed.
//! The server serves these to the harness and judges only what it sees on
//! the wire; the harness judges the rest and reports a verdict.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{Empty, SecretName};

/// One job, in order. Each is a one-key map in the YAML (`- mint: {…}`).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum KitStep {
    /// `mintToken(player_ref, {scopes?, origin?})`.
    Mint(MintJob),
    /// `deletePlayer(player_ref)`.
    Delete(DeleteJob),
    /// `tokenHandoff` over the last minted token (ES6).
    Handoff(Empty),
    /// ES5's sweep: the renderings of the server, router, directory and last
    /// minted token, and the captured logs, once every delivery has arrived.
    Render(Empty),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MintJob {
    pub player_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scopes: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeleteJob {
    pub player_ref: String,
}

/// The harness's `MemoryPlayerDirectory` before the case: subject → player_ref.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Directory {
    #[serde(default)]
    pub seed: BTreeMap<String, String>,
}

/// What a delivery's `onEvent` does: return, or raise (D8's `500` row).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HandlerMode {
    #[default]
    Ok,
    Raise,
}

/// The secrets the harness builds this delivery's `PlayerRouter` from.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Router {
    pub secrets: Vec<SecretName>,
}

/// `expect.minted`: B1's six fields in wire form, the token read through the
/// library's exposing accessor.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Minted {
    pub token: String,
    pub expires_at: String,
    pub expires_in: u64,
    pub subject: String,
    pub scopes: Vec<String>,
    pub account_linked: bool,
}

/// `expect.error`: a raised library (or C#) error's status and code. A
/// maintenance error's code is `maintenance`; an OAuth error's is its `error`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RaisedError {
    pub status: u16,
    pub code: String,
}



/// `handler: not_called`, or `handler: { player_ref: <ref> | null,
/// event_id?, unknown? }` (widened by 1.10.26af D11 from R1's
/// `{ player_ref }`, which still parses).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Handler {
    Word(HandlerWord),
    Called(Called),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HandlerWord {
    NotCalled,
}

/// What `onEvent` saw. The server judges `player_ref` against the harness's
/// report; the harness judges the whole of it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Called {
    pub player_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unknown: Option<bool>,
}

#[cfg(test)]
impl Handler {
    pub const NOT_CALLED: Handler = Handler::Word(HandlerWord::NotCalled);

    /// `{ player_ref: <r> }`, R1's original form.
    pub fn player_ref(r: &str) -> Handler {
        Handler::Called(Called { player_ref: Some(r.to_string()), ..Called::default() })
    }
}
