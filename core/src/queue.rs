//! The output queue (D2): bounded, so a task that finds it full waits and
//! the stream stops reading, with the server holding the backlog. Nothing is
//! ever dropped while the handle is open. `TokenNeeded` goes to a priority
//! slot ahead of it, so a full queue never hides the one event the core
//! cannot proceed without.
//!
//! After `close`, the queue yields one `Closed` and then nothing (a closing
//! `Log`, when the close was a panic, comes just before it).

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use tokio::runtime::Handle;
use tokio::sync::mpsc;

use crate::sync::lock;
use crate::types::{CloseReason, EmbedEvent};

enum CloseState {
    Open,
    Pending { reason: CloseReason, preface: Option<EmbedEvent> },
    Done,
}

/// The engine's end: polled without blocking, from any thread.
pub(crate) struct Queue {
    rx: Mutex<mpsc::Receiver<EmbedEvent>>,
    priority: Arc<Mutex<VecDeque<EmbedEvent>>>,
    state: Mutex<CloseState>,
}

/// The tasks' end. Cheap to clone.
#[derive(Clone)]
pub(crate) struct Outbox {
    tx: mpsc::Sender<EmbedEvent>,
    priority: Arc<Mutex<VecDeque<EmbedEvent>>>,
    handle: Handle,
}

/// A queue of `capacity` events, whose deferred sends run on `handle`.
pub(crate) fn channel(capacity: u32, handle: Handle) -> (Queue, Outbox) {
    let (tx, rx) = mpsc::channel(usize::try_from(capacity).unwrap_or(usize::MAX).max(1));
    let priority = Arc::new(Mutex::new(VecDeque::new()));
    let queue = Queue { rx: Mutex::new(rx), priority: Arc::clone(&priority), state: Mutex::new(CloseState::Open) };
    (queue, Outbox { tx, priority, handle })
}

impl Queue {
    /// The next event, never blocking: the priority slot first.
    pub fn poll(&self) -> Option<EmbedEvent> {
        {
            let mut state = lock(&self.state);
            match &mut *state {
                CloseState::Open => {}
                CloseState::Pending { preface, .. } if preface.is_some() => return preface.take(),
                CloseState::Pending { reason, .. } => {
                    let reason = *reason;
                    *state = CloseState::Done;
                    return Some(EmbedEvent::Closed { reason });
                }
                CloseState::Done => return None,
            }
        }
        if let Some(event) = lock(&self.priority).pop_front() {
            return Some(event);
        }
        lock(&self.rx).try_recv().ok()
    }

    pub fn drain(&self, max: u32) -> Vec<EmbedEvent> {
        let max = usize::try_from(max).unwrap_or(usize::MAX);
        std::iter::from_fn(|| self.poll()).take(max).collect()
    }

    /// Marks the queue closed; true the first time. Whatever was queued is
    /// discarded, and every waiting sender is released.
    pub fn close(&self, reason: CloseReason, preface: Option<EmbedEvent>) -> bool {
        let mut state = lock(&self.state);
        if !matches!(*state, CloseState::Open) {
            return false;
        }
        *state = CloseState::Pending { reason, preface };
        let mut rx = lock(&self.rx);
        rx.close();
        while rx.try_recv().is_ok() {}
        lock(&self.priority).clear();
        true
    }

    pub fn is_closed(&self) -> bool {
        !matches!(*lock(&self.state), CloseState::Open)
    }
}

impl Outbox {
    /// Queues `event`, waiting while the queue is full. A closed queue
    /// takes nothing.
    pub async fn send(&self, event: EmbedEvent) {
        let _ = self.tx.send(event).await;
    }

    /// `send` from a context that cannot wait (an engine-thread method):
    /// spawned onto the I/O thread, in order with other deferred sends.
    pub fn push_later(&self, event: EmbedEvent) {
        let tx = self.tx.clone();
        self.handle.spawn(async move {
            let _ = tx.send(event).await;
        });
    }

    /// Ahead of the queue, never waiting: `TokenNeeded` only.
    pub fn priority(&self, event: EmbedEvent) {
        if !self.tx.is_closed() {
            lock(&self.priority).push_back(event);
        }
    }

    pub fn handle(&self) -> &Handle {
        &self.handle
    }
}
