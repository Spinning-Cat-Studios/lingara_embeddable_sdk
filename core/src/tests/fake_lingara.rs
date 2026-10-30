//! A scripted loopback server for the unit tests: it records every request
//! and answers each from the test's own closure, on a runtime of its own,
//! so a test drives the real library over real HTTP. SSE answers can hold
//! the connection open, stay silent, or drop it; `open()` counts the
//! streams still connected.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Router;
use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use axum::response::Response;
use serde_json::Value;
use tokio::sync::mpsc;

/// One request as the server saw it. Header names are lower-case.
#[derive(Debug, Clone)]
pub struct Seen {
    pub method: String,
    pub path: String,
    pub query: Option<String>,
    pub headers: BTreeMap<String, String>,
    pub body: Value,
}

impl Seen {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(String::as_str)
    }

    pub fn is(&self, method: &str, path: &str) -> bool {
        self.method == method && self.path == path
    }
}

/// What a stream does after its frames.
#[derive(Debug, Clone, Copy)]
pub enum After {
    /// End the body cleanly.
    End,
    /// Stay open, writing a heartbeat comment every 100 ms.
    Hold,
    /// Stay open and write nothing: a heartbeat gap.
    Silent,
    /// Abort the connection.
    Drop,
}

pub enum Reply {
    Json(u16, Value),
    /// A non-JSON body: a maintenance page.
    Text(u16, &'static str),
    JsonHeaders(u16, Value, Vec<(&'static str, String)>),
    /// No body, with headers: a `304`.
    Empty(u16, Vec<(&'static str, String)>),
    Sse(Vec<String>, After),
}

type Handler = dyn Fn(&Seen, usize) -> Reply + Send + Sync;

struct Inner {
    seen: Mutex<Vec<Seen>>,
    open: AtomicUsize,
    handler: Box<Handler>,
}

pub struct FakeLingara {
    pub url: String,
    inner: Arc<Inner>,
}

impl FakeLingara {
    /// Serves `handler`, which is given each request and its 0-based index.
    pub fn start(handler: impl Fn(&Seen, usize) -> Reply + Send + Sync + 'static) -> Self {
        let inner = Arc::new(Inner { seen: Mutex::new(Vec::new()), open: AtomicUsize::new(0), handler: Box::new(handler) });
        let (tx, rx) = std::sync::mpsc::channel();
        let state = Arc::clone(&inner);
        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().expect("runtime");
            runtime.block_on(async move {
                let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
                tx.send(listener.local_addr().expect("addr").port()).expect("port");
                let app = Router::new().fallback(answer).with_state(state);
                axum::serve(listener, app).await.expect("serve");
            });
        });
        let port = rx.recv().expect("the fake server started");
        Self { url: format!("http://127.0.0.1:{port}"), inner }
    }

    pub fn seen(&self) -> Vec<Seen> {
        self.inner.seen.lock().unwrap().clone()
    }

    pub fn count(&self, method: &str, path: &str) -> usize {
        self.seen().iter().filter(|s| s.is(method, path)).count()
    }

    /// Streams still connected.
    pub fn open(&self) -> usize {
        self.inner.open.load(Ordering::SeqCst)
    }
}

async fn answer(State(inner): State<Arc<Inner>>, method: Method, uri: Uri, headers: HeaderMap, body: Bytes) -> Response {
    let seen = Seen {
        method: method.to_string(),
        path: uri.path().to_owned(),
        query: uri.query().map(str::to_owned),
        headers: headers.iter().map(|(n, v)| (n.as_str().to_owned(), v.to_str().unwrap_or_default().to_owned())).collect(),
        body: serde_json::from_slice(&body).unwrap_or(Value::Null),
    };
    let index = {
        let mut all = inner.seen.lock().unwrap();
        all.push(seen.clone());
        all.len() - 1
    };
    match (inner.handler)(&seen, index) {
        Reply::Json(status, json) => json_response(status, &json, Vec::new()),
        Reply::Text(status, text) => Response::builder().status(status).header("content-type", "text/html").body(Body::from(text)).unwrap(),
        Reply::JsonHeaders(status, json, extra) => json_response(status, &json, extra),
        Reply::Empty(status, extra) => {
            let builder = extra.into_iter().fold(Response::builder().status(status), |b, (name, value)| b.header(name, value));
            builder.body(Body::empty()).unwrap()
        }
        Reply::Sse(frames, after) => sse_response(&inner, frames, after),
    }
}

fn json_response(status: u16, json: &Value, extra: Vec<(&'static str, String)>) -> Response {
    let mut builder = Response::builder().status(StatusCode::from_u16(status).unwrap()).header("content-type", "application/json");
    for (name, value) in extra {
        builder = builder.header(name, value);
    }
    builder.body(Body::from(json.to_string())).unwrap()
}

/// Decrements the open count when hyper drops the body: the client is gone.
struct Open(Arc<Inner>);

impl Drop for Open {
    fn drop(&mut self) {
        self.0.open.fetch_sub(1, Ordering::SeqCst);
    }
}

fn sse_response(inner: &Arc<Inner>, frames: Vec<String>, after: After) -> Response {
    inner.open.fetch_add(1, Ordering::SeqCst);
    let guard = Open(Arc::clone(inner));
    let (tx, rx) = mpsc::channel::<Result<Bytes, std::io::Error>>(64);
    tokio::spawn(async move {
        for frame in frames {
            if tx.send(Ok(Bytes::from(frame))).await.is_err() {
                return;
            }
        }
        match after {
            After::End => {}
            After::Drop => {
                tokio::time::sleep(Duration::from_millis(50)).await;
                let _ = tx.send(Err(std::io::Error::other("scripted drop"))).await;
            }
            After::Hold => {
                while tx.send(Ok(Bytes::from(": hb\n\n"))).await.is_ok() {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
            After::Silent => tx.closed().await,
        }
    });
    let stream = futures_stream(rx, guard);
    Response::builder().status(200).header("content-type", "text/event-stream").body(Body::from_stream(stream)).unwrap()
}

/// The receiver as a body stream that owns the open-count guard.
fn futures_stream(rx: mpsc::Receiver<Result<Bytes, std::io::Error>>, guard: Open) -> impl tokio_stream::Stream<Item = Result<Bytes, std::io::Error>> {
    use tokio_stream::StreamExt as _;
    tokio_stream::wrappers::ReceiverStream::new(rx).map(move |item| {
        let _ = &guard;
        item
    })
}

/// `event: event` with an id: one envelope on the events stream.
pub fn event_frame(id: &str, kind: &str, data: Value) -> String {
    let envelope = serde_json::json!({
        "id": id, "type": kind, "created_at": "2026-10-01T12:00:00Z",
        "api_version": "2026-10-golden-remora", "subject": "lgr_sub_test", "data": data,
    });
    format!("event: event\nid: {id}\ndata: {envelope}\n\n")
}

/// One named frame with no id: a plan or dialogue stream's.
pub fn frame(event: &str, data: Value) -> String {
    format!("event: {event}\ndata: {data}\n\n")
}

/// N2 §6's judged crossing, before `done` (ADR 28.10.26f D5).
pub fn transition(edge: &str, to: &str) -> String {
    frame("transition", serde_json::json!({ "edge": edge, "to": to }))
}

/// N3 D5's export (ADR 28.10.26i): the IR as its body, with its revision
/// and `ETag`.
pub fn exported(ir: &Value, rev: u64, etag: &str) -> Reply {
    Reply::JsonHeaders(200, ir.clone(), vec![("lingara-dialogue-revision", rev.to_string()), ("etag", etag.to_owned())])
}

/// N3 D5's `304` to a matching `If-None-Match`: both headers, no body.
pub fn not_modified(rev: u64, etag: &str) -> Reply {
    Reply::Empty(304, vec![("lingara-dialogue-revision", rev.to_string()), ("etag", etag.to_owned())])
}

/// N2 §3's answer to a checkpointed turn while `enable_npc_dialogue_graphs`
/// is dark for the integrator.
pub fn graphs_dark() -> Reply {
    Reply::Json(400, serde_json::json!({ "code": "invalid_request", "error": "`checkpoint` is not available to this client" }))
}
