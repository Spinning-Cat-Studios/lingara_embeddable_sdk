use std::time::{Duration, Instant};

use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::case::load_dirs;
use crate::serve::{listening, start};
use crate::state::Shared;
use crate::test_support::{frozen, tree};

const ET: &str = "Bearer lgr_et_conformance0001";

/// One raw HTTP/1.1 exchange: the status, the decoded body, and whether a
/// chunked body ended with its terminator (false when the server aborted it).
async fn http(port: u16, method: &str, path: &str, body: &str) -> (u16, String, bool) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.expect("connects");
    let request = format!(
        "{method} {path} HTTP/1.1\r\nhost: x\r\nauthorization: {ET}\r\nconnection: close\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).await.expect("writes");
    let mut bytes = Vec::new();
    let mut buf = [0u8; 4096];
    while let Ok(n) = stream.read(&mut buf).await {
        if n == 0 {
            break;
        }
        bytes.extend_from_slice(&buf[..n]);
    }
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let status = text.split(' ').nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let (head, rest) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
    if !head.to_ascii_lowercase().contains("transfer-encoding: chunked") {
        return (status, rest.to_string(), true);
    }
    let (decoded, clean) = dechunk(rest);
    (status, decoded, clean)
}

/// Chunked transfer coding, decoded; false when the terminator never came.
fn dechunk(mut rest: &str) -> (String, bool) {
    let mut out = String::new();
    while let Some((size, after)) = rest.split_once("\r\n") {
        let Ok(n) = usize::from_str_radix(size.trim(), 16) else { break };
        if n == 0 {
            return (out, true);
        }
        let Some(chunk) = after.get(..n) else { break };
        out.push_str(chunk);
        rest = after.get(n + 2..).unwrap_or_default();
    }
    (out, false)
}

async fn control(port: u16, verb: &str, id: &str, body: &str) -> Value {
    let (_, body, _) = http(port, "POST", &format!("/__conformance/{verb}/{id}"), body).await;
    serde_json::from_str(body.trim()).unwrap_or(Value::Null)
}

async fn server(files: &[(&str, &str)]) -> (tempfile::TempDir, u16) {
    let dir = tree(files);
    let (loaded, problems) = load_dirs(&[dir.path().to_path_buf()]);
    assert_eq!(problems, Vec::<String>::new());
    let shared = Shared::new(loaded.into_iter().map(|l| l.case).collect(), frozen());
    let (port, _task) = start(shared, 0).await.expect("binds");
    (dir, port)
}

const FAULTS: &str = r#"
id: ek3.faults
behaviours: [EK3]
tier: ek
serve:
  - route: { operation: streamEvents }
    respond:
      sse: [ { id: "e1", event: "lesson_plan.ready", data: { n: 1 } }, { comment: "heartbeat" }, { id: "e2", event: "lesson_plan.ready", data: { n: 2 } } ]
      fault: { drop_after_events: 1 }
  - route: { operation: streamEvents }
    respond:
      sse: [ { comment: "heartbeat" }, { id: "e3", event: "lesson_plan.ready", data: { n: 3 } } ]
      fault: { heartbeat_gap_s: 0.3, delay_fragment_ms: 20 }
  - route: { operation: sendDialogueTurn }
    respond:
      sse: [ { event: "delta", data: { text: "你好" } }, { event: "notice", data: { code: "c", message: "m" } }, { event: "done", data: {} }, { event: "error", data: { code: "e", message: "m" } } ]
  - route: { operation: streamEvents }
    respond: { status: 401, json: { error: "unauthorized" } }
"#;

// 1.10.26x AC6
#[tokio::test]
async fn scripted_faults_happen_as_written() {
    let (_dir, port) = server(&[("ek3/faults.yaml", FAULTS)]).await;
    assert_eq!(control(port, "arm", "ek3.faults", "").await, Value::Null, "an arm answers 204");

    // A drop after one event: e1 arrives, e2 never does, and the body is cut.
    let (status, text, clean) = http(port, "GET", "/v1/events/stream", "").await;
    assert_eq!(status, 200);
    assert!(text.contains("id: e1\n") && text.contains("event: lesson_plan.ready\n") && !text.contains("e2"), "{text}");
    assert!(!clean, "a scripted drop aborts the body: {text:?}");

    // A heartbeat comment, an id: line, then a silent gap before the close.
    let started = Instant::now();
    let (_, text, clean) = http(port, "GET", "/v1/events/stream", "").await;
    assert!(started.elapsed() >= Duration::from_millis(300), "the gap held the stream open");
    assert!(text.contains(": heartbeat\n\n") && text.contains("id: e3\n") && clean, "{text}");

    // A dialogue turn: delta / notice / done / error, and no id: line at all.
    let (_, text, _) = http(port, "POST", "/v1/embed/dialogue/turns", "{}").await;
    for event in ["delta", "notice", "done", "error"] {
        assert!(text.contains(&format!("event: {event}\n")), "no {event} frame: {text}");
    }
    assert!(!text.contains("id:") && text.contains("你好"), "{text}");

    // A mid-session 401.
    let (status, _, _) = http(port, "GET", "/v1/events/stream", "").await;
    assert_eq!(status, 401);

    let verdict = control(port, "finish", "ek3.faults", "").await;
    assert_eq!(verdict["result"], "pass", "{verdict}");
}

const DEMO: &str = r#"
id: demo.mint-a-plan
behaviours: []
tier: ek
serve:
  - route: { operation: sendEvent }
    respond: { status: 202 }
"#;

// 1.10.26x AC21
#[tokio::test]
async fn serve_answers_a_verdict_at_finish() {
    // A case folder outside conformance/, served on a picked port.
    let (_dir, port) = server(&[("demo/mint-a-plan.yaml", DEMO)]).await;
    assert_ne!(port, 0);
    assert_eq!(listening(port), format!("listening {port}"));

    control(port, "arm", "demo.mint-a-plan", "").await;
    let (status, _, _) = http(port, "POST", "/v1/events", r#"{"type":"world.practice_completed"}"#).await;
    assert_eq!(status, 202);
    let verdict = control(port, "finish", "demo.mint-a-plan", "").await;
    assert_eq!((verdict["result"].as_str(), verdict["mismatches"].as_array().map(Vec::len)), (Some("pass"), Some(0)), "{verdict}");

    // A request that does not match: the verdict names the mismatch.
    control(port, "arm", "demo.mint-a-plan", "").await;
    http(port, "POST", "/v1/events", r#"{"data":{}}"#).await;
    let verdict = control(port, "finish", "demo.mint-a-plan", "").await;
    assert_eq!(verdict["result"], "fail");
    let first = verdict["mismatches"][0].as_str().unwrap_or_default();
    assert!(first.starts_with("demo.mint-a-plan: sendEvent: body"), "{verdict}");
}
