use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use crate::embed::Embed;
use crate::error::{Diagnostics, ErrorKind};
use crate::handoff::{BoxFuture, EmbedTokenSource, IssuedToken, stale_after};
use crate::ids::Ids;
use crate::queue::{self, Queue};
use crate::rfc3339;
use crate::secret::Secret;
use crate::tests::fake_lingara::{FakeLingara, Reply};
use crate::tests::{SECS, accepted, collect, config, context, refusal, started, token, wait_for};
use crate::token::{HostTokenSource, UpdateLatch};
use crate::token_fetch::Mode;
use crate::types::{Command, EmbedConfig, EmbedError, EmbedEvent, Header, LogLevel, TokenReason, TokenSourceConfig, UpdateAvailable};

fn is_error(kind: ErrorKind) -> impl Fn(&EmbedEvent) -> bool {
    move |e| matches!(e, EmbedEvent::Error { error } if error.kind == kind)
}

fn is_need(reason: TokenReason) -> impl Fn(&EmbedEvent) -> bool {
    move |e| matches!(e, EmbedEvent::TokenNeeded { reason: r, .. } if *r == reason)
}

fn bearer(n: u32) -> String {
    format!("Bearer {}", token(n))
}

// 1.10.26y AC1
#[test]
fn initial_token_is_asked_for_and_calls_wait() {
    let fake = FakeLingara::start(|_, _| Reply::Json(202, accepted("lgr_evt_1")));
    let embed = Embed::start(config(&fake.url)).unwrap();
    assert!(matches!(embed.poll_event(), Some(EmbedEvent::TokenNeeded { reason: TokenReason::Initial, .. })));
    let request = embed.send_context(context("market")).unwrap();
    std::thread::sleep(Duration::from_millis(300));
    assert!(fake.seen().is_empty(), "nothing is sent before the token");
    embed.provide_token(token(1), rfc3339::from_unix(now() + 900)).unwrap();
    let (sent, _) = wait_for(&embed, 5 * SECS, |e| matches!(e, EmbedEvent::WorldSent { .. }));
    assert!(matches!(sent, EmbedEvent::WorldSent { request: r, .. } if r == request));
    assert_eq!(fake.seen()[0].header("authorization"), Some(bearer(1).as_str()));
}

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
}

fn source() -> (Arc<HostTokenSource>, Queue) {
    let (queue, outbox) = queue::channel(16, tokio::runtime::Handle::current());
    let diag = Diagnostics { level: LogLevel::Warn, outbox: outbox.clone() };
    (HostTokenSource::new(Mode::Poll, diag, Arc::new(Ids::default()), 30 * SECS, UpdateLatch::new(None)), queue)
}

fn issued(expires_at: Option<String>, expires_in_secs: Option<u32>) -> IssuedToken {
    IssuedToken { token: Secret::new(token(1)), expires_at, expires_in_secs, update_available: None }
}

/// No need before `at` seconds, then exactly one, `Expiring`.
async fn asked_at(queue: &Queue, at: u64) {
    tokio::time::sleep(Duration::from_secs(at) - Duration::from_millis(500)).await;
    let early: Vec<_> = std::iter::from_fn(|| queue.poll()).filter(|e| matches!(e, EmbedEvent::TokenNeeded { .. })).collect();
    assert!(early.is_empty(), "asked before {at} s: {early:?}");
    tokio::time::sleep(SECS).await;
    assert!(matches!(queue.poll(), Some(EmbedEvent::TokenNeeded { reason: TokenReason::Expiring, .. })), "not asked at {at} s");
}

// 1.10.26y AC2
#[tokio::test(start_paused = true)]
async fn the_stale_point_and_the_clock_clamp() {
    assert_eq!(stale_after(900 * SECS), 840 * SECS);
    assert_eq!(stale_after(40 * SECS), 20 * SECS);
    for (secs, at) in [(900, 840), (40, 20)] {
        let (source, queue) = source();
        source.store(&issued(None, Some(secs))).unwrap();
        asked_at(&queue, at).await;
    }
    for skew in [-10, 7_200] {
        let (source, queue) = source();
        source.store(&issued(Some(rfc3339::from_unix(now() + skew)), None)).unwrap();
        tokio::task::yield_now().await;
        let log = std::iter::from_fn(|| queue.poll()).find_map(|e| match e {
            EmbedEvent::Log { level: LogLevel::Warn, message, .. } => Some(message),
            _ => None,
        });
        assert!(log.is_some_and(|m| m.contains("300 s")), "a skewed clock is clamped with a warning");
        // 300 s, so stale at 240 s.
        asked_at(&queue, 240).await;
    }
}

// 1.10.26y AC3
#[test]
fn a_rejected_token_is_asked_for_once_and_retried_once() {
    let fake = FakeLingara::start(|_, i| if i == 0 { Reply::Json(401, refusal("unauthorized")) } else { Reply::Json(202, accepted("lgr_evt_1")) });
    let embed = started(config(&fake.url));
    embed.send_context(context("market")).unwrap();
    wait_for(&embed, 5 * SECS, is_need(TokenReason::Rejected));
    embed.provide_token_for(token(2), 900).unwrap();
    wait_for(&embed, 5 * SECS, |e| matches!(e, EmbedEvent::WorldSent { .. }));
    let seen = fake.seen();
    assert_eq!((seen.len(), seen[0].header("authorization"), seen[1].header("authorization")), (2, Some(bearer(1).as_str()), Some(bearer(2).as_str())));

    let fake = FakeLingara::start(|_, _| Reply::Json(401, refusal("unauthorized")));
    let embed = started(config(&fake.url));
    embed.send_context(context("market")).unwrap();
    wait_for(&embed, 5 * SECS, is_need(TokenReason::Rejected));
    embed.provide_token_for(token(2), 900).unwrap();
    wait_for(&embed, 5 * SECS, is_error(ErrorKind::TokenRejected));
    assert_eq!(fake.seen().len(), 2, "one retry, no more");
}

struct Fixed(&'static str);

impl EmbedTokenSource for Fixed {
    fn fetch(&self, _reason: TokenReason) -> BoxFuture<'_, Result<IssuedToken, EmbedError>> {
        let token = self.0;
        Box::pin(async move { Ok(IssuedToken { token: Secret::new(token), expires_at: None, expires_in_secs: Some(900), update_available: None }) })
    }
}

fn http(fake: &FakeLingara, path: &str, headers: Vec<Header>) -> EmbedConfig {
    EmbedConfig { token_source: TokenSourceConfig::Http { url: format!("{}{path}", fake.url), headers }, ..config(&fake.url) }
}

// 1.10.26y AC4
#[test]
fn only_embed_tokens_and_unavailable_fails_waiters() {
    let embed = Embed::start(config("http://127.0.0.1:9")).unwrap();
    let refused = embed.provide_token("lgr_at_server_token", rfc3339::from_unix(now() + 900)).unwrap_err();
    assert_eq!((refused.kind, refused.retryable, embed.status().needs_token), (ErrorKind::InvalidInput, false, true));

    let fake = FakeLingara::start(|s, _| match s.path.as_str() {
        "/client-token" => Reply::Json(200, json!({ "token": "lgr_at_x", "expires_at": rfc3339::from_unix(now() + 900) })),
        "/broken" => Reply::Json(500, json!({})),
        _ => Reply::Json(202, accepted("lgr_evt_1")),
    });
    for config in [http(&fake, "/client-token", vec![]), http(&fake, "/broken", vec![])] {
        let embed = Embed::start(config).unwrap();
        embed.send_context(context("market")).unwrap();
        wait_for(&embed, 5 * SECS, |e| matches!(e, EmbedEvent::Error { error } if matches!(error.kind, ErrorKind::InvalidInput | ErrorKind::TokenUnavailable)));
    }
    let embed = Embed::start_with_source(config(&fake.url), Arc::new(Fixed("lgr_at_x"))).unwrap();
    embed.send_context(context("market")).unwrap();
    wait_for(&embed, 5 * SECS, is_error(ErrorKind::InvalidInput));
    assert_eq!(fake.count("POST", "/v1/events"), 0, "no refused token is ever sent");

    let embed = Embed::start(config(&fake.url)).unwrap();
    embed.send_context(context("a")).unwrap();
    embed.send_context(context("b")).unwrap();
    std::thread::sleep(Duration::from_millis(100));
    embed.token_unavailable("offline").unwrap();
    let (_, before) = wait_for(&embed, 5 * SECS, is_error(ErrorKind::TokenUnavailable));
    wait_for(&embed, 5 * SECS, is_error(ErrorKind::TokenUnavailable));
    assert!(!embed.status().needs_token && before.iter().all(|e| !matches!(e, EmbedEvent::WorldSent { .. })));
    embed.send_context(context("c")).unwrap();
    wait_for(&embed, 5 * SECS, is_need(TokenReason::Initial));

    let embed = Embed::start(EmbedConfig { token_wait_timeout_ms: 200, ..config(&fake.url) }).unwrap();
    embed.send_context(context("market")).unwrap();
    wait_for(&embed, 3 * SECS, is_error(ErrorKind::TokenUnavailable));
}

// 1.10.26y AC28
#[test]
fn one_need_one_event_and_the_sources_are_replaceable() {
    let fake = FakeLingara::start(|s, _| match s.path.as_str() {
        "/a" | "/b" => Reply::Json(200, json!({ "token": token(7), "expires_at": rfc3339::from_unix(now() + 2) })),
        _ => Reply::Json(202, accepted("lgr_evt_1")),
    });
    let embed = Embed::start(config(&fake.url)).unwrap();
    embed.send_context(context("a")).unwrap();
    embed.send_context(context("b")).unwrap();
    std::thread::sleep(Duration::from_millis(200));
    let needs = embed.drain(64).into_iter().filter(|e| matches!(e, EmbedEvent::TokenNeeded { .. })).count();
    assert_eq!((needs, embed.status().needs_token), (1, true));
    embed.provide_token_for(token(1), 2).unwrap();
    assert!(!embed.status().needs_token);
    wait_for(&embed, 3 * SECS, is_need(TokenReason::Expiring));

    let header = |v: &str| Header { name: "x-session".into(), value: Secret::new(v) };
    let embed = Embed::start(http(&fake, "/a", vec![header("s1")])).unwrap();
    embed.send_context(context("a")).unwrap();
    wait_for(&embed, 5 * SECS, |e| matches!(e, EmbedEvent::WorldSent { .. }));
    embed.set_token_endpoint(&format!("{}/b", fake.url), vec![header("s2")]).unwrap();
    let deadline = std::time::Instant::now() + 5 * SECS;
    while fake.count("POST", "/b") == 0 && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    let to_b: Vec<_> = fake.seen().into_iter().filter(|s| s.path == "/b").collect();
    assert_eq!(to_b.first().and_then(|s| s.header("x-session")), Some("s2"), "the stale point fetches from the new endpoint");
    assert_eq!(fake.seen().iter().find(|s| s.path == "/a").and_then(|s| s.header("x-session")), Some("s1"));
}

fn handoff(update_available: Option<Value>) -> Value {
    let mut body = json!({ "token": token(3), "expires_at": rfc3339::from_unix(now() + 900) });
    if let Some(update) = update_available {
        body["update_available"] = update;
    }
    body
}

fn declaring(config: EmbedConfig, version: &str) -> EmbedConfig {
    EmbedConfig { embed_version: Some(version.into()), ..config }
}

// 30.10.26p AC1
#[test]
fn the_declared_version_goes_to_the_token_endpoint_only() {
    for bad in ["v1.0", "1.0", "01.0.0", "123456.123456.1234567", "1.0.0-beta"] {
        let refused = Embed::start(declaring(config("http://127.0.0.1:9"), bad)).unwrap_err();
        assert_eq!(refused.kind, ErrorKind::Config, "{bad}");
    }
    assert_eq!("123456.123456.1234567".len(), 21);
    let fake = FakeLingara::start(|s, _| match s.path.as_str() {
        "/token" | "/other" => Reply::Json(200, handoff(None)),
        _ => Reply::Json(202, accepted("lgr_evt_1")),
    });
    let embed = Embed::start(declaring(http(&fake, "/token", vec![]), "1.1.0")).unwrap();
    embed.send_context(context("market")).unwrap();
    wait_for(&embed, 5 * SECS, |e| matches!(e, EmbedEvent::WorldSent { .. }));
    // A re-pointed endpoint carries it too: a 2 s token goes stale at 1 s and is fetched from `/other`.
    embed.set_token_endpoint(&format!("{}/other", fake.url), vec![]).unwrap();
    embed.provide_token_for(token(4), 2).unwrap();
    let deadline = std::time::Instant::now() + 5 * SECS;
    while fake.count("POST", "/other") == 0 && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    let header = crate::EMBED_VERSION_HEADER;
    for seen in fake.seen() {
        let want = matches!(seen.path.as_str(), "/token" | "/other").then_some("1.1.0");
        assert_eq!(seen.header(header), want, "{} {}", seen.method, seen.path);
    }
    assert_eq!(fake.count("POST", "/v1/events"), 1, "Lingara was called, without the header");

    let fake = FakeLingara::start(|s, _| match s.path.as_str() {
        "/token" => Reply::Json(200, handoff(None)),
        _ => Reply::Json(202, accepted("lgr_evt_1")),
    });
    let embed = Embed::start(http(&fake, "/token", vec![])).unwrap();
    embed.send_context(context("market")).unwrap();
    wait_for(&embed, 5 * SECS, |e| matches!(e, EmbedEvent::WorldSent { .. }));
    assert!(fake.seen().iter().all(|s| s.header(crate::EMBED_VERSION_HEADER).is_none()), "absent sends no header");
}

/// `update-notice.json`'s rows: (embed_version, update_available, line).
/// `None` only when there is no `conformance/` tree beside the crate at all —
/// the packaged crate `check-msrv-core` unpacks outside the repo, which ships
/// `src/**` alone. In the workspace a missing vector still fails.
fn notice_rows() -> Option<Vec<(Option<String>, Value, String)>> {
    let conformance = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../conformance");
    if !conformance.is_dir() {
        return None;
    }
    let path = conformance.join("vectors/update-notice.json");
    let vector: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("update-notice.json")).unwrap();
    assert_eq!(vector["target"], crate::token::update::TARGET);
    let rows = vector["rows"].as_array().unwrap().iter();
    Some(rows.map(|r| (r["embed_version"].as_str().map(str::to_owned), r["update_available"].clone(), r["line"].as_str().unwrap().to_owned())).collect())
}

/// The `Log` lines and `UpdateAvailable` events among `events`, in order.
fn told(events: &[EmbedEvent]) -> Vec<&EmbedEvent> {
    events.iter().filter(|e| matches!(e, EmbedEvent::UpdateAvailable { .. } | EmbedEvent::Log { level: LogLevel::Warn, .. })).collect()
}

fn poll_started(config: EmbedConfig) -> Embed {
    let embed = Embed::start(config).unwrap();
    assert!(matches!(embed.poll_event(), Some(EmbedEvent::TokenNeeded { .. })));
    embed
}

// 30.10.26p AC2
#[test]
fn an_update_is_said_once_per_handle() {
    let Some(rows) = notice_rows() else {
        eprintln!("skipped: no conformance/ tree beside the packaged crate");
        return;
    };
    assert_eq!(rows.len(), 18);
    for (version, update, line) in &rows {
        let notice = UpdateAvailable::from_value(update.clone()).expect("a well-formed row");
        assert_eq!(&crate::token::update::notice_line(&notice, version.as_deref()), line);
    }
    for (version, update, line) in rows.into_iter().take(4) {
        let notice = UpdateAvailable::from_value(update).unwrap();
        let config = EmbedConfig { embed_version: version, ..config("http://127.0.0.1:9") };
        for _handle in 0..2 {
            let embed = poll_started(config.clone());
            let expires = || rfc3339::from_unix(now() + 900);
            embed.provide_token_with_update(token(1), expires(), Some(notice.clone())).unwrap();
            embed.provide_token_with_update(token(2), expires(), Some(notice.clone())).unwrap();
            embed.provide_token(token(3), expires()).unwrap();
            let events = collect(&embed, Duration::from_millis(100));
            let want_log = EmbedEvent::Log { level: LogLevel::Warn, target: "lingara_embed::update".into(), message: line.clone() };
            let want_event = EmbedEvent::UpdateAvailable { latest: notice.latest.clone(), deprecated: notice.deprecated, message: notice.message.clone() };
            assert_eq!(told(&events), vec![&want_log, &want_event], "a new handle is told again, once");
        }
    }
}

// 30.10.26p AC3
#[test]
fn a_bad_notice_never_costs_the_token() {
    for bad in [json!(5), json!({ "latest": "1.2.0", "message": null })] {
        assert!(UpdateAvailable::parse(&bad).is_err(), "the Debug line's cause: {bad}");
        let body = handoff(Some(bad.clone()));
        let fake = FakeLingara::start(move |s, _| match s.path.as_str() {
            "/token" => Reply::Json(200, body.clone()),
            _ => Reply::Json(202, accepted("lgr_evt_1")),
        });
        let embed = Embed::start(http(&fake, "/token", vec![])).unwrap();
        embed.send_context(context("market")).unwrap();
        let (_, before) = wait_for(&embed, 5 * SECS, |e| matches!(e, EmbedEvent::WorldSent { .. }));
        assert!(told(&before).is_empty(), "a malformed notice says nothing");

        let command: Command = serde_json::from_value(json!({ "type": "provide_token", "token": token(1), "expires_at": rfc3339::from_unix(now() + 900), "update_available": bad })).unwrap();
        assert!(matches!(&command, Command::ProvideToken { update_available: None, .. }));
        let embed = poll_started(config("http://127.0.0.1:9"));
        embed.execute(command).unwrap();
        assert!(!embed.status().needs_token && told(&collect(&embed, Duration::from_millis(100))).is_empty());
    }
    let today = IssuedToken::from_handoff_json(handoff(None).to_string().as_bytes()).unwrap();
    assert!(today.update_available.is_none() && today.expires_at.is_some());
    let good = json!({ "latest": null, "deprecated": true, "message": null });
    let read = IssuedToken::from_handoff_json(handoff(Some(good)).to_string().as_bytes()).unwrap();
    assert_eq!(read.update_available, Some(UpdateAvailable { latest: None, deprecated: true, message: None }));
}
