//! `lingara-embed-uniffi`: the native library the JVM kit loads (ADR
//! 1.10.26ae D2). A thin UniFFI 0.31 wrapper over K1's `lingara-embed`: one
//! object, `EmbedHandle`, mirrors of K1's types with signed integers only,
//! and two free functions that read and write K1's own JSON.
//!
//! No callback interface and no `async` export: the game polls (roadmap
//! D3), so UniFFI emits no coroutine runtime into the Kotlin.
//!
//! Beside UniFFI's symbols sits one plain C function,
//! `lingara_embed_uniffi_version`, which `NativeLoader` calls before the
//! generated Kotlin first loads (D5).

// The mirrors are plain data passed by value, as K1's are: boxing a large
// variant would change nothing on the Kotlin side, so K1's answer to the
// size lints is this crate's too.
#![allow(clippy::large_enum_variant, clippy::result_large_err)]

mod config;
mod convert;
mod dialogue;
mod graph;
mod handle;
mod practice;
mod records;
mod types;

#[cfg(test)]
mod tests;

use std::ffi::c_char;

pub use convert::unknown_from_json;
pub use handle::EmbedHandle;
pub use types::*;

uniffi::setup_scaffolding!();

/// The crate version, NUL-terminated, for the C symbol below.
const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "\0");

/// A static NUL-terminated copy of the crate version (D5). Kotlin compares
/// it with the version it was built at, so a stale natives artefact fails
/// with both versions named rather than a checksum mismatch.
#[unsafe(no_mangle)]
pub extern "C" fn lingara_embed_uniffi_version() -> *const c_char {
    VERSION.as_ptr().cast()
}

/// An event in K1's JSON shape, `Unknown` as the K1 JSON it carries, for a
/// harness or a game's logger. No event holds a secret.
#[uniffi::export]
pub fn event_to_json(event: EmbedEvent) -> String {
    match event {
        EmbedEvent::Unknown { json, .. } => json,
        known => serde_json::to_string(&known).unwrap_or_default(),
    }
}

/// A command from K1's JSON shape, checked by K1's own deserialiser first.
#[uniffi::export]
pub fn command_from_json(json: String) -> Result<Command, EmbedException> {
    serde_json::from_str::<lingara_embed::Command>(&json).map_err(|e| convert::invalid("command", e))?;
    serde_json::from_str(&json).map_err(|e| convert::invalid("command", e))
}
