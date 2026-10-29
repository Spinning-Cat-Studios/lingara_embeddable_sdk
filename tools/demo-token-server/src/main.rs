//! A stand-in for a game's own server (ADR 1.10.26z D11): one route,
//! `POST /token`, that mints an embed token for one demo player and answers
//! exactly `{"token","expires_at"}`. It authenticates nobody. A real game's
//! server mints only for a player it has authenticated, and rate-limits its
//! own route per session.
//!
//! It binds loopback only and prints `listening <port>` first, so a test can
//! run it on `--port 0`. `--fake` mints nothing, needs no secret and no
//! network, and answers a visibly fake `lgr_et_` token: CI runs only that.
//! `LINGARA_BASE_URL` (and `LINGARA_TOKEN_URL`, else `<base>/oauth/token`)
//! point it at another Lingara, a local stack say (ADR 15.10.26g).

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::Router;
use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use lingara::AccessToken;
use lingara::embed::MintedToken;
use lingara_embed::server::{EmbedServer, MintOptions, token_handoff};
use tokio::net::TcpListener;

#[cfg(test)]
mod main_tests;

/// The five scopes a player token may hold. No origin: a native kit never
/// sends `X-Lingara-Embed-Origin`, so an origin-bound token would be refused.
const PLAYER_SCOPES: [&str; 5] = ["events:read", "events:write", "lesson_plans:read", "lesson_plans:write", "embed:play"];
const FAKE_TOKEN: &str = "lgr_et_FAKE_demo_token_not_valid";
const LIFETIME_SECS: u64 = 900;

#[derive(Debug, PartialEq, Eq)]
struct Options {
    port: u16,
    player_ref: String,
    fake: bool,
    credentials: Option<(String, String)>,
    base_url: Option<String>,
    token_url: Option<String>,
}

/// `--port`, `--player-ref`, `--fake`; the secret and any other Lingara from the environment.
fn options(args: &[String], env: impl Fn(&str) -> Option<String>) -> Result<Options, String> {
    let mut options = Options { port: 8787, player_ref: "demo-player-1".into(), fake: false, credentials: None, base_url: None, token_url: None };
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--fake" => options.fake = true,
            "--port" => options.port = args.next().and_then(|p| p.parse().ok()).ok_or("--port takes a number")?,
            "--player-ref" => options.player_ref = args.next().cloned().ok_or("--player-ref takes a value")?,
            other => return Err(format!("unknown argument {other}")),
        }
    }
    options.credentials = env("LINGARA_CLIENT_ID").zip(env("LINGARA_CLIENT_SECRET")).filter(|(id, secret)| !id.is_empty() && !secret.is_empty());
    let set = |k: &str| env(k).filter(|v| !v.is_empty());
    options.base_url = set("LINGARA_BASE_URL").map(|url| url.trim_end_matches('/').to_owned());
    // A token from one Lingara is refused by another, so the exchange follows the base by default.
    options.token_url = set("LINGARA_TOKEN_URL").or_else(|| options.base_url.as_ref().map(|base| format!("{base}/oauth/token")));
    if !options.fake && options.credentials.is_none() {
        return Err("set LINGARA_CLIENT_ID and LINGARA_CLIENT_SECRET (a metered client's), or pass --fake".into());
    }
    Ok(options)
}

/// Where tokens come from: the integrator's client, or nowhere.
enum Minter {
    Lingara(EmbedServer),
    Fake,
}

struct Server {
    minter: Minter,
    player_ref: String,
}

/// `now + secs` as RFC 3339 (civil-from-days, so no date crate).
fn rfc3339_after(secs: u64) -> String {
    let t = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) + secs;
    let (days, rem) = ((t / 86_400) as i64, t % 86_400);
    let z = days + 719_468;
    let (era, doe) = (z.div_euclid(146_097), z.rem_euclid(146_097));
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let (d, m) = (doy - (153 * mp + 2) / 5 + 1, if mp < 10 { mp + 3 } else { mp - 9 });
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, rem % 3600 / 60, rem % 60)
}

fn fake_token() -> MintedToken {
    MintedToken {
        token: AccessToken::new(FAKE_TOKEN),
        expires_at: rfc3339_after(LIFETIME_SECS),
        expires_in: Duration::from_secs(LIFETIME_SECS),
        subject: "lgr_sub_fake".into(),
        scopes: PLAYER_SCOPES.map(String::from).to_vec(),
        account_linked: false,
    }
}

/// The request body is ignored: K1's Http source sends `{}`, Godot may send none.
async fn token(State(server): State<Arc<Server>>) -> Response {
    let minted = match &server.minter {
        Minter::Fake => Ok(fake_token()),
        Minter::Lingara(embed) => {
            let options = MintOptions { scopes: Some(PLAYER_SCOPES.map(String::from).to_vec()), origin: None };
            embed.mint_token(&server.player_ref, options).await
        }
    };
    match minted {
        Ok(minted) => ([(header::CONTENT_TYPE, "application/json"), (header::CACHE_CONTROL, "no-store")], token_handoff(&minted)).into_response(),
        Err(e) => (StatusCode::BAD_GATEWAY, [(header::CACHE_CONTROL, "no-store")], format!("the mint failed: {e}")).into_response(),
    }
}

fn app(server: Server) -> Router {
    Router::new().route("/token", post(token)).with_state(Arc::new(server))
}

fn server(options: &Options) -> Result<Server, String> {
    let minter = match &options.credentials {
        Some((id, secret)) if !options.fake => {
            let mut builder = lingara::Client::builder().client_credentials(id, secret);
            if let Some(url) = &options.base_url {
                builder = builder.base_url(url);
            }
            if let Some(url) = &options.token_url {
                builder = builder.token_url(url);
            }
            let client = builder.build().map_err(|e| e.to_string())?;
            Minter::Lingara(EmbedServer::new(client))
        }
        _ => Minter::Fake,
    };
    Ok(Server { minter, player_ref: options.player_ref.clone() })
}

/// Loopback only: an emulator reaches it through `adb reverse`.
async fn bind(port: u16) -> std::io::Result<TcpListener> {
    TcpListener::bind(("127.0.0.1", port)).await
}

/// The first stdout line, which a test or a demo waits for.
fn listening(port: u16) -> String {
    format!("listening {port}")
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let started = options(&args, |k| std::env::var(k).ok()).and_then(|o| server(&o).map(|s| (o.port, s)));
    let (port, server) = started.unwrap_or_else(|e| {
        eprintln!("lingara-demo-token-server: {e}");
        std::process::exit(2);
    });
    let listener = bind(port).await.unwrap_or_else(|e| {
        eprintln!("lingara-demo-token-server: cannot bind 127.0.0.1:{port}: {e}");
        std::process::exit(1);
    });
    let port = listener.local_addr().map(|a| a.port()).unwrap_or(port);
    println!("{}", listening(port));
    if let Err(e) = axum::serve(listener, app(server)).await {
        eprintln!("lingara-demo-token-server: {e}");
    }
}
