#![doc = include_str!("../README.md")]
// The public enums are plain data, marshalled by value through four FFI
// layers (D10): boxing a large variant would put a `Box` in every binding's
// mirror of it, so the size lints are answered here, once.
#![allow(clippy::large_enum_variant, clippy::result_large_err)]

#[cfg(target_arch = "wasm32")]
compile_error!("lingara-embed does not target wasm32 (roadmap 1.10.26q D8): in a browser, use the widget");

mod api;
mod commands;
mod config;
mod dialogue;
mod embed;
mod error;
mod events;
mod library;
pub mod limits;
mod practice;
mod queue;
mod runtime;
#[cfg(feature = "server")]
pub mod server;
mod token;
pub mod types;
mod world;

#[cfg(test)]
mod tests;

// The helpers' short paths, crate-wide.
pub(crate) use runtime::{ids, shared, sync};
pub(crate) use token::{bridge as token_bridge, fetch as token_fetch, handoff, rfc3339};
pub(crate) use types::secret;

/// The `dialogue.v1` IR, its checks and its predicate language (ADR 28.10.26f D11).
pub use lingara_dialogue_ir as dialogue_ir;

pub use embed::Embed;
pub use handoff::{BoxFuture, EmbedTokenSource, IssuedToken, stale_after};
pub use secret::REDACTED;
pub use token_fetch::EMBED_VERSION_HEADER;
pub use types::*;
