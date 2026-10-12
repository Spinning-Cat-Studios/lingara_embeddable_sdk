//! `lingara_embed::limits` against the frozen API version's schemas (D12):
//! a server-side change to a bound fails here instead of shipping silently.
//! Here because the packaged crate has no `spec/`.

use lingara_embed::limits::*;
use serde_json::Value;

/// The frozen current: the newest `supported` or `lts` version.
fn frozen() -> Value {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../spec");
    let registry: toml::Table = std::fs::read_to_string(format!("{root}/versions.toml")).unwrap().parse().unwrap();
    let versions = registry["version"].as_array().unwrap();
    let current = versions.iter().rev().find(|v| matches!(v["state"].as_str(), Some("supported" | "lts"))).expect("a current version");
    let path = format!("{root}/versions/{}.openapi.json", current["id"].as_str().unwrap());
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn bound(spec: &Value, pointer: &str) -> u64 {
    let at = format!("/components/schemas/{pointer}");
    spec.pointer(&at).and_then(Value::as_u64).unwrap_or_else(|| panic!("{at} is not in the frozen spec"))
}

fn n(v: usize) -> u64 {
    v as u64
}

// 1.10.26y AC26
#[test]
fn limits_match_the_frozen_schema() {
    let spec = frozen();
    let pairs = [
        ("WorldContextChanged/properties/scene/maxLength", n(SCENE_MAX_CHARS)),
        ("WorldPracticeRequested/properties/topic/maxLength", n(TOPIC_MAX_CHARS)),
        ("Npc/properties/name/maxLength", n(NPC_NAME_MAX_CHARS)),
        ("Npc/properties/persona/maxLength", n(NPC_PERSONA_MAX_CHARS)),
        ("WorldContextChanged/properties/level/minimum", u64::from(LEVEL_MIN)),
        ("WorldContextChanged/properties/level/maximum", u64::from(LEVEL_MAX)),
        ("WorldPracticeRequested/properties/level/maximum", u64::from(LEVEL_MAX)),
        ("WorldContextChanged/properties/tags/maxItems", n(TAGS_MAX_ITEMS)),
        ("WorldContextChanged/properties/tags/items/maxLength", n(TAG_MAX_CHARS)),
        ("WorldPracticeRequested/properties/tags/maxItems", n(TAGS_MAX_ITEMS)),
        ("WorldPracticeCompleted/properties/items/maxItems", n(MAX_PRACTICE_ITEMS)),
        ("DialogueTurnRequest/properties/history/maxItems", n(DIALOGUE_HISTORY_MAX_ITEMS)),
        ("DialogueTurnRequest/properties/line/maxLength", n(DIALOGUE_LINE_MAX_CHARS)),
        ("DialogueTurnRequest/properties/level/minimum", u64::from(LEVEL_MIN)),
        ("DialogueTurnRequest/properties/level/maximum", u64::from(LEVEL_MAX)),
        ("DialogueTurnRequest/properties/scene/maxLength", n(SCENE_MAX_CHARS)),
        ("DialogueEntry/properties/text/maxLength", n(DIALOGUE_TEXT_MAX_CHARS)),
    ];
    let wrong: Vec<String> = pairs.iter().filter(|(p, want)| bound(&spec, p) != *want).map(|(p, want)| format!("{p}: spec {} ≠ limits {want}", bound(&spec, p))).collect();
    assert!(wrong.is_empty(), "{wrong:#?}");
    let pattern = spec.pointer("/components/schemas/WorldContextChanged/properties/tags/items/pattern").and_then(Value::as_str);
    assert_eq!(pattern, Some(TAG_PATTERN));
}
