use std::time::Duration;

use serde_json::json;

use crate::embed::Embed;
use crate::error::ErrorKind;
use crate::tests::fake_lingara::{After, FakeLingara, Reply, frame};
use crate::tests::{SECS, config, refusal, started, wait_for};
use crate::types::{DialogueConfig, DialogueId, DialoguePhase, EmbedEvent, Npc, NpcTurnUpdate, Speaker, WindowEntry};

const TURNS: &str = "/v1/embed/dialogue/turns";

fn mei(window: Vec<WindowEntry>) -> DialogueConfig {
    DialogueConfig { npc: Npc { name: "Mei".into(), persona: None }, scene: None, source_lang: "en".into(), target_lang: "zh".into(), level: 2, window }
}

fn entry(speaker: Speaker, text: &str) -> WindowEntry {
    WindowEntry { speaker, text: text.into() }
}

fn turn_update(e: &EmbedEvent) -> Option<NpcTurnUpdate> {
    if let EmbedEvent::NpcTurn { update, .. } = e { Some(update.clone()) } else { None }
}

fn finished(e: &EmbedEvent) -> bool {
    matches!(turn_update(e), Some(NpcTurnUpdate::Finished { .. }))
}

/// Opens Mei over a fake whose turns are answered by `turns` (the n-th turn).
fn open(turns: impl Fn(usize) -> Reply + Send + Sync + 'static, window: Vec<WindowEntry>) -> (FakeLingara, Embed, DialogueId) {
    let fake = FakeLingara::start(move |_, i| turns(i));
    let embed = started(config(&fake.url));
    let dialogue = embed.dialogue_open(mei(window)).unwrap();
    (fake, embed, dialogue)
}

// 1.10.26y AC15
#[test]
fn a_turn_streams_finishes_and_cancels_cleanly() {
    let reply = |_| {
        let frames = vec![frame("delta", json!({ "text": "Ni" })), frame("notice", json!({ "code": "truncated", "message": "cut" })), frame("delta", json!({ "text": " hao" })), frame("done", json!({}))];
        Reply::Sse(frames, After::End)
    };
    let (fake, embed, d) = open(reply, vec![entry(Speaker::Player, "hi"), entry(Speaker::Npc, "hello")]);
    embed.dialogue_say(d, "  how are you?  ").unwrap();
    let (done, before) = wait_for(&embed, 5 * SECS, finished);
    let updates: Vec<_> = before.iter().filter_map(turn_update).collect();
    let notice = NpcTurnUpdate::Notice { code: "truncated".into(), message: "cut".into() };
    let fragment = |t: &str| NpcTurnUpdate::Fragment { text: t.into() };
    assert_eq!(updates, [NpcTurnUpdate::Opened, fragment("Ni"), notice, fragment(" hao")]);
    assert_eq!(turn_update(&done), Some(NpcTurnUpdate::Finished { text: "Ni hao".into() }));
    let want = json!({ "npc": { "name": "Mei" }, "source_lang": "en", "target_lang": "zh", "level": 2, "line": "how are you?",
        "history": [{ "speaker": "player", "text": "hi" }, { "speaker": "npc", "text": "hello" }] });
    assert_eq!(fake.seen()[0].body, want);
    assert_eq!(embed.dialogue_snapshot(d).unwrap().window.len(), 4);

    let hold = |_| Reply::Sse(vec![frame("delta", json!({ "text": "partial" }))], After::Hold);
    let (_, embed, d) = open(hold, vec![]);
    embed.dialogue_say(d, "hello").unwrap();
    wait_for(&embed, 5 * SECS, |e| matches!(turn_update(e), Some(NpcTurnUpdate::Fragment { .. })));
    // Busy is asserted on the held turn: the first turn's whole stream can
    // finish between two calls, and then a second say is accepted.
    assert_eq!(embed.dialogue_say(d, "again").unwrap_err().kind, ErrorKind::Busy);
    embed.dialogue_cancel(d).unwrap();
    wait_for(&embed, 5 * SECS, |e| matches!(turn_update(e), Some(NpcTurnUpdate::Cancelled)));
    let snapshot = embed.dialogue_snapshot(d).unwrap();
    assert_eq!((snapshot.phase, snapshot.window.len()), (DialoguePhase::Idle, 0), "a cancel discards the partial");

    let (fake, embed, d) = open(|_| Reply::Json(503, refusal("unavailable")), vec![]);
    let request = embed.dialogue_say(d, "hello").unwrap();
    let (failed, _) = wait_for(&embed, 5 * SECS, |e| matches!(e, EmbedEvent::Error { .. }));
    assert!(matches!(failed, EmbedEvent::Error { error } if error.request == Some(request) && error.kind == ErrorKind::Unavailable));
    std::thread::sleep(Duration::from_millis(500));
    assert_eq!((fake.count("POST", TURNS), embed.dialogue_snapshot(d).unwrap().window.len()), (1, 0), "a failed turn is never re-sent");
}

// 1.10.26y AC30
#[test]
fn say_checks_the_line_and_settings_keep_the_window() {
    let done = |_| Reply::Sse(vec![frame("delta", json!({ "text": "hao" })), frame("done", json!({}))], After::End);
    let supplied = vec![entry(Speaker::Player, &"长".repeat(600)), entry(Speaker::Npc, "  ")];
    let (fake, embed, d) = open(done, supplied);
    let window = embed.dialogue_snapshot(d).unwrap().window;
    assert_eq!((window.len(), window[0].text.chars().count()), (1, 500), "a supplied window is cut and an empty entry dropped");

    for bad in [String::new(), "   ".into(), "x".repeat(501), "two\nlines".into()] {
        assert_eq!(embed.dialogue_say(d, &bad).unwrap_err().kind, ErrorKind::InvalidInput, "{bad:?}");
    }
    assert_eq!(embed.dialogue_set_level(d, 0).unwrap_err().kind, ErrorKind::InvalidInput);
    embed.dialogue_set_level(d, 5).unwrap();
    embed.dialogue_set_scene(d, Some("a tea house".into())).unwrap();
    embed.dialogue_say(d, "ni hao").unwrap();
    wait_for(&embed, 5 * SECS, finished);
    let seen = fake.seen();
    assert_eq!(seen.len(), 1, "a refused line sends nothing");
    assert_eq!((&seen[0].body["level"], &seen[0].body["scene"]), (&json!(5), &json!("a tea house")));
    assert_eq!(seen[0].body["history"].as_array().unwrap().len(), 1, "settings keep the window");
}
