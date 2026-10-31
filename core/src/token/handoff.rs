//! What a token source hands the core (D4): the integrator's handoff body,
//! the Rust trait a Rust game implements, and a token's lifetime on a
//! device whose clock may be wrong by hours.

use std::future::Future;
use std::pin::Pin;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Deserialize;

use crate::error::{EmbedError, ErrorKind};
use crate::limits;
use crate::rfc3339;
use crate::secret::Secret;
use crate::types::{TokenReason, UpdateAvailable};

/// A boxed, `Send` future, so an `EmbedTokenSource` needs no `futures` crate.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A token as a source issues it: `expires_at` (the handoff body's), or
/// `expires_in_secs` (from a host holding the mint's whole answer). One of
/// the two is needed. `update_available` is the mint's optional notice
/// (ADR 30.10.26p D3). Rust-only: no serde, not in the shape vector.
#[derive(Debug, Clone)]
pub struct IssuedToken {
    pub token: Secret,
    pub expires_at: Option<String>,
    pub expires_in_secs: Option<u32>,
    pub update_available: Option<UpdateAvailable>,
}

/// A Rust game's own token source, awaited on the core's I/O thread
/// (`Embed::start_with_source`). Never part of the FFI surface.
pub trait EmbedTokenSource: Send + Sync + 'static {
    fn fetch(&self, reason: TokenReason) -> BoxFuture<'_, Result<IssuedToken, EmbedError>>;
}

/// The handoff body V1's ES6 fixes: `{token, expires_at}`, and since ADR
/// 30.10.26p an optional `update_available`, read leniently (a malformed
/// one is dropped, never fatal). Any other field (B1's whole six-field
/// answer, forwarded) is ignored.
#[derive(Deserialize)]
struct Handoff {
    token: String,
    expires_at: String,
    #[serde(default, deserialize_with = "crate::types::update::lenient")]
    update_available: Option<UpdateAvailable>,
}

impl IssuedToken {
    /// The Http source's parser. The message never echoes the body.
    pub fn from_handoff_json(body: &[u8]) -> Result<Self, EmbedError> {
        let handoff: Handoff = serde_json::from_slice(body)
            .map_err(|_| EmbedError::new(ErrorKind::TokenUnavailable, "the token endpoint's answer is not {token, expires_at}"))?;
        Ok(Self { token: Secret::new(handoff.token), expires_at: Some(handoff.expires_at), expires_in_secs: None, update_available: handoff.update_available })
    }
}

/// Refuses anything but an embed token (EK1). Not retryable: an endpoint
/// that answers an `lgr_at_` is misconfigured and will not fix itself.
pub(crate) fn check_embed_token(token: &Secret) -> Result<(), EmbedError> {
    if token.expose_secret().starts_with(limits::EMBED_TOKEN_PREFIX) {
        return Ok(());
    }
    Err(EmbedError::invalid("only an embed token (lgr_et_…) may be used on a player's device; a client-credentials token belongs on the integrator's server"))
}

/// The library's refresh rule: stale at `lifetime − min(60 s, lifetime / 2)`.
pub fn stale_after(lifetime: Duration) -> Duration {
    lifetime - Duration::from_secs(limits::STALE_MARGIN_SECS).min(lifetime / 2)
}

/// A lifetime, and the warning to log when it was clamped.
pub(crate) struct Lifetime {
    pub duration: Duration,
    pub clamped: Option<String>,
}

/// A lifetime outside `(0 s, 1 h]` is taken as 300 s: a device clock can be
/// wrong by hours, and B1's tokens live 900 s.
fn clamp(secs: i64, said: &str) -> Lifetime {
    if secs <= 0 || secs > limits::MAX_TRUSTED_LIFETIME_SECS as i64 {
        let message = format!("a token lifetime of {said} is outside (0 s, 1 h] on this device; treating it as {} s", limits::CLAMPED_LIFETIME_SECS);
        return Lifetime { duration: Duration::from_secs(limits::CLAMPED_LIFETIME_SECS), clamped: Some(message) };
    }
    Lifetime { duration: Duration::from_secs(secs.unsigned_abs()), clamped: None }
}

/// The lifetime of `issued`, read against `now` on the device clock.
pub(crate) fn lifetime(issued: &IssuedToken, now: SystemTime) -> Result<Lifetime, EmbedError> {
    if let Some(secs) = issued.expires_in_secs {
        return Ok(clamp(i64::from(secs), &format!("{secs} s")));
    }
    let Some(at) = issued.expires_at.as_deref() else {
        return Err(EmbedError::invalid("a token needs expires_at or expires_in_secs"));
    };
    let Some(at_secs) = rfc3339::to_unix(at) else {
        return Err(EmbedError::invalid("expires_at is not an RFC 3339 timestamp"));
    };
    let now_secs = now.duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    Ok(clamp(at_secs - now_secs, &format!("expires_at {at} − now {}", rfc3339::from_unix(now_secs))))
}
