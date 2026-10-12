use std::sync::{Arc, Mutex, Once};

use serde_json::json;

use crate::embed::Embed;
use crate::error::ErrorKind;
use crate::secret::Secret;
use crate::tests::fake_lingara::{After, FakeLingara, Reply, frame};
use crate::tests::{SECS, accepted, config, context, refusal, started, wait_for};
use crate::types::{Command, EmbedConfig, EmbedError, EmbedEvent, Header, PracticePhase, PracticeSource, TokenSourceConfig};

/// Every log line any test emits, through the `log` facade.
static LINES: Mutex<Vec<String>> = Mutex::new(Vec::new());

struct Capture;

impl log::Log for Capture {
    fn enabled(&self, _: &log::Metadata<'_>) -> bool {
        true
    }

    fn log(&self, record: &log::Record<'_>) {
        LINES.lock().unwrap().push(format!("{} {}", record.target(), record.args()));
    }

    fn flush(&self) {}
}

fn capture_logs() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = log::set_logger(&Capture);
        log::set_max_level(log::LevelFilter::Trace);
    });
}

const TOKEN: &str = "lgr_et_SECRETSECRETSECRETSECRETSECRETSECRETSECRETS";
const SESSION: &str = "session-cookie-SECRET";

fn leaks(text: &str) -> bool {
    text.contains("SECRET")
}

// 1.10.26y AC8
#[test]
fn secrets_never_render() {
    capture_logs();
    let header = Header { name: "cookie".into(), value: Secret::new(SESSION) };
    let commands = [
        Command::ProvideToken { token: Secret::new(TOKEN), expires_at: "2026-10-01T12:15:00Z".into() },
        Command::ProvideTokenFor { token: Secret::new(TOKEN), expires_in_secs: 900 },
        Command::SetTokenEndpoint { url: "https://g.example/t".into(), headers: vec![header.clone()] },
    ];
    for c in &commands {
        let json = serde_json::to_string(c).unwrap();
        assert!(!leaks(&format!("{c:?}")) && !leaks(&json) && json.contains(crate::secret::REDACTED), "{json}");
    }
    let fake = FakeLingara::start(|s, _| match s.path.as_str() {
        "/token" => Reply::Json(200, json!({ "token": TOKEN, "expires_at": crate::rfc3339::from_unix(4_000_000_000) })),
        "/v1/events/stream" => Reply::Sse(vec![], After::Hold),
        _ => Reply::Json(202, accepted("lgr_evt_1")),
    });
    let http = TokenSourceConfig::Http { url: format!("{}/token", fake.url), headers: vec![header] };
    let config = EmbedConfig { token_source: http, stream_on_start: true, log_level: crate::types::LogLevel::Trace, ..config(&fake.url) };
    assert!(!leaks(&format!("{config:?}")) && !leaks(&serde_json::to_string(&config).unwrap()));
    let embed = Embed::start(config).unwrap();
    embed.send_context(context("market")).unwrap();
    let (_, mut seen) = wait_for(&embed, 5 * SECS, |e| matches!(e, EmbedEvent::WorldSent { .. }));
    seen.extend(crate::tests::collect(&embed, std::time::Duration::from_millis(200)));
    embed.provide_token("lgr_at_SECRET", "x").unwrap_err();
    assert_eq!(fake.seen()[0].header("cookie"), Some(SESSION), "the header is sent, only never rendered");
    for event in &seen {
        assert!(!leaks(&format!("{event:?}")) && !leaks(&serde_json::to_string(event).unwrap()), "{event:?}");
    }
    assert!(!leaks(&format!("{embed:?}")) && !leaks(&format!("{:?}", embed.status())));
    assert!(LINES.lock().unwrap().iter().all(|l| !leaks(l)), "no log line carries a token or a header value");
}

/// The failure a `GET /v1/lesson-plans/{id}` answered `reply` reports.
fn failure(reply: impl Fn() -> Reply + Send + Sync + 'static, source: PracticeSource) -> EmbedError {
    let fake = FakeLingara::start(move |_, _| reply());
    let embed = started(config(&fake.url));
    let session = embed.practice_start(source).unwrap();
    let (failed, _) = wait_for(&embed, 10 * SECS, |e| matches!(e, EmbedEvent::Practice { snapshot, .. } if snapshot.phase == PracticePhase::Failed));
    let _ = session;
    match failed {
        EmbedEvent::Practice { snapshot, .. } => snapshot.failure.expect("a failure"),
        _ => unreachable!(),
    }
}

fn read() -> PracticeSource {
    PracticeSource::Plan { plan_id: "8f0e2c1a-0000-4000-8000-000000000001".into(), report: true }
}

// 1.10.26y AC18
#[test]
fn library_errors_map_to_their_kinds() {
    let cases: [(u16, &str, ErrorKind, bool); 8] = [
        (400, "invalid_request", ErrorKind::InvalidInput, false),
        (402, "spend_cap_reached", ErrorKind::Billing, false),
        (403, "embed_needs_metered", ErrorKind::Forbidden, false),
        (403, "insufficient_scope", ErrorKind::Forbidden, false),
        (404, "not_found", ErrorKind::NotFound, false),
        (410, "api_version_discontinued", ErrorKind::Config, false),
        (422, "safety_input_flagged", ErrorKind::InvalidInput, false),
        (500, "internal", ErrorKind::Unavailable, true),
    ];
    for (status, code, kind, retryable) in cases {
        let err = failure(move || Reply::Json(status, refusal(code)), read());
        assert_eq!((err.kind, err.retryable, err.status, err.code.as_deref()), (kind, retryable, Some(status), Some(code)));
    }
    // Above the library's 60 s cap, a Retry-After is raised, not slept.
    let err = failure(|| Reply::JsonHeaders(429, refusal("rate_limited"), vec![("retry-after", "120".into())]), read());
    assert_eq!((err.kind, err.retryable, err.retry_after_ms), (ErrorKind::RateLimited, true, Some(120_000)));
    let err = failure(|| Reply::Text(503, "<h1>maintenance</h1>"), read());
    assert_eq!((err.kind, err.retryable), (ErrorKind::Unavailable, true));
    let create = PracticeSource::Create { context: "tea".into(), source_lang: "en".into(), target_lang: "zh".into(), level: 2 };
    let in_stream = || Reply::Sse(vec![frame("error", json!({ "code": "stream_interrupted", "message": "provider" }))], After::End);
    let err = failure(in_stream, create);
    assert_eq!((err.kind, err.retryable, err.status, err.code.as_deref()), (ErrorKind::Unavailable, true, Some(200), Some("stream_interrupted")));
    let err = failure(|| Reply::Sse(vec![], After::Drop), read());
    assert_eq!((err.kind, err.retryable), (ErrorKind::Network, true));
    let _ = Arc::new(());
}
