//! A dialogue loaded by its store id (ADR 28.10.26i): the fetch over the
//! real library and `fake_lingara`, and the cache, the `loading` rules and
//! the one-fetch-per-id map under paused time over `fake_api`.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use serde_json::{Value, json};

use crate::api::Exported;
use crate::embed::Embed;
use crate::error::{EmbedError, ErrorKind};
use crate::limits;
use crate::tests::fake_api::FakeApi;
use crate::tests::fake_lingara::{FakeLingara, Reply, exported, not_modified};
use crate::tests::{SECS, collect, config, paused, refusal, started, wait_for};
use crate::types::{CrossedVia, DialogueId, DialoguePhase, DialogueSource, EmbedEvent, FactValue, GraphSession, NpcTurnUpdate};

const ID: &str = "3f2a9c1e-7b4d-4e8a-9c0f-1a2b3c4d5e6f";
const ETAG: &str = "\"sha256-one\"";

/// `trust` opens `warm`, an edge with no `when`, so a held fact shows as an auto crossing.
fn graph(tag: &str) -> Value {
    json!({
        "schema": "dialogue.v1", "id": tag,
        "npcs": [{ "id": "elena", "name": "Elena" }],
        "facts": [{ "key": "trust", "type": "number", "default": 0 }],
        "start": "intro",
        "checkpoints": [
            { "id": "intro", "goal": "Elena is wary", "edges": [
                { "id": "warm", "to": "ask", "predicate": "trust >= 0.4" },
                { "id": "hello", "to": "ask", "when": "the player greets her" }] },
            { "id": "ask", "goal": "Admit she saw soldiers", "edges": [{ "id": "bye", "to": "success", "when": "the player says goodbye" }] }
        ]
    })
}

fn session(npc: &str) -> GraphSession {
    GraphSession { npc: Some(npc.into()), source_lang: "en".into(), target_lang: "es".into(), level: 2, scene: None, facts: BTreeMap::new(), checkpoint: None, window: vec![] }
}

fn stored(id: &str) -> DialogueSource {
    DialogueSource::Stored { dialogue_id: id.into() }
}

fn id(n: usize) -> String {
    format!("00000000-0000-4000-8000-{n:012}")
}

fn body(tag: &str, rev: u64, etag: &str) -> Result<Exported, EmbedError> {
    Ok(Exported::Body { rev, etag: etag.into(), ir_json: graph(tag).to_string() })
}

fn loaded(d: DialogueId) -> impl Fn(&EmbedEvent) -> bool {
    move |e| matches!(e, EmbedEvent::DialogueLoaded { dialogue, .. } if *dialogue == d)
}

fn failed(e: &EmbedEvent, d: DialogueId) -> Option<EmbedError> {
    match e {
        EmbedEvent::DialogueLoadFailed { dialogue, error } if *dialogue == d => Some(error.clone()),
        _ => None,
    }
}

fn wait_failed(embed: &Embed, d: DialogueId) -> EmbedError {
    let (event, _) = wait_for(embed, 5 * SECS, |e| failed(e, d).is_some());
    failed(&event, d).unwrap()
}

/// Jumps the I/O thread's paused clock by `by`.
fn advance(embed: &Embed, by: Duration) {
    let (tx, rx) = std::sync::mpsc::channel();
    embed.shared.io.handle().spawn(async move {
        tokio::time::sleep(by).await;
        let _ = tx.send(());
    });
    rx.recv_timeout(5 * SECS).expect("the paused clock advanced");
}

fn exports(api: &FakeApi) -> usize {
    api.at("export_dialogue").len()
}

/// Answers the held fetch once it is the `nth` export: every load spawned
/// before it has then been planned, so none can miss the fetch in flight.
fn release(api: &FakeApi, nth: usize, answer: Result<Exported, EmbedError>) {
    let deadline = std::time::Instant::now() + 5 * SECS;
    while exports(api) < nth && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(exports(api), nth, "the held fetch was sent");
    api.exports.lock().unwrap().push_back(answer);
    api.release_exports.notify_one();
}

// 28.10.26i AC1
#[test]
fn the_fetch_carries_the_token_reads_the_revision_and_refuses_a_bad_id() {
    let fake = FakeLingara::start(|_, i| if i == 0 { exported(&graph("v4"), 4, ETAG) } else { not_modified(4, ETAG) });
    let embed = started(config(&fake.url));
    for bad in ["not-a-uuid", "3f2a9c1e7b4d4e8a9c0f1a2b3c4d5e6f0000", "3f2a9c1e-7b4d-4e8a-9c0f-1a2b3c4d5e6g"] {
        let refused = embed.dialogue_load(stored(bad), session("elena")).unwrap_err();
        assert_eq!(refused.kind, ErrorKind::InvalidInput, "{bad}");
    }
    assert!(fake.seen().is_empty(), "a malformed id sends nothing");

    let d = embed.dialogue_load(stored(&ID.to_uppercase()), session("elena")).unwrap();
    let (event, _) = wait_for(&embed, 5 * SECS, loaded(d));
    assert_eq!(event, EmbedEvent::DialogueLoaded { dialogue: d, dialogue_id: ID.into(), rev: 4 }, "the header's revision, under the lowercased id");
    let seen = &fake.seen()[0];
    assert!(seen.is("GET", &format!("/v1/dialogues/{ID}/export")), "{} {}", seen.method, seen.path);
    assert_eq!(seen.query, None, "N3's defaults: no revision, no layout");
    assert_eq!(seen.header("authorization"), Some(format!("Bearer {}", crate::tests::token(1)).as_str()));
    assert_eq!(seen.header("if-none-match"), None, "nothing cached yet");

    // The adapter's revalidation, straight through the port: `If-None-Match` out, a `304` in.
    let (api, (tx, rx)) = (Arc::clone(&embed.shared.api), std::sync::mpsc::channel());
    embed.shared.io.handle().spawn(async move {
        let _ = tx.send(api.export_dialogue(ID.into(), Some(ETAG.into())).await);
    });
    assert_eq!(rx.recv_timeout(5 * SECS).unwrap(), Ok(Exported::NotModified { rev: 4, etag: ETAG.into() }));
    assert_eq!(fake.seen()[1].header("if-none-match"), Some(ETAG));
}

// 28.10.26i AC2
#[test]
fn a_stored_load_starts_like_an_inline_one() {
    let api = Arc::new(FakeApi::default());
    api.hold_exports.store(true, Ordering::SeqCst);
    let embed = paused(config("http://127.0.0.1:9"), &api);
    let d = embed.dialogue_load(stored(ID), session("elena")).unwrap();
    let snapshot = embed.dialogue_snapshot(d).unwrap();
    assert_eq!((snapshot.phase, snapshot.graph), (DialoguePhase::Loading, None));
    assert_eq!(embed.dialogue_say(d, "hola").unwrap_err().kind, ErrorKind::InvalidState);
    assert_eq!(embed.dialogue_cancel(d).unwrap_err().kind, ErrorKind::InvalidState);
    embed.dialogue_set_level(d, 3).unwrap();
    embed.dialogue_set_scene(d, Some("the night market".into())).unwrap();
    embed.dialogue_set_fact(d, "trust", FactValue::Number(serde_json::Number::from_f64(0.5).unwrap())).unwrap();
    let undeclared = embed.dialogue_load(stored(ID), session("elena")).unwrap();
    embed.dialogue_set_fact(undeclared, "nope", FactValue::Boolean(true)).unwrap();
    let closed = embed.dialogue_load(stored(ID), session("elena")).unwrap();
    embed.dialogue_close(closed).unwrap();

    release(&api, 1, body("v4", 4, ETAG));
    let (event, mut events) = wait_for(&embed, 5 * SECS, |e| failed(e, undeclared).is_some());
    assert_eq!(failed(&event, undeclared).unwrap().kind, ErrorKind::InvalidInput, "a held fact K1 refuses fails the load");
    events.extend(collect(&embed, SECS / 2));
    events.retain(|e| !matches!(e, EmbedEvent::Log { .. } | EmbedEvent::TokenNeeded { .. }));
    let ours: Vec<_> = events.iter().filter(|e| matches!(e, EmbedEvent::DialogueLoaded { dialogue, .. } | EmbedEvent::CheckpointReached { dialogue, .. } if *dialogue == d)).collect();
    assert!(matches!(ours[0], EmbedEvent::DialogueLoaded { rev: 4, .. }), "{ours:?}");
    assert!(matches!(ours[1], EmbedEvent::CheckpointReached { checkpoint, via: CrossedVia::Start, .. } if checkpoint == "intro"));
    assert!(matches!(ours[2], EmbedEvent::CheckpointReached { checkpoint, via: CrossedVia::Auto, .. } if checkpoint == "ask"), "the held fact is in force before the first auto-crossing");
    assert!(!events.iter().any(|e| format!("{e:?}").contains(&format!("dialogue: {closed}"))), "a close while loading emits nothing");
    let snapshot = embed.dialogue_snapshot(d).unwrap();
    assert_eq!((snapshot.phase, snapshot.level, snapshot.scene.as_deref()), (DialoguePhase::Idle, 3, Some("the night market")));
    assert!(embed.dialogue_snapshot(undeclared).is_none() && embed.dialogue_snapshot(closed).is_none());
    assert_eq!(exports(&api), 1, "three loads in flight, one request");
}

// 28.10.26i AC3
#[test]
fn one_fetch_per_id_per_five_minutes() {
    let api = Arc::new(FakeApi::default());
    let embed = paused(config("http://127.0.0.1:9"), &api);
    let load = |id: &str| embed.dialogue_load(stored(id), session("elena")).unwrap();
    api.exports.lock().unwrap().push_back(body("v4", 4, ETAG));
    wait_for(&embed, 5 * SECS, loaded(load(ID)));
    let first = embed.shared.dialogue_cache.peek(ID).unwrap();
    wait_for(&embed, 5 * SECS, loaded(load(ID)));
    assert_eq!(exports(&api), 1, "a second load within 300 s sends nothing");

    advance(&embed, Duration::from_millis(limits::DIALOGUE_ACTIVE_TTL_MS + 1_000));
    api.exports.lock().unwrap().push_back(Ok(Exported::NotModified { rev: 4, etag: ETAG.into() }));
    wait_for(&embed, 5 * SECS, loaded(load(ID)));
    assert_eq!(api.export_requests.lock().unwrap()[1], (ID.to_owned(), Some(ETAG.to_owned())), "revalidated with If-None-Match");
    assert!(Arc::ptr_eq(&first, &embed.shared.dialogue_cache.peek(ID).unwrap()), "a 304 keeps the same Arc");
    advance(&embed, Duration::from_millis(limits::DIALOGUE_ACTIVE_TTL_MS + 1_000));
    api.exports.lock().unwrap().push_back(body("v4", 4, ETAG));
    wait_for(&embed, 5 * SECS, loaded(load(ID)));
    assert!(Arc::ptr_eq(&first, &embed.shared.dialogue_cache.peek(ID).unwrap()), "an unchanged 200 is not reparsed");

    api.hold_exports.store(true, Ordering::SeqCst);
    let (a, b) = (load(&id(1)), load(&id(1)));
    release(&api, 4, body("v1", 1, ETAG));
    wait_for(&embed, 5 * SECS, loaded(a));
    wait_for(&embed, 5 * SECS, loaded(b));
    let (c, d) = (load(&id(2)), load(&id(2)));
    release(&api, 5, Err(EmbedError::new(ErrorKind::NotFound, "no such dialogue")));
    assert_eq!((wait_failed(&embed, c).kind, wait_failed(&embed, d).kind), (ErrorKind::NotFound, ErrorKind::NotFound), "each waiter fails on its own");
    let gone = load(&id(3));
    embed.dialogue_close(gone).unwrap();
    release(&api, 6, body("v3", 3, ETAG));
    let deadline = std::time::Instant::now() + 5 * SECS;
    while embed.shared.dialogue_cache.peek(&id(3)).is_none() && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(embed.shared.dialogue_cache.peek(&id(3)).is_some(), "a fetch whose dialogue closed still fills the cache");
    assert_eq!(exports(&api), 6, "{:?}", api.export_requests.lock().unwrap());
}

// 28.10.26i AC4
#[test]
fn eviction_and_new_revisions_never_touch_a_running_story() {
    let api = Arc::new(FakeApi::default());
    let embed = paused(config("http://127.0.0.1:9"), &api);
    let load = |n: usize| embed.dialogue_load(stored(&id(n)), session("elena")).unwrap();
    api.exports.lock().unwrap().extend((0..=limits::DIALOGUE_CACHE_MAX_ENTRIES).map(|n| body(&format!("v{n}"), 4, ETAG)));
    let running = load(0);
    wait_for(&embed, 5 * SECS, loaded(running));
    for n in 1..=limits::DIALOGUE_CACHE_MAX_ENTRIES {
        wait_for(&embed, 5 * SECS, loaded(load(n)));
    }
    assert!(embed.shared.dialogue_cache.peek(&id(0)).is_none(), "33 ids: the least recently used is gone");
    assert!(embed.shared.dialogue_cache.peek(&id(1)).is_some());
    let request = embed.dialogue_say(running, "hola").unwrap();
    wait_for(&embed, 5 * SECS, |e| matches!(e, EmbedEvent::NpcTurn { request: r, update: NpcTurnUpdate::Finished { .. }, .. } if *r == request));

    let older = load(1);
    wait_for(&embed, 5 * SECS, loaded(older));
    advance(&embed, Duration::from_millis(limits::DIALOGUE_ACTIVE_TTL_MS + 1_000));
    api.exports.lock().unwrap().push_back(body("v1_new", 5, "\"sha256-two\""));
    let newer = load(1);
    let (event, _) = wait_for(&embed, 5 * SECS, loaded(newer));
    assert!(matches!(event, EmbedEvent::DialogueLoaded { rev: 5, .. }));
    assert_eq!(embed.dialogue_snapshot(older).unwrap().graph.unwrap().id, "v1", "a running story keeps its revision");
    assert_eq!(embed.dialogue_snapshot(newer).unwrap().graph.unwrap().id, "v1_new");
}

fn fails_as(fake_reply: impl Fn() -> Reply + Send + Sync + 'static) -> (EmbedError, FakeLingara, Embed) {
    let fake = FakeLingara::start(move |_, _| fake_reply());
    let embed = started(config(&fake.url));
    let d = embed.dialogue_load(stored(ID), session("elena")).unwrap();
    let error = wait_failed(&embed, d);
    assert!(embed.dialogue_snapshot(d).is_none(), "a failed load removes the dialogue");
    (error, fake, embed)
}

// 28.10.26i AC5
#[test]
fn each_load_failure_has_its_kind_and_cache_rule() {
    let rows: [(u16, &str, ErrorKind); 5] = [
        (404, "not_found", ErrorKind::NotFound),
        (404, "dialogue_not_active", ErrorKind::NotFound),
        (403, "insufficient_scope", ErrorKind::Forbidden),
        (403, "embed_origin_mismatch", ErrorKind::Forbidden),
        (403, "embed_origin_required", ErrorKind::Forbidden),
    ];
    for (status, code, kind) in rows {
        let (error, fake, _embed) = fails_as(move || Reply::Json(status, refusal(code)));
        assert_eq!((error.kind, error.code.as_deref(), error.retryable), (kind, Some(code), false), "{status} {code}");
        assert_eq!(fake.seen().len(), 1, "never retried: {code}");
    }
    // A `Retry-After` past the library's cap is raised, never slept.
    let (error, _, _) = fails_as(|| Reply::JsonHeaders(429, refusal("rate_limited"), vec![("retry-after", "3600".into())]));
    assert_eq!((error.kind, error.retry_after_ms, error.retryable), (ErrorKind::RateLimited, Some(3_600_000), true));
    let (error, _, _) = fails_as(|| Reply::Json(500, refusal("internal")));
    assert_eq!((error.kind, error.retryable), (ErrorKind::Unavailable, true));
    let (error, _, _) = fails_as(|| Reply::JsonHeaders(200, graph("v4"), vec![("etag", ETAG.into())]));
    assert_eq!((error.kind, error.retryable), (ErrorKind::Network, true), "a 200 without its revision is MalformedResponse");

    let mut broken = graph("v4");
    broken["checkpoints"][0]["edges"][1]["to"] = json!("nowhere");
    let (error, fake, embed) = fails_as(move || exported(&broken, 7, ETAG));
    assert_eq!(error.kind, ErrorKind::InvalidInput);
    assert!(error.message.contains(ID) && error.message.contains("revision 7") && error.message.contains("nowhere"), "{}", error.message);
    let again = embed.dialogue_load(stored(ID), session("elena")).unwrap();
    wait_failed(&embed, again);
    assert_eq!(fake.seen().len(), 2, "a refused body caches nothing");

    let fake = FakeLingara::start(|_, _| exported(&graph("v4"), 4, ETAG));
    let embed = started(config(&fake.url));
    let d = embed.dialogue_load(stored(ID), session("tomas")).unwrap();
    let error = wait_failed(&embed, d);
    assert!(error.kind == ErrorKind::InvalidInput && error.message.contains("tomas"), "{error:?}");
    wait_for(&embed, 5 * SECS, loaded(embed.dialogue_load(stored(ID), session("elena")).unwrap()));
    assert_eq!(fake.seen().len(), 1, "an unknown npc leaves the valid IR cached");

    let embed = started(config("http://127.0.0.1:9"));
    let d = embed.dialogue_load(stored(ID), session("elena")).unwrap();
    assert_eq!(wait_failed(&embed, d).kind, ErrorKind::Network, "no response");
}

// 28.10.26i AC6
#[test]
fn layout_in_the_export_is_ignored() {
    // A copy of spec/dialogue/examples/two-keepers.json: the packaged crate
    // ships only src/**, and check-msrv-core tests it unpacked. The EK
    // harness's spec-limits test holds the copy to the spec, byte for byte.
    let two_keepers: Value = serde_json::from_str(include_str!("two-keepers.json")).unwrap();
    assert!(two_keepers.get("x-layout").is_some());
    let fake = FakeLingara::start(move |_, _| exported(&two_keepers, 4, ETAG));
    let embed = started(config(&fake.url));
    let d = embed.dialogue_load(stored(ID), session("old_wei")).unwrap();
    let (_, before) = wait_for(&embed, 5 * SECS, |e| matches!(e, EmbedEvent::CheckpointReached { dialogue, .. } if *dialogue == d));
    assert!(before.iter().any(loaded(d)), "DialogueLoaded first");
    assert_eq!(embed.dialogue_snapshot(d).unwrap().graph.unwrap().checkpoint, "suspicious_of_player");
}
