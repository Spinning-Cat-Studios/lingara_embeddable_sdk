//! Routing a webhook to its player (ADR 1.10.26af D8): the library's
//! verifier on the raw body first, then the directory, so a forged event
//! never reaches the integrator's store. `handle` maps every outcome to the
//! status all eight kits answer Lingara with.

use std::fmt;
use std::future::Future;
use std::sync::Arc;

use lingara::events::{Event, VerifyError, Webhook, WebhookHeaders};

use super::directory::{DirectoryError, PlayerDirectory};

/// A verified event and the player it concerns, or `None` for a subject the
/// directory does not know (the integrator's own, a deleted player).
#[derive(Clone, Debug)]
pub struct RoutedEvent {
    pub event: Event,
    pub player_ref: Option<String>,
}

/// Why `route` refused. Neither arm holds the body, a signature or a secret.
#[derive(Debug, thiserror::Error)]
pub enum RouteError {
    #[error(transparent)]
    Verify(#[from] VerifyError),
    #[error("the player directory failed: {0}")]
    Directory(DirectoryError),
}

/// A failure `handle` answered with `500`, as `on_error` receives it.
#[derive(Debug)]
pub enum HandleFailure {
    Directory(DirectoryError),
    Handler(DirectoryError),
}

impl fmt::Display for HandleFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HandleFailure::Directory(e) => write!(f, "the player directory failed: {e}"),
            HandleFailure::Handler(e) => write!(f, "the event handler failed: {e}"),
        }
    }
}

type OnError = Arc<dyn Fn(&HandleFailure) + Send + Sync>;

/// Verifies Lingara's webhooks and resolves each to a player.
#[derive(Clone)]
pub struct PlayerRouter {
    webhook: Webhook,
    directory: Arc<dyn PlayerDirectory>,
    on_error: OnError,
}

impl PlayerRouter {
    /// From a built verifier (`Webhook::with_secrets` for a rotation, whose
    /// builder error is the misconfiguration answer at startup).
    pub fn new(webhook: Webhook, directory: Arc<dyn PlayerDirectory>) -> Self {
        let on_error: OnError = Arc::new(|failure| log::error!("lingara webhook: {failure}"));
        Self { webhook, directory, on_error }
    }

    /// Where a `500`'s cause goes; by default `log::error!`.
    #[must_use]
    pub fn with_on_error(mut self, on_error: impl Fn(&HandleFailure) + Send + Sync + 'static) -> Self {
        self.on_error = Arc::new(on_error);
        self
    }

    /// Verifies the **raw** body, then looks the subject up.
    pub async fn route(&self, body: &[u8], headers: &impl WebhookHeaders) -> Result<RoutedEvent, RouteError> {
        let event = self.webhook.verify(body, headers)?;
        let player_ref = self.directory.player_ref_for(event.subject()).await.map_err(RouteError::Directory)?;
        Ok(RoutedEvent { event, player_ref })
    }

    /// `route`, then `on_event`, answered as the status your route sends:
    /// `204` handled, `401` a bad signature, `400` a malformed payload,
    /// `500` the directory or the handler failed. The body is always empty.
    pub async fn handle<F, Fut>(&self, body: &[u8], headers: &impl WebhookHeaders, on_event: F) -> u16
    where
        F: FnOnce(RoutedEvent) -> Fut,
        Fut: Future<Output = Result<(), DirectoryError>>,
    {
        let routed = match self.route(body, headers).await {
            Ok(routed) => routed,
            Err(RouteError::Verify(VerifyError::MalformedPayload)) => return 400,
            Err(RouteError::Verify(_)) => return 401,
            Err(RouteError::Directory(e)) => return self.failed(HandleFailure::Directory(e)),
        };
        match on_event(routed).await {
            Ok(()) => 204,
            Err(e) => self.failed(HandleFailure::Handler(e)),
        }
    }

    fn failed(&self, failure: HandleFailure) -> u16 {
        (self.on_error)(&failure);
        500
    }
}

impl fmt::Debug for PlayerRouter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PlayerRouter").field("webhook", &self.webhook).finish_non_exhaustive()
    }
}
