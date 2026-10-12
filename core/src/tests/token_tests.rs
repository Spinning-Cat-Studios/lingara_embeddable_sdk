use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::json;

use crate::embed::Embed;
use crate::error::{Diagnostics, ErrorKind};
use crate::handoff::{BoxFuture, EmbedTokenSource, IssuedToken, stale_after};
use crate::ids::Ids;
use crate::queue::{self, Queue};
use crate::rfc3339;
use crate::secret::Secret;
use crate::tests::fake_lingara::{FakeLingara, Reply};
use crate::tests::{SECS, accepted, config, context, refusal, started, token, wait_for};
use crate::token::HostTokenSource;
use crate::token_fetch::Mode;
use crate::types::{EmbedConfig, EmbedError, EmbedEvent, Header, LogLevel, TokenReason, TokenSourceConfig};

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
    (HostTokenSource::new(Mode::Poll, outbox, diag, Arc::new(Ids::default()), 30 * SECS), queue)
}

fn issued(expires_at: Option<String>, expires_in_secs: Option<u32>) -> IssuedToken {
    IssuedToken { token: Secret::new(token(1)), expires_at, expires_in_secs }
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
        Box::pin(async move { Ok(IssuedToken { token: Secret::new(token), expires_at: None, expires_in_secs: Some(900) }) })
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
