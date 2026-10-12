//! The I/O thread (D2): one OS thread named `lingara-embed`, running one
//! current-thread tokio runtime. Every task runs there; the engine thread
//! only hands work to it through the runtime's handle, whose spawn never
//! blocks. Shutdown drops the runtime, which cancels every task, and
//! dropping a task drops its connection.

pub(crate) mod ids;
pub(crate) mod shared;
pub(crate) mod sync;

use std::future::Future;
use std::sync::mpsc;
use std::sync::{Arc, Mutex, Weak};
use std::thread::JoinHandle;
use std::time::Duration;

use tokio::runtime::Handle;
use tokio::sync::oneshot;
use tokio::task::AbortHandle;

use crate::error::{EmbedError, ErrorKind};
use crate::shared::Shared;
use crate::sync::lock;

pub(crate) struct IoThread {
    handle: Handle,
    shutdown: Mutex<Option<oneshot::Sender<()>>>,
    done: Mutex<Option<mpsc::Receiver<()>>>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

/// The current-thread runtime, with I/O and time. `paused` starts tokio's
/// clock paused, for the unit tests that jump a 60 s restart.
fn build(paused: bool) -> std::io::Result<tokio::runtime::Runtime> {
    let mut builder = tokio::runtime::Builder::new_current_thread();
    builder.enable_all();
    #[cfg(test)]
    builder.start_paused(paused);
    #[cfg(not(test))]
    let _ = paused;
    builder.build()
}

#[cfg(test)]
thread_local! {
    /// I/O threads started from this thread: a test's proof that a refused
    /// config spawned none.
    pub(crate) static SPAWNED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

impl IoThread {
    pub fn spawn(paused: bool) -> Result<Self, EmbedError> {
        #[cfg(test)]
        SPAWNED.with(|n| n.set(n.get() + 1));
        let (handle_tx, handle_rx) = mpsc::channel();
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let (done_tx, done_rx) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("lingara-embed".into())
            .spawn(move || {
                match build(paused) {
                    Ok(runtime) => {
                        let _ = handle_tx.send(Ok(runtime.handle().clone()));
                        runtime.block_on(async {
                            let _ = shutdown_rx.await;
                        });
                        // Dropping the runtime cancels every task it holds.
                        drop(runtime);
                    }
                    Err(e) => {
                        let _ = handle_tx.send(Err(e.to_string()));
                    }
                }
                let _ = done_tx.send(());
            })
            .map_err(|e| EmbedError::new(ErrorKind::Internal, format!("cannot start the lingara-embed thread: {e}")))?;
        let handle = match handle_rx.recv() {
            Ok(Ok(handle)) => handle,
            Ok(Err(e)) => return Err(EmbedError::new(ErrorKind::Internal, format!("cannot build the I/O runtime: {e}"))),
            Err(_) => return Err(EmbedError::new(ErrorKind::Internal, "the lingara-embed thread exited at start")),
        };
        Ok(Self { handle, shutdown: Mutex::new(Some(shutdown_tx)), done: Mutex::new(Some(done_rx)), thread: Mutex::new(Some(thread)) })
    }

    pub fn handle(&self) -> &Handle {
        &self.handle
    }

    /// Signals the thread and returns at once.
    pub fn stop(&self) {
        if let Some(tx) = lock(&self.shutdown).take() {
            let _ = tx.send(());
        }
    }

    /// Waits at most `timeout` for the thread to exit, then joins it.
    pub fn join(&self, timeout: Duration) -> bool {
        let Some(done) = lock(&self.done).take() else { return lock(&self.thread).is_none() };
        if done.recv_timeout(timeout).is_err() {
            *lock(&self.done) = Some(done);
            return false;
        }
        // Joining the I/O thread from itself would deadlock; a task that
        // closes the handle never waits.
        let thread = lock(&self.thread).take();
        if let Some(thread) = thread.filter(|t| t.thread().id() != std::thread::current().id()) {
            let _ = thread.join();
        }
        true
    }
}

/// Spawns `task` on the I/O thread. A panic inside it is caught at the task
/// boundary, logged, and closes the handle with `Closed { Internal }`.
pub(crate) fn spawn(shared: &Arc<Shared>, task: impl Future<Output = ()> + Send + 'static) -> AbortHandle {
    let handle = shared.io.handle();
    let joined = handle.spawn(task);
    let abort = joined.abort_handle();
    let weak: Weak<Shared> = Arc::downgrade(shared);
    handle.spawn(async move {
        if let Err(e) = joined.await {
            if e.is_panic() {
                if let Some(shared) = weak.upgrade() {
                    shared.close_internal("a task on the lingara-embed thread panicked");
                }
            }
        }
    });
    abort
}
