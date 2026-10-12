//! The player directory (ADR 1.10.26af D5): the one mapping a kit cannot
//! derive. `subject` is a pairwise HMAC computed by Lingara, so the mint's
//! answer is the only moment a `player_ref` and its `subject` are in one
//! place; the integrator's store keeps them.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Mutex;

use lingara::BoxFuture;

/// Whatever the integrator's store raises. The kit propagates it unchanged.
pub type DirectoryError = Box<dyn std::error::Error + Send + Sync>;

/// The integrator's store of `player_ref` ↔ `subject`, unique on `subject`
/// only: a player who links an account owns a second subject beside the
/// guest's. Implement it over your own database, beside your player rows.
pub trait PlayerDirectory: Send + Sync {
    /// Upserts the row keyed on `subject`. A guest's subject is stable
    /// across mints, so remembering it again changes nothing.
    fn remember<'a>(&'a self, player_ref: &'a str, subject: &'a str) -> BoxFuture<'a, Result<(), DirectoryError>>;

    /// The player a subject belongs to, or `None` for one this store never
    /// saw (the integrator's own subject, a deleted player).
    fn player_ref_for<'a>(&'a self, subject: &'a str) -> BoxFuture<'a, Result<Option<String>, DirectoryError>>;

    /// Removes every row for the player. Idempotent.
    fn forget<'a>(&'a self, player_ref: &'a str) -> BoxFuture<'a, Result<(), DirectoryError>>;
}

/// A process-local directory, for tests and single-process demos. It is
/// lost on restart and not shared between instances: production code
/// implements [`PlayerDirectory`] over its own database.
#[derive(Default)]
pub struct MemoryPlayerDirectory {
    rows: Mutex<BTreeMap<String, String>>,
}

impl MemoryPlayerDirectory {
    pub fn new() -> Self {
        Self::default()
    }

    /// Every row, subject → player_ref.
    pub fn snapshot(&self) -> BTreeMap<String, String> {
        self.rows().clone()
    }

    fn rows(&self) -> std::sync::MutexGuard<'_, BTreeMap<String, String>> {
        self.rows.lock().unwrap_or_else(|p| p.into_inner())
    }
}

impl PlayerDirectory for MemoryPlayerDirectory {
    fn remember<'a>(&'a self, player_ref: &'a str, subject: &'a str) -> BoxFuture<'a, Result<(), DirectoryError>> {
        self.rows().insert(subject.to_string(), player_ref.to_string());
        Box::pin(async { Ok(()) })
    }

    fn player_ref_for<'a>(&'a self, subject: &'a str) -> BoxFuture<'a, Result<Option<String>, DirectoryError>> {
        let found = self.rows().get(subject).cloned();
        Box::pin(async move { Ok(found) })
    }

    fn forget<'a>(&'a self, player_ref: &'a str) -> BoxFuture<'a, Result<(), DirectoryError>> {
        self.rows().retain(|_, owner| owner != player_ref);
        Box::pin(async { Ok(()) })
    }
}

impl fmt::Debug for MemoryPlayerDirectory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MemoryPlayerDirectory").field("rows", &self.rows().len()).finish()
    }
}
