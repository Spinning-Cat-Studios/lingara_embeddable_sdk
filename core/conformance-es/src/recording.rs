//! What the harness observes beside the kit: a directory decorator that
//! counts `player_ref_for` calls (a lookup changes no contents, so only a
//! count shows a forged delivery never reached the store), and the log sink
//! that ES5's `render` sweeps.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use lingara::BoxFuture;
use lingara_embed::server::{DirectoryError, MemoryPlayerDirectory, PlayerDirectory};

/// The case's `MemoryPlayerDirectory`, with its lookups counted.
#[derive(Debug, Default)]
pub struct Recording {
    inner: MemoryPlayerDirectory,
    lookups: AtomicUsize,
}

impl Recording {
    pub async fn seeded(seed: &serde_json::Map<String, serde_json::Value>) -> Arc<Self> {
        let directory = Arc::new(Self::default());
        for (subject, player) in seed {
            let _ = directory.inner.remember(player.as_str().unwrap_or_default(), subject).await;
        }
        directory
    }

    pub fn lookups(&self) -> usize {
        self.lookups.load(Ordering::SeqCst)
    }

    pub fn snapshot(&self) -> BTreeMap<String, String> {
        self.inner.snapshot()
    }
}

impl PlayerDirectory for Recording {
    fn remember<'a>(&'a self, player_ref: &'a str, subject: &'a str) -> BoxFuture<'a, Result<(), DirectoryError>> {
        self.inner.remember(player_ref, subject)
    }

    fn player_ref_for<'a>(&'a self, subject: &'a str) -> BoxFuture<'a, Result<Option<String>, DirectoryError>> {
        self.lookups.fetch_add(1, Ordering::SeqCst);
        self.inner.player_ref_for(subject)
    }

    fn forget<'a>(&'a self, player_ref: &'a str) -> BoxFuture<'a, Result<(), DirectoryError>> {
        self.inner.forget(player_ref)
    }
}

/// Every line the routers' `on_error` and the `log` facade wrote.
#[derive(Clone, Default)]
pub struct Sink(Arc<Mutex<Vec<String>>>);

impl Sink {
    pub fn push(&self, line: String) {
        self.0.lock().unwrap_or_else(|p| p.into_inner()).push(line);
    }

    pub fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.0.lock().unwrap_or_else(|p| p.into_inner()))
    }
}

/// The `log` facade into the sink: the router's default `on_error` facility.
struct SinkLogger(Sink);

impl log::Log for SinkLogger {
    fn enabled(&self, _: &log::Metadata<'_>) -> bool {
        true
    }
    fn log(&self, record: &log::Record<'_>) {
        self.0.push(format!("{} {}: {}", record.level(), record.target(), record.args()));
    }
    fn flush(&self) {}
}

/// Installs the sink as the process's logger, once.
pub fn capture_logs(sink: Sink) {
    if log::set_boxed_logger(Box::new(SinkLogger(sink))).is_ok() {
        log::set_max_level(log::LevelFilter::Trace);
    }
}
