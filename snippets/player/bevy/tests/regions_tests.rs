//! The Bevy page's regions (ADR 1.10.26z D13; P1 §3): every key the page
//! reads, and P1's two credential guards, checked here before P1's sync
//! ever sees a region.

use std::collections::BTreeMap;

// The snippet is a binary, so its `dialogueGraph` region is reached by
// mounting the file as a module (ADR 28.10.26g D8). Its systems run only in
// `main`, which a test never calls.
#[allow(dead_code)]
#[path = "../src/main.rs"]
mod snippet;

const SOURCE: &str = include_str!("../src/main.rs");
const INSTALL: &str = include_str!("../install.sh");
/// The worked dialogue's IR the docs show whole (D8).
const SNIPPET_IR: &str = include_str!("../../../dialogue/missing-brother.json");

/// Every region's text, joined per key in file order.
fn regions() -> BTreeMap<String, String> {
    let mut out: BTreeMap<String, String> = BTreeMap::new();
    let mut open: Option<String> = None;
    for line in SOURCE.lines() {
        let marker = line.trim();
        if let Some(key) = marker.strip_prefix("// lingara:begin ") {
            assert!(open.is_none(), "region {key} opens inside another");
            open = Some(key.trim().to_owned());
        } else if marker == "// lingara:end" {
            assert!(open.take().is_some(), "an end with no begin");
        } else if let Some(key) = &open {
            let text = out.entry(key.clone()).or_default();
            text.push_str(line);
            text.push('\n');
        }
    }
    assert!(open.is_none(), "an unclosed region");
    out
}

/// `lgr_[a-z]+_[A-Za-z0-9]{8,}`, without a regex crate.
fn holds_a_token(text: &str) -> bool {
    text.match_indices("lgr_").any(|(at, _)| {
        let rest = &text[at + 4..];
        let kind = rest.chars().take_while(char::is_ascii_lowercase).count();
        kind > 0 && rest[kind..].starts_with('_') && rest[kind + 1..].chars().take_while(char::is_ascii_alphanumeric).count() >= 8
    })
}

/// 1.10.26z AC16, 28.10.26g AC14: the keys, and no region holds or names a credential.
#[test]
fn regions_are_present_and_hold_no_credential() {
    let regions = regions();
    let keys: Vec<&str> = regions.keys().map(String::as_str).collect();
    assert_eq!(keys, ["dialogue", "dialogueGraph", "poll", "practice", "tokenSource"]);
    assert_eq!(INSTALL.trim(), "cargo add bevy_lingara@{{version}}", "the install key is the whole file");
    for (key, text) in regions.iter().map(|(k, t)| (k.as_str(), t.as_str())).chain([("install", INSTALL)]) {
        assert!(!holds_a_token(text), "region {key} holds a token");
        for word in ["lgr_cs_", "lgr_whsec_", "lgr_at_", "client_secret", "LINGARA_CLIENT_SECRET"] {
            assert!(!text.contains(word), "player region {key} mentions {word}");
        }
    }
    assert!(holds_a_token("lgr_et_abcd1234") && !holds_a_token("lgr_et_…"), "the guard itself");
}

/// 28.10.26g AC17: the region's dialogue is the IR the docs show, to the byte.
#[test]
fn the_graph_region_writes_the_snippet_ir() {
    let ir = snippet::missing_brother().expect("the region's dialogue builds");
    assert_eq!(ir.to_canonical_json(), SNIPPET_IR);
}
