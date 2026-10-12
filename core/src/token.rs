//! `HostTokenSource` (D4): the library's `TokenSource`, filled by the host
//! rather than an OAuth endpoint. It caches one token and its stale point,
//! raises one `TokenNeeded` per outstanding need (poll source) or fetches
//! (Http and trait sources), asks again proactively at the stale point, and
//! fails every waiter at once on a timeout or a refusal. `token_bridge.rs`
//! hands it to the library.

pub(crate) mod bridge;
pub(crate) mod fetch;
pub(crate) mod handoff;
pub(crate) mod rfc3339;

use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use lingara::AccessToken;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio::time::Instant;

use crate::error::{Diagnostics, EmbedError, ErrorKind};
use crate::handoff::IssuedToken;
use crate::ids::Ids;
use crate::queue::Outbox;
use crate::sync::lock;
use crate::token_fetch::Mode;
use crate::types::{EmbedEvent, TokenReason};

struct Cached {
    token: AccessToken,
    stale_at: Instant,
}

impl Cached {
    fn fresh(&self) -> bool {
        Instant::now() < self.stale_at
    }
}

struct State {
    cached: Option<Cached>,
    /// The open need's request id, under the poll source.
    need: Option<u64>,
    reason: TokenReason,
    mode: Mode,
    timer: Option<JoinHandle<()>>,
}

/// One generation per fill or failure. Waiters wait for the next one.
#[derive(Clone, Default)]
pub(crate) struct Generation {
    pub n: u64,
    pub failure: Option<EmbedError>,
}

pub(crate) struct HostTokenSource {
    state: Mutex<State>,
    changed: watch::Sender<Generation>,
    fetching: tokio::sync::Mutex<()>,
    outbox: Outbox,
    diag: Diagnostics,
    ids: Arc<Ids>,
    wait: Duration,
}

impl HostTokenSource {
    pub fn new(mode: Mode, outbox: Outbox, diag: Diagnostics, ids: Arc<Ids>, wait: Duration) -> Arc<Self> {
        let state = State { cached: None, need: None, reason: TokenReason::Initial, mode, timer: None };
        let (changed, _) = watch::channel(Generation::default());
        Arc::new(Self { state: Mutex::new(state), changed, fetching: tokio::sync::Mutex::new(()), outbox, diag, ids, wait })
    }

    /// A fresh token: the cached one, or one waited or fetched for.
    pub(crate) async fn ready(self: &Arc<Self>) -> Result<AccessToken, EmbedError> {
        loop {
            let mut changes = self.changed.subscribe();
            let mode = {
                let mut state = lock(&self.state);
                if let Some(cached) = state.cached.as_ref().filter(|c| c.fresh()) {
                    return Ok(cached.token.clone());
                }
                if matches!(state.mode, Mode::Poll) {
                    self.open_need(&mut state);
                }
                state.mode.clone()
            };
            match mode {
                Mode::Poll => self.wait_for_change(&mut changes).await?,
                source => self.fetch(source).await?,
            }
        }
    }

    /// Raises `TokenNeeded` unless a need is already open.
    fn open_need(&self, state: &mut State) {
        if state.need.is_none() {
            let request = self.ids.next();
            state.need = Some(request);
            self.outbox.priority(EmbedEvent::TokenNeeded { request, reason: state.reason });
        }
    }

    async fn wait_for_change(&self, changes: &mut watch::Receiver<Generation>) -> Result<(), EmbedError> {
        match tokio::time::timeout(self.wait, changes.changed()).await {
            Ok(Ok(())) => changes.borrow().failure.clone().map_or(Ok(()), Err),
            Ok(Err(_)) => Err(EmbedError::closed()),
            Err(_) => {
                let err = EmbedError::new(ErrorKind::TokenUnavailable, format!("no token within {} ms", self.wait.as_millis()));
                self.fail_all(err.clone());
                Err(err)
            }
        }
    }

    /// One fetch at a time from the Http or trait source; a failure, or a
    /// token that is not an embed token, fails every waiter.
    async fn fetch(self: &Arc<Self>, mode: Mode) -> Result<(), EmbedError> {
        let _one = self.fetching.lock().await;
        let reason = {
            let state = lock(&self.state);
            if state.cached.as_ref().is_some_and(Cached::fresh) {
                return Ok(());
            }
            state.reason
        };
        let issued = match tokio::time::timeout(self.wait, fetch::fetch(&mode, reason)).await {
            Ok(result) => result,
            Err(_) => Err(EmbedError::new(ErrorKind::TokenUnavailable, format!("the token source gave nothing within {} ms", self.wait.as_millis()))),
        };
        let stored = issued.and_then(|issued| self.store(&issued));
        if let Err(err) = &stored {
            self.fail_all(err.clone());
        }
        stored
    }

    /// A token from the host (`provide_token`) or a source. A refusal leaves
    /// an open need open.
    pub fn store(self: &Arc<Self>, issued: &IssuedToken) -> Result<(), EmbedError> {
        handoff::check_embed_token(&issued.token)?;
        let lifetime = handoff::lifetime(issued, SystemTime::now())?;
        if let Some(message) = lifetime.clamped {
            self.diag.warn("lingara_embed::token", message);
        }
        let stale_at = Instant::now() + handoff::stale_after(lifetime.duration);
        let me = Arc::clone(self);
        let timer = self.outbox.handle().spawn(async move {
            tokio::time::sleep_until(stale_at).await;
            me.on_stale();
        });
        let mut state = lock(&self.state);
        state.cached = Some(Cached { token: AccessToken::new(issued.token.expose_secret()), stale_at });
        state.need = None;
        state.reason = TokenReason::Expiring;
        if let Some(old) = state.timer.replace(timer) {
            old.abort();
        }
        drop(state);
        self.bump(None);
        Ok(())
    }

    /// The stale point: ask again before any call meets an expired token.
    fn on_stale(self: &Arc<Self>) {
        let mut state = lock(&self.state);
        match state.mode.clone() {
            Mode::Poll => self.open_need(&mut state),
            source => {
                drop(state);
                let me = Arc::clone(self);
                self.outbox.handle().spawn(async move {
                    let _ = me.fetch(source).await;
                });
            }
        }
    }

    /// Fails every waiter and caches nothing.
    pub fn fail_all(&self, err: EmbedError) {
        let mut state = lock(&self.state);
        state.need = None;
        state.cached = None;
        drop(state);
        self.bump(Some(err));
    }

    fn bump(&self, failure: Option<EmbedError>) {
        self.changed.send_modify(|g| {
            g.n += 1;
            g.failure = failure;
        });
    }

    /// Under the poll source with no fresh token, raise the need now rather
    /// than at the first call, so the host's first poll already sees it.
    pub fn ask(&self) {
        let mut state = lock(&self.state);
        if matches!(state.mode, Mode::Poll) && !state.cached.as_ref().is_some_and(Cached::fresh) {
            self.open_need(&mut state);
        }
    }

    pub fn set_mode(&self, mode: Mode) {
        lock(&self.state).mode = mode;
    }

    pub fn needs_token(&self) -> bool {
        lock(&self.state).need.is_some()
    }

    /// Fills and failures, for a task that waits for the next token.
    pub fn subscribe(&self) -> watch::Receiver<Generation> {
        self.changed.subscribe()
    }

    pub fn shutdown(&self) {
        if let Some(timer) = lock(&self.state).timer.take() {
            timer.abort();
        }
    }
}
