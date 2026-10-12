//! The per-frame drain and the handle's life (ADR 1.10.26ab D2), over the
//! `Core` seam in `port.rs`.
//!
//! Stopping never waits on the network; unloading does, briefly. `stop()`
//! calls K1's `close()`, which does not join K1's thread, and the handle
//! moves to a library-level list rather than being dropped. Godot's
//! deinitialisation joins every listed handle with `close_and_wait(1000)`,
//! so no K1 thread still runs code from the library when Godot unmaps it.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};

use lingara_embed::{EmbedEvent, EmbedStatus};

pub use crate::port::Core;

/// The deinit join's bound, K1 D2's.
pub const UNLOAD_JOIN_MS: u32 = 1000;

/// A closed handle, kept until it can be joined. `drained` once its one
/// `Closed` has been seen, so its thread is finishing on its own.
struct Retired {
    core: Box<dyn Core>,
    drained: bool,
}

/// What outlives every node: the closed handles, and how many are live
/// (D2's one-per-game warning).
#[derive(Default)]
pub struct Library {
    retired: Mutex<Vec<Retired>>,
    live: AtomicUsize,
}

/// The one the extension's nodes share. Tests make their own.
pub static LIBRARY: LazyLock<Arc<Library>> = LazyLock::new(Arc::default);

impl Library {
    fn retired(&self) -> MutexGuard<'_, Vec<Retired>> {
        self.retired.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn retire(&self, core: Box<dyn Core>, drained: bool) {
        self.retired().push(Retired { core, drained });
    }

    /// Drops every drained handle whose thread has already exited: a
    /// zero-timeout join, so a long session does not accumulate them.
    pub fn prune(&self) {
        self.retired().retain(|r| !(r.drained && r.core.close_and_wait(0)));
    }

    /// Godot's deinit: every listed handle joined, bounded.
    pub fn join_all(&self, timeout_ms: u32) {
        let all: Vec<Retired> = std::mem::take(&mut *self.retired());
        for retired in all {
            retired.core.close_and_wait(timeout_ms);
        }
    }

    /// How many handles are listed (the tests' view).
    #[cfg(test)]
    pub fn retired_count(&self) -> usize {
        self.retired().len()
    }

    #[cfg(test)]
    pub fn live_count(&self) -> usize {
        self.live.load(Ordering::SeqCst)
    }
}

/// One node's handle. `closing` after `stop()`: still drained until its
/// `Closed`, so the `closed` signal reaches the game.
pub struct Pump {
    library: Arc<Library>,
    current: Option<Box<dyn Core>>,
    closing: bool,
}

impl Pump {
    pub fn new(library: Arc<Library>) -> Self {
        Self { library, current: None, closing: false }
    }

    pub fn is_running(&self) -> bool {
        self.current.is_some() && !self.closing
    }

    /// The live handle, for a command.
    pub fn live(&self) -> Option<&dyn Core> {
        self.current.as_deref().filter(|_| !self.closing)
    }

    pub fn status(&self) -> Option<EmbedStatus> {
        self.current.as_ref().map(|c| c.status())
    }

    /// Installs a started handle. True when another node's handle was
    /// already live: two would mean two streams for one player.
    pub fn install(&mut self, core: Box<dyn Core>) -> bool {
        self.stop();
        if let Some(old) = self.current.take() {
            self.library.retire(old, false);
        }
        self.current = Some(core);
        self.closing = false;
        self.library.live.fetch_add(1, Ordering::SeqCst) > 0
    }

    /// K1's `close()`, never a join. The handle stays until its `Closed`
    /// has been drained.
    pub fn stop(&mut self) {
        self.library.prune();
        if let (Some(core), false) = (&self.current, self.closing) {
            core.close();
            self.closing = true;
            self.library.live.fetch_sub(1, Ordering::SeqCst);
        }
    }

    /// One frame: exactly one `drain(max)`. A drained `Closed` retires the
    /// handle to the library's list.
    pub fn frame(&mut self, max: u32) -> Vec<EmbedEvent> {
        let Some(core) = &self.current else { return Vec::new() };
        let events = core.drain(max);
        if events.iter().any(|e| matches!(e, EmbedEvent::Closed { .. })) {
            if !self.closing {
                // K1 closed on its own (`internal`): no stop() counted it down.
                self.library.live.fetch_sub(1, Ordering::SeqCst);
            }
            if let Some(core) = self.current.take() {
                self.library.retire(core, true);
            }
            self.closing = false;
        }
        events
    }
}

/// A node freed with its handle still listed hands it to the library, so
/// the deinit join still reaches it.
impl Drop for Pump {
    fn drop(&mut self) {
        self.stop();
        if let Some(core) = self.current.take() {
            self.library.retire(core, false);
        }
    }
}
