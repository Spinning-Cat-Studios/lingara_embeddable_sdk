//! A case's `kit:` jobs through `EmbedServer`, and the comparisons only the
//! kit's caller can make (ES `CONTRACT.md`, *What a harness reports*).

use std::sync::Arc;

use lingara_embed::server::{EmbedServer, MintOptions, MintedToken, ServerError, token_handoff};
use serde_json::{Value, json};

use crate::recording::Recording;

/// What the jobs left behind.
#[derive(Default)]
pub struct Outcome {
    pub minted: Option<MintedToken>,
    pub handoff: Option<String>,
    pub error: Option<Value>,
    pub render: bool,
}

/// Every job in order; a raised error is recorded and the next job runs.
pub async fn run(server: &EmbedServer, jobs: &[Value]) -> Outcome {
    let mut out = Outcome::default();
    for job in jobs {
        let result = match (job.get("mint"), job.get("delete")) {
            (Some(m), _) => server.mint_token(text(m, "player_ref"), options(m)).await.map(|t| out.minted = Some(t)),
            (_, Some(d)) => server.delete_player(text(d, "player_ref")).await,
            _ if job.get("handoff").is_some() => {
                out.handoff = out.minted.as_ref().map(token_handoff);
                Ok(())
            }
            _ => {
                out.render = true;
                Ok(())
            }
        };
        if let Err(e) = result {
            out.error = Some(raised(&e));
        }
    }
    out
}

fn text<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or_default()
}

fn options(m: &Value) -> MintOptions {
    let scopes = m.get("scopes").and_then(Value::as_array).map(|s| s.iter().filter_map(Value::as_str).map(String::from).collect());
    MintOptions { scopes, origin: m.get("origin").and_then(Value::as_str).map(String::from) }
}

/// `{status, code}`: an API error's, an OAuth error's `error`, `maintenance`.
fn raised(e: &ServerError) -> Value {
    let (status, code) = match e {
        ServerError::Library(lingara::Error::Api(a)) => (a.status, a.code.clone()),
        ServerError::Library(lingara::Error::OAuth(o)) => (o.status, o.error.clone()),
        ServerError::Library(lingara::Error::Maintenance(_)) => (503, "maintenance".into()),
        ServerError::Library(lingara::Error::Transport(t)) => (0, t.kind.as_str().into()),
        ServerError::Library(other) => (0, other.to_string()),
        ServerError::Request { field, .. } => (0, format!("request_{field}")),
        ServerError::Directory(_) => (0, "directory".into()),
    };
    json!({ "status": status, "code": code })
}

/// `value` with every null object member dropped, at any depth.
pub fn without_nulls(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(map.iter().filter(|(_, v)| !v.is_null()).map(|(k, v)| (k.clone(), without_nulls(v))).collect()),
        Value::Array(items) => Value::Array(items.iter().map(without_nulls).collect()),
        other => other.clone(),
    }
}

/// The six fields in wire form, the token read through its accessor.
fn wire(t: &MintedToken) -> Value {
    json!({
        "token": t.token.expose_secret(), "expires_at": t.expires_at, "expires_in": t.expires_in.as_secs(),
        "subject": t.subject, "scopes": t.scopes, "account_linked": t.account_linked,
    })
}

/// `expect.minted`, `expect.handoff`, `expect.error`, `expect.directory` and
/// `expect.directory_lookups`, each a `fail <key>` line when it differs.
pub fn judge(id: &str, expect: &Value, out: &Outcome, directory: &Arc<Recording>) -> Vec<Value> {
    let got = |key: &str| -> Option<Value> {
        match key {
            "minted" => out.minted.as_ref().map(wire),
            "handoff" => out.handoff.clone().map(Value::from),
            "error" => out.error.clone(),
            "directory" => Some(json!(directory.snapshot())),
            _ => Some(json!(directory.lookups())),
        }
    };
    let mut found = Vec::new();
    for key in ["minted", "handoff", "error", "directory", "directory_lookups"] {
        let (want, have) = (expect.get(key).map(without_nulls), got(key).map(|v| without_nulls(&v)));
        // A key the case names must match; an error the case did not
        // expect is a mismatch too.
        let differs = match &want {
            Some(want) => have.as_ref() != Some(want),
            None => key == "error" && have.is_some(),
        };
        if differs {
            found.push(json!(format!("{id}: fail {key}: got {}, expected {}", show(&have), show(&want))));
        }
    }
    found
}

fn show(v: &Option<Value>) -> String {
    v.as_ref().map_or("nothing".into(), Value::to_string)
}
