//! The handoff body (ADR 1.10.26af D6, ES6): `{"token":"lgr_et_…","expires_at":"…"}`,
//! the shape every player kit and the widget read, and since ADR 31.10.26f a
//! third key, `update_available`, when the mint answered one. This is the one
//! place a kit exposes the token on purpose.

use lingara::embed::MintedToken;

use crate::types::UpdateAvailable;

/// The JSON your token route returns to the player. `expires_at` is the
/// server's string, copied byte for byte; `expires_in` and `subject` are
/// not forwarded. The mint's `update_available` is forwarded when present.
pub fn token_handoff(minted: &MintedToken) -> String {
    let notice = minted.update_available.as_ref().map(|n| UpdateAvailable {
        latest: n.latest.clone(),
        deprecated: n.deprecated,
        message: n.message.clone().map(String::from),
    });
    handoff_body(minted.token.expose_secret(), &minted.expires_at, notice.as_ref())
}

/// The pure body, held to `token-handoff.json`: compact, `token` then
/// `expires_at`, then `update_available` only when given (its keys `latest`,
/// `deprecated`, `message`, nulls kept); `/` and non-ASCII unescaped.
pub(crate) fn handoff_body(token: &str, expires_at: &str, update_available: Option<&UpdateAvailable>) -> String {
    let notice = match update_available {
        // The field order of `UpdateAvailable` is the wire order (V1 §6).
        Some(n) => format!(",\"update_available\":{}", serde_json::to_string(n).unwrap_or_default()),
        None => String::new(),
    };
    format!("{{\"token\":{},\"expires_at\":{}{notice}}}", json_string(token), json_string(expires_at))
}

fn json_string(value: &str) -> String {
    serde_json::Value::from(value).to_string()
}
