//! The server itself (ADR 1.10.26x D5): the `/__conformance` control routes
//! and the scripted answers. Every other request is judged against the armed
//! case's next step and answered with that step's `respond`.
//!
//! `serve --cases <dir>` is this app with no harness: a client arms and
//! finishes cases through the same control routes, and each finish answers
//! the case's verdict, so no results file is needed.

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use axum::Router;
use axum::body::{Body, Bytes};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, Method, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use serde::Deserialize;
use serde_json::json;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

use crate::case::{Respond, ServeStep};
use crate::judge::{judge, Seen};
use crate::sign;
use crate::state::{Armed, LogEntry, Shared};

/// How long a closed kit has to drop every stream (EK9).
const STREAM_DROP_WITHIN: Duration = Duration::from_secs(2);

/// The first stdout line of `serve`: what a demo or harness waits for.
pub fn listening(port: u16) -> String {
    format!("listening {port}")
}

/// Binds 127.0.0.1:`port` (0 picks one) and serves until the task is dropped.
pub async fn start(shared: Shared, port: u16) -> std::io::Result<(u16, JoinHandle<()>)> {
    let listener = TcpListener::bind(("127.0.0.1", port)).await?;
    let port = listener.local_addr()?.port();
    let app = Router::new()
        .route("/__conformance/cases/{id}", get(case_json))
        .route("/__conformance/arm/{id}", post(arm))
        .route("/__conformance/finish/{id}", post(finish))
        .fallback(scripted)
        .with_state(shared);
    let task = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    Ok((port, task))
}

async fn case_json(State(shared): State<Shared>, Path(id): Path<String>) -> Response {
    match shared.lock().cases.get(&id) {
        Some(case) => axum::Json(case.clone()).into_response(),
        None => (StatusCode::NOT_FOUND, format!("no case {id}")).into_response(),
    }
}

async fn arm(State(shared): State<Shared>, Path(id): Path<String>) -> Response {
    let mut state = shared.lock();
    let Some(case) = state.cases.get(&id).cloned() else {
        return (StatusCode::NOT_FOUND, format!("no case {id}")).into_response();
    };
    let deliveries = match (&case.deliver, state.receiver.clone()) {
        (Some(items), Some(receiver)) => Some(tokio::spawn(sign::deliver_all(case.id.clone(), items.clone(), receiver))),
        (Some(_), None) => Some(tokio::spawn(async move { vec![format!("{}: deliver: needs a receiver (run --tier es)", case.id)] })),
        (None, _) => None,
    };
    state.log.push(LogEntry::Arm(id.clone()));
    state.armed = Some(Armed { id, deliveries, ..Armed::default() });
    StatusCode::NO_CONTENT.into_response()
}

/// What a harness may send at finish: the player each delivery's handler
/// saw, in delivery order (`null` for a handler never called).
#[derive(Debug, Default, Deserialize)]
struct FinishBody {
    #[serde(default)]
    handlers: Vec<Option<String>>,
}

async fn finish(State(shared): State<Shared>, Path(id): Path<String>, body: Bytes) -> Response {
    let reported: FinishBody = serde_json::from_slice(&body).unwrap_or_default();
    let (armed, case) = {
        let mut state = shared.lock();
        state.log.push(LogEntry::Finish(id.clone()));
        let armed = state.armed.take().filter(|a| a.id == id);
        (armed, state.cases.get(&id).cloned())
    };
    let (Some(mut armed), Some(case)) = (armed, case) else {
        return (StatusCode::CONFLICT, format!("{id} is not the armed case")).into_response();
    };
    if let Some(task) = armed.deliveries.take() {
        armed.mismatches.extend(task.await.unwrap_or_else(|e| vec![format!("{id}: deliveries panicked: {e}")]));
    }
    let mut mismatches = armed.mismatches;
    mismatches.extend(streams_left_open(&shared, &id).await);
    mismatches.extend(sign::handler_mismatches(&case, &reported.handlers));
    let unused = case.serve.len().saturating_sub(armed.cursor);
    if unused > 0 {
        mismatches.push(format!("{id}: {unused} scripted answer(s) never requested"));
    }
    if let Some(want) = case.expect.requests.as_ref().map(|r| r.count).filter(|n| *n != armed.requests) {
        mismatches.push(format!("{id}: {} request(s), expected {want}", armed.requests));
    }
    let result = if mismatches.is_empty() { "pass" } else { "fail" };
    axum::Json(json!({ "case": id, "result": result, "mismatches": mismatches })).into_response()
}

/// A harness finishes a case after closing its kit, so every stream must be
/// gone within 2 s of the finish (K1 D12, EK9): a kit that leaves one open
/// holds a connection a game can never get back.
async fn streams_left_open(shared: &Shared, id: &str) -> Option<String> {
    let deadline = tokio::time::Instant::now() + STREAM_DROP_WITHIN;
    while shared.open_streams.load(Ordering::SeqCst) > 0 && tokio::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let open = shared.open_streams.load(Ordering::SeqCst);
    (open > 0).then(|| format!("{id}: {open} stream(s) still connected 2 s after finish"))
}

async fn scripted(State(shared): State<Shared>, method: Method, uri: Uri, headers: HeaderMap, body: Bytes) -> Response {
    let seen = Seen {
        method: method.to_string(),
        path: uri.path().to_string(),
        headers: headers.iter().map(|(n, v)| (n.as_str().to_string(), v.to_str().unwrap_or_default().to_string())).collect(),
        body: body.to_vec(),
    };
    match next_step(&shared, &seen) {
        Some(step) => respond(step.respond, &shared),
        None => (StatusCode::NOT_FOUND, "no scripted answer").into_response(),
    }
}

/// Judges `seen` against the armed case's next step and advances to it.
fn next_step(shared: &Shared, seen: &Seen) -> Option<ServeStep> {
    let mut guard = shared.lock();
    let state = &mut *guard;
    let line = format!("{} {}", seen.method, seen.path);
    let Some(armed) = state.armed.as_mut() else {
        state.stray.push(line);
        return None;
    };
    armed.requests += 1;
    let case = state.cases.get(&armed.id)?;
    let Some(step) = case.serve.get(armed.cursor).cloned() else {
        armed.mismatches.push(format!("{}: unscripted request {line}", case.id));
        return None;
    };
    armed.cursor += 1;
    armed.mismatches.extend(judge(case, &step, seen, armed.previous.as_ref(), &shared.frozen));
    armed.previous = Some(seen.clone());
    Some(step)
}

fn respond(respond: Respond, shared: &Shared) -> Response {
    let status = StatusCode::from_u16(respond.status.unwrap_or(200)).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let mut builder = Response::builder().status(status);
    for (name, value) in &respond.headers {
        builder = builder.header(name.as_str(), value.as_str());
    }
    let body = if let Some(frames) = respond.sse {
        builder = builder.header(header::CONTENT_TYPE, "text/event-stream").header(header::CACHE_CONTROL, "no-store");
        Body::from_stream(crate::sse::stream(frames, respond.fault.unwrap_or_default(), Arc::clone(&shared.open_streams)))
    } else if let Some(json) = respond.json {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
        Body::from(json.to_string())
    } else {
        Body::empty()
    };
    builder.body(body).unwrap_or_else(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())
}
