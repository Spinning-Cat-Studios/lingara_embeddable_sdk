use std::time::Duration;

use serde_json::json;

use crate::error::ErrorKind;
use crate::tests::fake_lingara::{FakeLingara, Reply};
use crate::tests::{SECS, accepted, config, context, refusal, started, wait_for};
use crate::types::{EmbedEvent, Npc, Reaction, WorldContext};

fn full() -> WorldContext {
    WorldContext {
        npc: Some(Npc { name: "Mei".into(), persona: Some("a tea seller".into()) }),
        tags: vec!["market".into(), "day-2".into()],
        generate: true,
        ..context("the night market")
    }
}

// 1.10.26y AC7
#[test]
fn context_body_bounds_and_idempotency() {
    let fake = FakeLingara::start(|_, i| match i {
        0 => Reply::JsonHeaders(503, refusal("unavailable"), vec![("retry-after", "1".into())]),
        _ => {
            let mut body = accepted("lgr_evt_9");
            body["reaction"] = json!({ "status": "started", "plan_id": "p1", "plan_status": "generating" });
            Reply::Json(202, body)
        }
    });
    let embed = started(config(&fake.url));
    let request = embed.send_context(full()).unwrap();
    let (sent, _) = wait_for(&embed, 5 * SECS, |e| matches!(e, EmbedEvent::WorldSent { .. }));
    let reaction = Reaction { status: "started".into(), plan_id: Some("p1".into()), plan_status: Some("generating".into()), code: None, error: None };
    assert_eq!(sent, EmbedEvent::WorldSent { request, event_id: "lgr_evt_9".into(), reaction: Some(reaction) });
    let seen = fake.seen();
    let want = json!({ "type": "world.context_changed", "data": {
        "scene": "the night market", "source_lang": "en", "target_lang": "zh", "level": 2,
        "npc": { "name": "Mei", "persona": "a tea seller" }, "tags": ["market", "day-2"], "generate": true,
    }});
    assert_eq!(seen[0].body, want);
    let key = seen[0].header("idempotency-key").expect("a key");
    assert_eq!(key.len(), 32);
    assert_eq!(seen[1].header("idempotency-key"), Some(key), "one key across the 503 retry");

    let bad = [
        WorldContext { scene: "x".repeat(161), ..full() },
        WorldContext { scene: "   ".into(), ..full() },
        WorldContext { level: 0, ..full() },
        WorldContext { level: 10, ..full() },
        WorldContext { tags: vec!["Market".into()], ..full() },
        WorldContext { tags: (0..9).map(|i| format!("t{i}")).collect(), ..full() },
        WorldContext { tags: vec!["a".repeat(25)], ..full() },
        WorldContext { npc: Some(Npc { name: "n".repeat(33), persona: None }), ..full() },
        WorldContext { npc: Some(Npc { name: "Mei".into(), persona: Some("p".repeat(121)) }), ..full() },
    ];
    for context in bad {
        let err = embed.send_context(context.clone()).expect_err(&format!("{context:?}"));
        assert_eq!(err.kind, ErrorKind::InvalidInput);
    }
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(fake.seen().len(), 2, "an out-of-bounds context sends nothing");
}
