//! The one id counter (D3): `RequestId`, `SessionId` and `DialogueId` all
//! come from it, so no two kinds collide, and a binding that flattens them
//! into one integer type loses nothing.

use std::sync::atomic::{AtomicU64, Ordering};

pub(crate) struct Ids(AtomicU64);

impl Default for Ids {
    fn default() -> Self {
        Self(AtomicU64::new(1))
    }
}

impl Ids {
    /// From 1, never above `i64::MAX`, so a Godot int or a JVM `Long` holds it.
    pub fn next(&self) -> u64 {
        self.0.fetch_add(1, Ordering::Relaxed).min(i64::MAX as u64)
    }
}

/// 16 random bytes in hex: an idempotency key or a session key (D6, D7).
pub(crate) fn random_key() -> String {
    let mut bytes = [0u8; 16];
    if getrandom::fill(&mut bytes).is_err() {
        // No CSPRNG: the clock and a process-wide counter still make every
        // key distinct, which is all an idempotency key needs.
        static FALLBACK: AtomicU64 = AtomicU64::new(0);
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        bytes[..8].copy_from_slice(&(nanos as u64).to_le_bytes());
        bytes[8..].copy_from_slice(&FALLBACK.fetch_add(1, Ordering::Relaxed).to_le_bytes());
    }
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
