#![doc = include_str!("../README.md")]
// `lingara-embed`'s plain-data enums are large by design (they cross four
// FFI layers by value); the Messages that carry them inherit the lint.
#![allow(clippy::large_enum_variant, clippy::result_large_err)]
// No `compile_error!` for wasm32 here (D9): Cargo builds `lingara-embed`
// first, and its own refusal names the reason.

mod config;
mod core;
mod dialogue;
mod frame;
mod lifecycle;
mod messages;
mod plugin;
mod practice;
mod token;

#[cfg(test)]
mod tests;

pub use config::{DEFAULT_MAX_EVENTS_PER_FRAME, LingaraConfig};
pub use core::Lingara;
pub use dialogue::{NpcDialogue, NpcTranscript, Turn};
pub use messages::*;
pub use plugin::{LingaraPlugin, LingaraSystems};
pub use practice::{PracticeAction, PracticeSession};

/// `lingara-embed`, re-exported at the version this plugin pins, so a game
/// names K1's types (`WorldContext`, `PracticeSource`, `Header`) without a
/// second dependency line that could drift.
pub use lingara_embed;
