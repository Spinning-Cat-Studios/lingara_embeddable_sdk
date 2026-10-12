//! `PlayerRouter::handle` over E7's shared signature vectors, read at their
//! own `now` through the library's clock seam.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use lingara::BoxFuture;
use lingara::events::Webhook;
use serde_json::Value;

use super::{DirectoryError, MemoryPlayerDirectory, PlayerDirectory, PlayerRouter};

struct At(u64);

impl lingara::Clock for At {
    fn now(&self) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(self.0)
    }
}

/// A directory that counts lookups, and fails them when asked to.
#[derive(Default)]
struct Counting {
    inner: MemoryPlayerDirectory,
    lookups: AtomicUsize,
    fail: bool,
}

impl PlayerDirectory for Counting {
    fn remember<'a>(&'a self, player_ref: &'a str, subject: &'a str) -> BoxFuture<'a, Result<(), DirectoryError>> {
        self.inner.remember(player_ref, subject)
    }
    fn player_ref_for<'a>(&'a self, subject: &'a str) -> BoxFuture<'a, Result<Option<String>, DirectoryError>> {
        self.lookups.fetch_add(1, Ordering::SeqCst);
        if self.fail {
            return Box::pin(async { Err("the store is down".into()) });
        }
        self.inner.player_ref_for(subject)
    }
    fn forget<'a>(&'a self, player_ref: &'a str) -> BoxFuture<'a, Result<(), DirectoryError>> {
        self.inner.forget(player_ref)
    }
}

fn vector(name: &str) -> (Webhook, Vec<u8>, HashMap<String, String>) {
    let path = format!("{}/../conformance/vectors/webhook-signatures.json", env!("CARGO_MANIFEST_DIR"));
    let all: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("vectors")).expect("json");
    let v = all["vectors"].as_array().expect("vectors").iter().find(|v| v["name"] == name).expect(name);
    let secrets: Vec<&str> = v["secrets"].as_array().expect("secrets").iter().filter_map(Value::as_str).collect();
    let webhook = Webhook::with_secrets(secrets).expect("secrets").clock(Arc::new(At(v["now"].as_u64().expect("now"))));
    let headers = v["headers"].as_object().expect("headers").iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_string())).collect();
    (webhook, v["body"].as_str().expect("body").as_bytes().to_vec(), headers)
}

/// The router over `name`'s verifier, a directory knowing `lgr_sub_conformance`
/// as `p1` (or failing), and an `on_error` that records what it is given.
async fn router(name: &str, fail: bool) -> (PlayerRouter, Arc<Counting>, Arc<Mutex<Vec<String>>>) {
    let directory = Arc::new(Counting { fail, ..Counting::default() });
    directory.inner.remember("p1", "lgr_sub_conformance").await.expect("seed");
    let errors = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&errors);
    let router = PlayerRouter::new(vector(name).0, directory.clone()).with_on_error(move |f| sink.lock().unwrap().push(f.to_string()));
    (router, directory, errors)
}

// 1.10.26af AC19
/// Through `handle`: a tampered body is `401` and never reaches the
/// directory or `on_event`; a valid event reaches `on_event` with the
/// remembered `player_ref`; an unknown subject arrives with none; a raising
/// handler and a raising directory are each `500` and reach `on_error`.
#[tokio::test]
async fn handle_verifies_before_resolving() {
    let seen: Arc<Mutex<Vec<Option<String>>>> = Arc::default();
    let record = |seen: Arc<Mutex<Vec<Option<String>>>>| {
        move |routed: super::RoutedEvent| async move {
            seen.lock().unwrap().push(routed.player_ref);
            Ok::<(), DirectoryError>(())
        }
    };

    let (r, dir, _) = router("tampered-body", false).await;
    let (_, body, headers) = vector("tampered-body");
    assert_eq!(r.handle(&body, &headers, record(seen.clone())).await, 401);
    assert_eq!((dir.lookups.load(Ordering::SeqCst), seen.lock().unwrap().len()), (0, 0));

    let (r, dir, _) = router("valid-single-signature", false).await;
    let (_, body, headers) = vector("valid-single-signature");
    assert_eq!(r.handle(&body, &headers, record(seen.clone())).await, 204);
    assert_eq!(dir.lookups.load(Ordering::SeqCst), 1);
    assert_eq!(seen.lock().unwrap().pop(), Some(Some("p1".into())));

    dir.inner.forget("p1").await.expect("forget");
    assert_eq!(r.handle(&body, &headers, record(seen.clone())).await, 204);
    assert_eq!(seen.lock().unwrap().pop(), Some(None), "an unknown subject routes to no player");

    let (r, _, errors) = router("valid-single-signature", false).await;
    let raise = |_| async { Err::<(), DirectoryError>("the handler broke".into()) };
    assert_eq!(r.handle(&body, &headers, raise).await, 500);
    assert_eq!(errors.lock().unwrap().as_slice(), ["the event handler failed: the handler broke"]);

    let (r, _, errors) = router("valid-single-signature", true).await;
    assert_eq!(r.handle(&body, &headers, record(seen.clone())).await, 500);
    assert_eq!(errors.lock().unwrap().as_slice(), ["the player directory failed: the store is down"]);
    assert!(seen.lock().unwrap().is_empty(), "a failed lookup never calls on_event");

    let (r, _, _) = router("body-not-json", false).await;
    let (_, body, headers) = vector("body-not-json");
    assert_eq!(r.handle(&body, &headers, record(seen.clone())).await, 400, "a valid signature over a malformed payload");
}
