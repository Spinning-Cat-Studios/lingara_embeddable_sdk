//! The ES harness on the Rust server kit (ADR 1.10.26af D13): every ES case
//! through `lingara_embed::server` and nothing else.
//!
//! `conformance-server run --tier es --lang core -- <this>` starts it with
//! the server's URL, the results path, the receiver's two webhook secrets
//! and the visibly fake client credentials. It binds its webhook receiver to
//! port 0, prints `listening <port>`, and then for each case: fetches it,
//! builds a fresh client, server and directory, arms it, runs the `kit:`
//! jobs, waits for every delivery, judges what only the caller sees,
//! finishes, and writes one result line. There is no skip.

mod kit;
mod receiver;
mod recording;

use std::collections::HashMap;
use std::io::Write as _;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use lingara_embed::server::EmbedServer;
use serde_json::{Value, json};

use receiver::{Armed, Receiver, Seen};
use recording::{Recording, Sink};

/// The language this harness reports: `make/core.mk`'s `ES_LANGS` entry.
const LANG: &str = "core";
const SECRET_PREFIXES: [&str; 3] = ["lgr_et_", "lgr_cs_", "lgr_whsec_"];

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("{name} is not set"))
}

#[tokio::main]
async fn main() {
    let sink = Sink::default();
    recording::capture_logs(sink.clone());
    let named: Vec<String> = env("LINGARA_EMBED_WEBHOOK_SECRETS").split(',').map(String::from).collect();
    let secrets = ["primary", "secondary"].iter().zip(&named).map(|(n, s)| (n.to_string(), s.clone())).collect::<HashMap<_, _>>();
    let receiver = Receiver { armed: Arc::new(Mutex::new(None)), secrets: Arc::new(secrets), sink };
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.expect("bind the receiver");
    println!("listening {}", listener.local_addr().expect("an address").port());
    std::io::stdout().flush().expect("flush");
    let app = receiver::app(receiver.clone());
    tokio::spawn(async move { axum::serve(listener, app).await.expect("serve") });

    let mut out = std::fs::File::create(env("LINGARA_EMBED_RESULTS")).expect("the results file");
    let cases = std::env::var("LINGARA_EMBED_CASES").unwrap_or_else(|_| "conformance/es/cases".into());
    let http = reqwest::Client::new();
    for id in case_ids(Path::new(&cases)) {
        let line = run_case(&http, &receiver, &id).await;
        writeln!(out, "{line}").expect("write a result");
    }
}

/// `<group>/<slug>.yaml` → `<group>.<slug>`, sorted.
fn case_ids(dir: &Path) -> Vec<String> {
    let mut ids = Vec::new();
    for group in std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_dir()) {
        for file in std::fs::read_dir(&group).into_iter().flatten().flatten().map(|e| e.path()) {
            if file.extension().is_some_and(|e| e == "yaml") {
                let (g, s) = (group.file_name().unwrap_or_default(), file.file_stem().unwrap_or_default());
                ids.push(format!("{}.{}", g.to_string_lossy(), s.to_string_lossy()));
            }
        }
    }
    ids.sort();
    ids
}

async fn run_case(http: &reqwest::Client, receiver: &Receiver, id: &str) -> Value {
    let server = env("LINGARA_EMBED_SERVER_URL");
    let control = |verb: &str| format!("{server}/__conformance/{verb}/{id}");
    let case: Value = match http.get(control("cases")).send().await.and_then(|r| r.error_for_status()) {
        Ok(r) => r.json().await.unwrap_or(Value::Null),
        Err(e) => return json!({ "case": id, "lang": LANG, "result": "fail", "mismatches": [format!("cannot read the case: {e}")] }),
    };
    let empty = serde_json::Map::new();
    let directory = Recording::seeded(case.pointer("/directory/seed").and_then(Value::as_object).unwrap_or(&empty)).await;
    let deliver = case["deliver"].as_array().cloned().unwrap_or_default();
    let count = deliver.len();
    *receiver.lock() = Some(Armed { deliver, directory: Arc::clone(&directory), seen: vec![None; count], received: 0, answered: 0 });
    receiver.sink.take();
    let client = lingara::Client::builder()
        .client_credentials(env("LINGARA_EMBED_CLIENT_ID"), env("LINGARA_EMBED_CLIENT_SECRET"))
        .base_url(server.clone())
        .token_url(format!("{server}/oauth/token"))
        .build()
        .expect("a client");
    let embed = EmbedServer::new(client).with_directory(directory.clone());

    let mut mismatches: Vec<Value> = Vec::new();
    if let Err(e) = http.post(control("arm")).send().await.and_then(|r| r.error_for_status()) {
        mismatches.push(format!("cannot arm: {e}").into());
    }
    let outcome = kit::run(&embed, case["kit"].as_array().map(Vec::as_slice).unwrap_or_default()).await;
    let seen = wait_for_deliveries(receiver, count).await;
    mismatches.extend(kit::judge(id, &case["expect"], &outcome, &directory));
    mismatches.extend(handler_mismatches(id, &case, &seen));
    if outcome.render {
        mismatches.extend(render_mismatches(id, receiver, &embed, &directory, &outcome));
    }
    let handlers: Vec<Option<String>> = seen.iter().map(|s| s.as_ref().and_then(|s| s.player_ref.clone())).collect();
    match http.post(control("finish")).json(&json!({ "handlers": handlers })).send().await {
        Ok(r) => mismatches.extend(r.json::<Value>().await.ok().and_then(|v| v["mismatches"].as_array().cloned()).unwrap_or_default()),
        Err(e) => mismatches.push(format!("cannot finish: {e}").into()),
    }
    *receiver.lock() = None;
    let result = if mismatches.is_empty() { "pass" } else { "fail" };
    json!({ "case": id, "lang": LANG, "result": result, "mismatches": mismatches })
}

/// Until every delivery has been answered, or 15 s; then what each saw.
async fn wait_for_deliveries(receiver: &Receiver, count: usize) -> Vec<Option<Seen>> {
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let done = receiver.lock().as_ref().is_none_or(|a| a.answered >= count);
        if done || Instant::now() > deadline {
            return receiver.lock().as_ref().map(|a| a.seen.clone()).unwrap_or_default();
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// Each delivery's `expect.handler` against what its `on_event` saw.
fn handler_mismatches(id: &str, case: &Value, seen: &[Option<Seen>]) -> Vec<Value> {
    let items = case["deliver"].as_array().cloned().unwrap_or_default();
    let mut found = Vec::new();
    for (n, item) in items.iter().enumerate() {
        let want = &item["expect"]["handler"];
        let got = seen.get(n).cloned().flatten();
        let ok = match (want.as_str(), &got) {
            (Some("not_called"), None) => true,
            (None, Some(s)) => {
                let player = want.get("player_ref").map(|p| p.as_str().map(String::from));
                player.is_none_or(|p| p == s.player_ref)
                    && want.get("event_id").and_then(Value::as_str).is_none_or(|e| e == s.event_id)
                    && want.get("unknown").and_then(Value::as_bool).is_none_or(|u| u == s.unknown)
            }
            _ => false,
        };
        if !ok {
            found.push(json!(format!("{id}: fail handler: deliver[{n}] saw {got:?}, expected {want}")));
        }
    }
    found
}

/// ES5: no rendering of a kit object, and no captured log line, carries a secret prefix.
fn render_mismatches(id: &str, receiver: &Receiver, embed: &EmbedServer, directory: &Arc<Recording>, out: &kit::Outcome) -> Vec<Value> {
    let router = receiver.router(&json!({ "router": { "secrets": ["primary", "secondary"] } }), Arc::clone(directory));
    let mut renderings = vec![format!("{embed:?}"), format!("{router:?}"), format!("{directory:?}"), format!("{:?}", out.minted)];
    if let Some(minted) = &out.minted {
        renderings.push(format!("{:?}", minted.token));
    }
    renderings.extend(receiver.sink.take());
    let leaks: Vec<&String> = renderings.iter().filter(|r| SECRET_PREFIXES.iter().any(|p| r.contains(p))).collect();
    if leaks.is_empty() { Vec::new() } else { vec![json!(format!("{id}: fail render: {} rendering(s) carry a secret prefix", leaks.len()))] }
}
