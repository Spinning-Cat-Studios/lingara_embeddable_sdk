//! A poisoned lock is recovered, never propagated: a panic on one task is
//! reported once as `Closed { Internal }` (D2), and must not turn every
//! later engine-thread call into a second panic.

use std::sync::{Mutex, MutexGuard, PoisonError};

pub(crate) fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}
