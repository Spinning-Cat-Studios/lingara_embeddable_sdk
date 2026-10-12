//! The handoff body (ADR 1.10.26af D6, ES6): exactly
//! `{"token":"lgr_et_…","expires_at":"…"}`, the shape every player kit and
//! the widget read. This is the one place a kit exposes the token on purpose.

use lingara::embed::MintedToken;

/// The JSON your token route returns to the player. `expires_at` is the
/// server's string, copied byte for byte; `expires_in` and `subject` are
/// not forwarded.
pub fn token_handoff(minted: &MintedToken) -> String {
    handoff_body(minted.token.expose_secret(), &minted.expires_at)
}

/// The pure body over two strings, held to `token-handoff.json`: compact,
/// these two keys in this order, `/` and non-ASCII unescaped.
pub(crate) fn handoff_body(token: &str, expires_at: &str) -> String {
    format!("{{\"token\":{},\"expires_at\":{}}}", json_string(token), json_string(expires_at))
}

fn json_string(value: &str) -> String {
    serde_json::Value::from(value).to_string()
}
