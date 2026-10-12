//! The host token source as the library sees it (D4).
//!
//! A failure the library cannot carry (it has no variant for "the host gave
//! no token") travels beside the call in a task-local slot: the library's
//! `token()` is awaited inside the caller's own future, so `tracked` can
//! read the slot back and report `TokenUnavailable` rather than the
//! transport error the library raised in its place.

use std::sync::{Arc, Mutex};

use lingara::{AccessToken, TokenSource, TransportKind};

use crate::error::{EmbedError, from_library};
use crate::sync::lock;
use crate::token::HostTokenSource;
use crate::types::TokenReason;

tokio::task_local! {
    static FAILURE: Slot;
}

/// The last token outcome inside one tracked call: set on a failure,
/// cleared on a success, so it describes only the latest ask.
#[derive(Clone, Default)]
struct Slot(Arc<Mutex<Option<EmbedError>>>);

/// Runs one library call, mapping its error, a token failure first.
pub(crate) async fn tracked<T>(call: impl Future<Output = Result<T, lingara::Error>>) -> Result<T, EmbedError> {
    let slot = Slot::default();
    let result = FAILURE.scope(slot.clone(), call).await;
    result.map_err(|e| lock(&slot.0).take().unwrap_or_else(|| from_library(&e)))
}

pub(crate) struct LibrarySource(pub Arc<HostTokenSource>);

impl TokenSource for LibrarySource {
    fn token(&self) -> lingara::BoxFuture<'_, Result<AccessToken, lingara::Error>> {
        Box::pin(async move {
            let result = self.0.ready().await;
            let _ = FAILURE.try_with(|slot| *lock(&slot.0) = result.as_ref().err().cloned());
            result.map_err(|_| TransportKind::Connect.into())
        })
    }

    /// Compare-and-clear, as the library's own source does; the next ask
    /// says `Rejected`.
    fn invalidate(&self, token: &AccessToken) {
        let mut state = lock(&self.0.state);
        if state.cached.as_ref().is_some_and(|c| c.token.expose_secret() == token.expose_secret()) {
            state.cached = None;
            state.reason = TokenReason::Rejected;
        }
    }
}
