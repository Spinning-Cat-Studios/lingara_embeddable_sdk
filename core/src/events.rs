//! The stream (D5): the library's CONTRACT K5a tail, used unchanged, kept
//! alive. When the tail raises, the stream goes `Down` and a new tail opens
//! from the cursor after 60 s, or, after a `403` or a rejected token, once
//! the next token arrives. A resumed tail's repeats are dropped by id.

use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::task::AbortHandle;

use crate::error::{EmbedError, ErrorKind};
use crate::limits;
use crate::runtime;
use crate::shared::Shared;
use crate::sync::lock;
use crate::types::{EmbedEvent, StreamState};

struct Inner {
    task: Option<AbortHandle>,
    state: StreamState,
    cursor: Option<String>,
}

pub(crate) struct StreamControl {
    inner: Mutex<Inner>,
}

impl StreamControl {
    pub fn new(cursor: Option<String>) -> Self {
        Self { inner: Mutex::new(Inner { task: None, state: StreamState::Paused, cursor }) }
    }

    pub fn state(&self) -> (StreamState, Option<String>) {
        let inner = lock(&self.inner);
        (inner.state.clone(), inner.cursor.clone())
    }

    /// Starts a tail from the kept cursor, unless one is running.
    pub fn resume(&self, shared: &Arc<Shared>) {
        let mut inner = lock(&self.inner);
        if inner.task.is_none() {
            inner.task = Some(runtime::spawn(shared, run(Arc::clone(shared))));
        }
    }

    /// Drops the tail and its connection; the cursor stays.
    pub fn pause(&self, shared: &Arc<Shared>) {
        let mut inner = lock(&self.inner);
        let Some(task) = inner.task.take() else { return };
        task.abort();
        inner.state = StreamState::Paused;
        let cursor = inner.cursor.clone();
        drop(inner);
        shared.outbox.push_later(EmbedEvent::Stream { state: StreamState::Paused, cursor });
    }

    fn set_cursor(&self, cursor: Option<String>) {
        if cursor.is_some() {
            lock(&self.inner).cursor = cursor;
        }
    }

    fn cursor(&self) -> Option<String> {
        lock(&self.inner).cursor.clone()
    }

    /// Records a state and returns the event that reports it.
    fn enter(&self, state: StreamState) -> EmbedEvent {
        let mut inner = lock(&self.inner);
        inner.state = state.clone();
        EmbedEvent::Stream { state, cursor: inner.cursor.clone() }
    }
}

/// The last `DEDUP_WINDOW` event ids.
#[derive(Default)]
pub(crate) struct Dedup {
    order: VecDeque<String>,
    seen: HashSet<String>,
}

impl Dedup {
    /// True the first time `id` is seen within the window.
    pub fn first(&mut self, id: &str) -> bool {
        if self.seen.contains(id) {
            return false;
        }
        if self.order.len() == limits::DEDUP_WINDOW {
            if let Some(old) = self.order.pop_front() {
                self.seen.remove(&old);
            }
        }
        self.order.push_back(id.to_owned());
        self.seen.insert(id.to_owned());
        true
    }
}

/// The stream task: a tail, then `Down`, then a new tail, until aborted.
async fn run(shared: Arc<Shared>) {
    let mut dedup = Dedup::default();
    loop {
        shared.outbox.send(shared.stream.enter(StreamState::Connecting)).await;
        let error = read_tail(&shared, &mut dedup).await;
        let waits_for_token = matches!(error.kind, ErrorKind::Forbidden | ErrorKind::TokenRejected);
        let retry_in_ms = if waits_for_token { 0 } else { limits::STREAM_RESTART_MS };
        shared.outbox.send(shared.stream.enter(StreamState::Down { error, retry_in_ms })).await;
        if waits_for_token {
            next_token(&shared).await;
        } else {
            tokio::time::sleep(Duration::from_millis(retry_in_ms)).await;
        }
    }
}

/// One tail, read until it raises. Each new event reaches the host once, and
/// the plan events reach any practice session waiting on them.
async fn read_tail(shared: &Arc<Shared>, dedup: &mut Dedup) -> EmbedError {
    let mut tail = shared.api.tail(shared.stream.cursor(), shared.config.event_types.clone());
    let mut live = false;
    loop {
        let item = tail.next().await;
        shared.stream.set_cursor(tail.cursor());
        let event = match item {
            Some(Ok(event)) => event,
            Some(Err(error)) => return error,
            None => return EmbedError::new(ErrorKind::Network, "the tail ended"),
        };
        if !live {
            live = true;
            shared.outbox.send(shared.stream.enter(StreamState::Live)).await;
        }
        if dedup.first(&event.id) {
            shared.practice.on_event(&event);
            shared.outbox.send(EmbedEvent::Lingara { event }).await;
        }
    }
}

/// Waits for the next successful token: a scope or origin refusal will not
/// change until the integrator mints differently.
async fn next_token(shared: &Shared) {
    let mut changes = shared.tokens.subscribe();
    while changes.changed().await.is_ok() {
        if changes.borrow().failure.is_none() {
            return;
        }
    }
}
