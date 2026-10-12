//! Where tokens come from (ADR 1.10.26ab D4): `token_url` selects K1's
//! Http source; empty, K1's Poll source and the `token_needed` round trip.
//! `set_token_endpoint` before `start()` is kept here, in memory only, and
//! becomes the Http source at start; after it, it is K1's own command.

use lingara_embed::{Command, EmbedError};
use serde_json::{Value, json};

use crate::commands;

/// An endpoint set before `start()`. Its header values are run-time only:
/// never a property, so never saved in a scene or an export.
#[derive(Default)]
pub struct Tokens {
    pending: Option<Value>,
}

/// What `set_token_endpoint` does now.
pub enum Endpoint {
    /// Kept for `start()`.
    Kept,
    /// Sent to the live handle.
    Send(Command),
}

impl Tokens {
    /// Before start the endpoint is kept; after, it is K1's command.
    pub fn set_endpoint(&mut self, running: bool, url: &str, headers: Value) -> Result<Endpoint, EmbedError> {
        let headers = commands::headers(headers)?;
        if running {
            return commands::build("set_token_endpoint", vec![("url", url.into()), ("headers", headers)]).map(Endpoint::Send);
        }
        self.pending = Some(json!({ "type": "http", "url": url, "headers": headers }));
        Ok(Endpoint::Kept)
    }

    /// K1's `token_source` in its JSON shape: a kept endpoint, else
    /// `token_url`, else Poll.
    pub fn source(&self, token_url: &str) -> Value {
        match (&self.pending, token_url.trim()) {
            (Some(kept), _) => kept.clone(),
            (None, "") => json!({ "type": "poll" }),
            (None, url) => json!({ "type": "http", "url": url, "headers": [] }),
        }
    }
}
