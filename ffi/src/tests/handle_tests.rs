use std::collections::HashSet;
use std::time::{Duration, Instant};

use crate::tests::loopback::Loopback;
use crate::tests::{DIALOGUE, OFFLINE, answered_id, exec, open, poll, token};
use crate::*;

/// A raw handle, to move to a second thread. The ABI makes it `Send + Sync` (D4).
#[derive(Clone, Copy)]
struct Shared(usize);

impl Shared {
    fn get(self) -> *mut LingaraEmbed {
        self.0 as *mut LingaraEmbed
    }
}

// 1.10.26aa AC8
#[test]
fn execute_and_poll_from_two_threads() {
    let handle = Shared(open(OFFLINE) as usize);
    let sayer = std::thread::spawn(move || {
        let h = handle.get();
        let requests: Vec<u64> = (0..50)
            .map(|_| {
                let dialogue = answered_id(h, DIALOGUE);
                answered_id(h, &format!(r#"{{"type": "dialogue_say", "dialogue": {dialogue}, "line": "hello"}}"#))
            })
            .collect();
        // Every turn is waiting for the token before the host gives up.
        std::thread::sleep(Duration::from_millis(300));
        assert_eq!(exec(h, r#"{"type": "token_unavailable", "reason": "offline"}"#).0, LINGARA_EMBED_OK);
        requests
    });

    let (mut needs, mut errors) = (0, Vec::new());
    let deadline = Instant::now() + Duration::from_secs(10);
    while errors.len() < 50 && Instant::now() < deadline {
        match poll(handle.get()) {
            Some(event) if event["type"] == "token_needed" => needs += 1,
            Some(event) if event["type"] == "error" => errors.push(event["error"]["request"].as_u64().expect("an answering error")),
            Some(other) => panic!("unexpected event {other}"),
            None => std::thread::sleep(Duration::from_millis(2)),
        }
    }
    let requests: HashSet<u64> = sayer.join().expect("the second thread").into_iter().collect();
    std::thread::sleep(Duration::from_millis(100));
    assert!(poll(handle.get()).is_none(), "nothing repeated");
    assert_eq!(needs, 1, "one token_needed");
    assert_eq!(errors.len(), 50, "no event lost");
    let seen: HashSet<u64> = errors.iter().copied().collect();
    assert_eq!(seen, requests, "each error answers one of the returned ids, once");
    unsafe { lingara_embed_free(handle.get()) };
}

// 1.10.26aa AC9
#[test]
fn free_stops_the_io_thread_within_a_second() {
    let peer = Loopback::start();
    let handle = open(&format!(r#"{{"base_url": "{}", "stream_on_start": true}}"#, peer.url));
    let provide = format!(r#"{{"type": "provide_token_for", "token": "{}", "expires_in_secs": 900}}"#, token(1));
    assert_eq!(exec(handle, &provide).0, LINGARA_EMBED_OK);
    let deadline = Instant::now() + Duration::from_secs(5);
    while peer.opened() == 0 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(peer.opened(), 1, "the stream is held open");

    let begun = Instant::now();
    unsafe { lingara_embed_free(handle) };
    assert!(begun.elapsed() < Duration::from_millis(1_100), "free returned in {:?}", begun.elapsed());
    // The connection belonged to the I/O thread: it is dropped with it.
    let deadline = Instant::now() + Duration::from_secs(1);
    while peer.closed() == 0 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(peer.closed(), 1, "the I/O thread's connection is gone");
}
