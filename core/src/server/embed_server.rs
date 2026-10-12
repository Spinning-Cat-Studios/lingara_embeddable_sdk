//! Mint and delete (ADR 1.10.26af D4, D9): one library call each, the
//! directory written only after a success. There is no cache: every
//! `mint_token` is one `POST`, so a token `delete_player` revoked on another
//! instance is never handed out again.

use std::fmt;
use std::sync::Arc;

use lingara::embed::MintedToken;
use lingara::models::EmbedTokenRequest;

use super::directory::{DirectoryError, PlayerDirectory};

/// What a mint may send beside the player: both optional, and sent only
/// when given. `origin` is your page's origin, one of your client's embed
/// origins; a game's server mints without one.
#[derive(Clone, Debug, Default)]
pub struct MintOptions {
    pub scopes: Option<Vec<String>>,
    pub origin: Option<String>,
}

/// Why a job failed. A failed API call is the library's error, unchanged.
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error(transparent)]
    Library(#[from] lingara::Error),
    /// A value the request model refuses before anything is sent (a
    /// `player_ref` over 128 characters, an unknown scope). It never
    /// carries the value.
    #[error("{field} is not a value Lingara accepts: {reason}")]
    Request { field: &'static str, reason: String },
    /// The integrator's directory raised.
    #[error("the player directory failed: {0}")]
    Directory(DirectoryError),
}

/// Holds the integrator's `lingara` client and, optionally, their directory.
#[derive(Clone)]
pub struct EmbedServer {
    client: lingara::Client,
    directory: Option<Arc<dyn PlayerDirectory>>,
}

impl EmbedServer {
    /// Over a client built with your client credentials. Its token cache,
    /// retries and deprecation hook are the library's.
    pub fn new(client: lingara::Client) -> Self {
        Self { client, directory: None }
    }

    /// Records each mint's subject, and forgets a deleted player.
    #[must_use]
    pub fn with_directory(mut self, directory: Arc<dyn PlayerDirectory>) -> Self {
        self.directory = Some(directory);
        self
    }

    /// Mints a token for one player: one `POST /v1/embed/tokens`, then
    /// `remember(player_ref, subject)`. On a failure nothing is remembered.
    pub async fn mint_token(&self, player_ref: &str, options: MintOptions) -> Result<MintedToken, ServerError> {
        let request = request(player_ref, options)?;
        let minted = self.client.create_embed_token(&request).await?.into_inner();
        if let Some(directory) = &self.directory {
            directory.remember(player_ref, &minted.subject).await.map_err(ServerError::Directory)?;
        }
        Ok(minted)
    }

    /// Deletes a player and revokes their tokens, then forgets them. An
    /// unknown player is a success; on a failure the row is kept.
    pub async fn delete_player(&self, player_ref: &str) -> Result<(), ServerError> {
        self.client.delete_embed_player(player_ref).await?;
        if let Some(directory) = &self.directory {
            directory.forget(player_ref).await.map_err(ServerError::Directory)?;
        }
        Ok(())
    }
}

/// The library's request model, the options present only when given.
fn request(player_ref: &str, options: MintOptions) -> Result<EmbedTokenRequest, ServerError> {
    let refused = |field: &'static str| move |e: lingara::models::error::ConversionError| ServerError::Request { field, reason: e.to_string() };
    let scopes = options.scopes.map(|s| s.iter().map(|x| x.parse().map_err(|_| ServerError::Request { field: "scopes", reason: "an unknown scope".into() })).collect());
    Ok(EmbedTokenRequest {
        player_ref: player_ref.parse().map_err(refused("player_ref"))?,
        scopes: scopes.transpose()?,
        origin: options.origin.map(|o| o.parse().map_err(refused("origin"))).transpose()?,
    })
}

impl fmt::Debug for EmbedServer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EmbedServer").field("client", &self.client).field("directory", &self.directory.is_some()).finish()
    }
}
