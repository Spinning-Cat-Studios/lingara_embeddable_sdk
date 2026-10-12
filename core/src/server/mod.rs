//! The server tier (ADR 1.10.26af), behind the `server` feature: what a
//! game's or a site's own server does to vouch for a player. Four jobs, each
//! a thin layer over the `lingara` library this crate already depends on:
//!
//! - [`EmbedServer::mint_token`] mints a player's token and remembers its
//!   `subject` in your [`PlayerDirectory`];
//! - [`token_handoff`] is the body your token route returns to the player,
//!   and [`widget_snippet`] the markup that mounts the web widget;
//! - [`PlayerRouter::handle`] verifies a webhook and resolves it to a player;
//! - [`EmbedServer::delete_player`] forgets a player, at Lingara and here.
//!
//! ```no_run
//! # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
//! use std::sync::Arc;
//! use lingara_embed::server::{EmbedServer, MemoryPlayerDirectory, MintOptions, token_handoff};
//!
//! let client = lingara::Client::builder().client_credentials("lgr_ci_…", "lgr_cs_…").build()?;
//! let server = EmbedServer::new(client).with_directory(Arc::new(MemoryPlayerDirectory::new()));
//! let minted = server.mint_token("player-42", MintOptions::default()).await?;
//! let body = token_handoff(&minted); // {"token":"lgr_et_…","expires_at":"…"}
//! # let _ = body; Ok(()) }
//! ```

mod directory;
mod embed_server;
mod handoff;
mod router;
mod snippet_rules;
mod widget_snippet;

#[cfg(test)]
mod router_tests;
#[cfg(test)]
mod vectors_tests;

pub use directory::{DirectoryError, MemoryPlayerDirectory, PlayerDirectory};
pub use embed_server::{EmbedServer, MintOptions, ServerError};
pub use handoff::token_handoff;
pub use lingara::embed::MintedToken;
pub use router::{HandleFailure, PlayerRouter, RouteError, RoutedEvent};
pub use widget_snippet::{DEFAULT_ELEMENT_SRC, SnippetError, SnippetOptions, widget_snippet};
