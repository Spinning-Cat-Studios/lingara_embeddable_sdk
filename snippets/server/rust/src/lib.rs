//! The Rust server kit's examples the docs show, on axum. Each region
//! builds in `make test-core`, so an example cannot drift from the kit.

use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::IntoResponse;
use lingara::BoxFuture;
use lingara_embed::server::{
    DirectoryError, EmbedServer, MintOptions, PlayerDirectory, PlayerRouter, ServerError, SnippetOptions, token_handoff, widget_snippet,
};

/// What your routes share.
#[derive(Clone)]
pub struct AppState {
    pub embed: EmbedServer,
    pub router: PlayerRouter,
}

/// Your own session lookup.
fn current_player(_headers: &HeaderMap) -> Option<String> {
    None
}

/// Built once, at startup, from your client credentials.
pub fn app_state(players: Arc<dyn PlayerDirectory>) -> Result<AppState, Box<dyn std::error::Error>> {
    let env = |name: &str| std::env::var(name).unwrap_or_default();
    let client = lingara::Client::builder().client_credentials(env("LINGARA_CLIENT_ID"), env("LINGARA_CLIENT_SECRET")).build()?;
    let embed = EmbedServer::new(client).with_directory(Arc::clone(&players));
    // lingara:begin webhook
    // Every live secret: two during a rotation, so either one verifies.
    let webhook = lingara::events::Webhook::with_secrets(env("LINGARA_WEBHOOK_SECRETS").split(','))?;
    let router = PlayerRouter::new(webhook, players);
    // lingara:end
    Ok(AppState { embed, router })
}

// lingara:begin mint
/// `POST /lingara/token`: your own route, behind your own session, and
/// rate-limited per session. The widget calls it with same-origin cookies.
pub async fn token_route(State(app): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let Some(player_ref) = current_player(&headers) else {
        return (StatusCode::UNAUTHORIZED, [(header::CACHE_CONTROL, "no-store")], String::new());
    };
    // This page's origin, one of your client's embed origins. A game's own
    // server mints without one.
    let options = MintOptions { origin: Some("https://game.example".into()), ..MintOptions::default() };
    match app.embed.mint_token(&player_ref, options).await {
        // {"token":"lgr_et_…","expires_at":"…"}
        Ok(minted) => (StatusCode::OK, [(header::CACHE_CONTROL, "no-store")], token_handoff(&minted)),
        Err(_) => (StatusCode::BAD_GATEWAY, [(header::CACHE_CONTROL, "no-store")], String::new()),
    }
}
// lingara:end

/// From your own account-deletion path.
pub async fn delete_account(app: &AppState, player_ref: &str) -> Result<(), ServerError> {
    // lingara:begin deletePlayer
    // Revokes every token the player holds, then forgets their subjects.
    // An unknown player is a success; on a failure the row is kept.
    app.embed.delete_player(player_ref).await?;
    // lingara:end
    Ok(())
}

// lingara:begin webhook
/// `POST /lingara/webhook`: the **raw** body, before any JSON parsing.
/// 204 handled, 401 a bad signature, 400 a malformed payload, 500 your
/// handler failed. Lingara redelivers every non-2xx; dedupe on the event id.
pub async fn webhook_route(State(app): State<AppState>, headers: HeaderMap, body: Bytes) -> StatusCode {
    let status = app
        .router
        .handle(&body, &headers, |routed| async move {
            match routed.player_ref {
                Some(player) => println!("{} for {player}", routed.event.event_type()),
                None => println!("{} for no player of ours", routed.event.event_type()),
            }
            Ok(()) // a slow handler enqueues and returns: delivery expects a 2xx within 10 s
        })
        .await;
    StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
}

/// Your own store, beside your player rows: `player_ref`, and `subject`
/// with a unique index (a player who links an account owns two subjects).
pub struct SqlPlayerDirectory {
    pub pool: Arc<tokio::sync::Mutex<Vec<(String, String)>>>, // stands in for your database pool
}

impl PlayerDirectory for SqlPlayerDirectory {
    fn remember<'a>(&'a self, player_ref: &'a str, subject: &'a str) -> BoxFuture<'a, Result<(), DirectoryError>> {
        // INSERT INTO lingara_players (player_ref, subject) VALUES ($1, $2)
        //   ON CONFLICT (subject) DO UPDATE SET player_ref = excluded.player_ref
        Box::pin(async move {
            let mut rows = self.pool.lock().await;
            rows.retain(|(_, s)| s != subject);
            rows.push((player_ref.to_string(), subject.to_string()));
            Ok(())
        })
    }

    fn player_ref_for<'a>(&'a self, subject: &'a str) -> BoxFuture<'a, Result<Option<String>, DirectoryError>> {
        // SELECT player_ref FROM lingara_players WHERE subject = $1
        Box::pin(async move { Ok(self.pool.lock().await.iter().find(|(_, s)| s == subject).map(|(p, _)| p.clone())) })
    }

    fn forget<'a>(&'a self, player_ref: &'a str) -> BoxFuture<'a, Result<(), DirectoryError>> {
        // DELETE FROM lingara_players WHERE player_ref = $1
        Box::pin(async move {
            self.pool.lock().await.retain(|(p, _)| p != player_ref);
            Ok(())
        })
    }
}
// lingara:end

/// The widget for one of your plans.
pub fn widget_markup(plan_id: &str, csp_nonce: &str) -> Result<String, lingara_embed::server::SnippetError> {
    // lingara:begin widgetSnippet
    // The element, its script build and a token provider that POSTs to your
    // token route. The token never appears in the markup.
    let html = widget_snippet(&SnippetOptions {
        locale: Some("fr".into()),
        nonce: Some(csp_nonce.into()), // your page's CSP nonce, for a strict script-src
        ..SnippetOptions::new(plan_id, "/lingara/token")
    })?;
    // lingara:end
    Ok(html)
}
