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

/// The vendored `dialogue.v1` schema (ADR 28.10.26f D9): frozen by name.
fn dialogue_file(name: &str) -> String {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../spec/dialogue");
    std::fs::read_to_string(format!("{root}/{name}")).unwrap_or_else(|e| panic!("{name}: {e} (run make sync-embed-spec)"))
}

fn at(schema: &Value, pointer: &str) -> u64 {
    schema.pointer(pointer).and_then(Value::as_u64).unwrap_or_else(|| panic!("{pointer} is not in the schema"))
}

/// Every `dialogue.v1` bound `lingara-dialogue-ir` holds against the schema,
/// N1's two languages bounds and the turn's graph bounds against the frozen
/// OpenAPI, and N2's worked example loading cleanly.
fn dialogue_limits_match(spec: &Value) {
    let schema: Value = serde_json::from_str(&dialogue_file("dialogue.v1.schema.json")).unwrap();
    assert_pairs(&schema, &v1_pairs());
    assert_eq!(schema.pointer("/$defs/id/pattern").and_then(Value::as_str), Some(ID_PATTERN));
    assert_eq!(schema.pointer("/$defs/outcome/enum"), Some(&serde_json::json!(OUTCOMES)));
    turn_limits_match(spec);
}

/// v1's frozen bounds. They govern the same members in v2 (ADR 3.11.26g §1).
fn v1_pairs() -> Vec<(&'static str, u64)> {
    vec![
        ("/$defs/id/maxLength", n(MAX_ID_CHARS)),
        ("/properties/title/maxLength", n(MAX_TITLE_CHARS)),
        ("/$defs/checkpoint/properties/goal/maxLength", n(MAX_GOAL_CHARS)),
        ("/$defs/edge/properties/when/maxLength", n(MAX_CONDITION_CHARS)),
        ("/$defs/edge/properties/predicate/maxLength", n(MAX_PREDICATE_CHARS)),
        ("/$defs/checkpoint/properties/edges/maxItems", n(MAX_EDGES_PER_CHECKPOINT)),
        ("/$defs/checkpoint/properties/may_disclose/maxItems", n(MAX_DISCLOSE)),
        ("/$defs/fact/allOf/0/then/properties/default/maxLength", n(MAX_FACT_STRING_CHARS)),
        ("/properties/facts/maxItems", n(MAX_FACTS)),
        ("/properties/npcs/maxItems", n(MAX_NPCS)),
        ("/properties/checkpoints/maxItems", n(MAX_CHECKPOINTS)),
        ("/x-maxBytes", n(MAX_IR_BYTES)),
        ("/$defs/npc/properties/name/maxLength", n(MAX_NPC_NAME_CHARS)),
        ("/$defs/npc/properties/persona/maxLength", n(MAX_NPC_PERSONA_CHARS)),
        ("/$defs/npc/properties/languages/maxItems", n(MAX_NPC_LANGUAGES)),
        ("/$defs/language/properties/code/maxLength", n(MAX_LANGUAGE_CODE_CHARS)),
        ("/$defs/language/properties/speaking/minimum", u64::from(MIN_LEVEL)),
        ("/$defs/language/properties/speaking/maximum", u64::from(MAX_LEVEL)),
    ]
}

fn assert_pairs<P: AsRef<str>>(schema: &Value, pairs: &[(P, u64)]) {
    let wrong: Vec<String> = pairs.iter().map(|(p, want)| (p.as_ref(), *want)).filter(|(p, want)| at(schema, p) != *want).map(|(p, want)| format!("{p}: schema {} ≠ limits {want}", at(schema, p))).collect();
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// The turn's graph bounds against the frozen OpenAPI, and N2's worked example.
fn turn_limits_match(spec: &Value) {
    let turn = [
        ("DialogueNpc/properties/languages/maxItems", n(NPC_LANGUAGES_MAX_ITEMS)),
        ("NpcLanguage/properties/code/maxLength", n(NPC_LANGUAGE_CODE_MAX_CHARS)),
        ("DialogueTurnRequest/properties/edges/maxItems", n(MAX_EDGES_PER_CHECKPOINT)),
        ("GraphEdge/properties/when/maxLength", n(MAX_CONDITION_CHARS)),
        ("GraphCheckpoint/properties/goal/maxLength", n(MAX_GOAL_CHARS)),
    ];
    let wrong: Vec<String> = turn.iter().filter(|(p, want)| bound(spec, p) != *want).map(|(p, want)| format!("{p}: spec {} ≠ limits {want}", bound(spec, p))).collect();
    assert!(wrong.is_empty(), "{wrong:#?}");

    let example = dialogue_file("examples/two-keepers.json");
    lingara_embed::dialogue_ir::load(&example).unwrap_or_else(|e| panic!("two-keepers.json: {e}"));
    assert!(example == include_str!("../../src/tests/two-keepers.json"), "core/src/tests/two-keepers.json is not the spec's example: copy it again");
}

/// Every `dialogue.v2` bound in `limits.rs` against the vendored v2 schema
/// (ADR 3.11.26g Phase B). `MAX_ANGLE_CHARS`, `MAX_BEAT_ID_CHARS` and
/// `MAX_EXITS_PER_BEAT` bound a planner's fragment, which the document schema
/// does not describe.
fn v2_pairs() -> Vec<(String, u64)> {
    let d = "/$defs";
    let p = |s: &str| format!("{d}/procedural/properties/{s}");
    vec![
        ("/properties/endings/maxItems".to_owned(), n(MAX_ENDINGS)),
        ("/properties/endings/minItems".to_owned(), 1),
        (format!("{d}/ending/properties/label/maxLength"), n(MAX_TITLE_CHARS)),
        (p("disclosable/maxItems"), n(MAX_DISCLOSABLE)),
        (p("topics/maxItems"), n(MAX_TOPICS)),
        (p("topics/minItems"), 1),
        (format!("{d}/topic/properties/discloses/maxItems"), n(MAX_TOPIC_DISCLOSES)),
        (p("forbidden/maxItems"), n(MAX_FORBIDDEN)),
        (p("goals/maxItems"), n(MAX_GOALS)),
        (p("premise/maxLength"), n(MAX_PREMISE_CHARS)),
        (format!("{d}/topic/properties/about/maxLength"), n(MAX_ABOUT_CHARS)),
        (format!("{d}/forbidden/properties/about/maxLength"), n(MAX_ABOUT_CHARS)),
        (format!("{d}/goal/properties/describe/maxLength"), n(MAX_DESCRIBE_CHARS)),
        (format!("{d}/horizon/properties/plan_depth/maximum"), u64::from(MAX_PLAN_DEPTH)),
        (format!("{d}/horizon/properties/plan_depth/minimum"), 1),
        (format!("{d}/horizon/properties/sketch_depth/maximum"), u64::from(MAX_SKETCH_DEPTH)),
        (format!("{d}/horizon/properties/sketch_depth/minimum"), 0),
        (format!("{d}/horizon/properties/max_nodes/maximum"), u64::from(MAX_FRONTIER_NODES)),
        (format!("{d}/horizon/properties/max_nodes/minimum"), 3),
        (format!("{d}/termination/properties/max_turns/maximum"), u64::from(MAX_TURNS)),
        (format!("{d}/termination/properties/max_beats/maximum"), u64::from(MAX_BEATS)),
        (format!("{d}/termination/properties/stall/properties/turns/minimum"), u64::from(MIN_STALL_TURNS)),
        (format!("{d}/termination/properties/stall/properties/turns/maximum"), u64::from(MAX_STALL_TURNS)),
    ]
}

// 3.11.26g AC12: the v2 bounds, v1's bounds where v2 keeps them, and the byte cap.
#[test]
fn v2_limits_match_the_schema() {
    let schema: Value = serde_json::from_str(&dialogue_file("dialogue.v2.schema.json")).unwrap();
    assert_eq!(schema.pointer("/properties/schema/const").and_then(Value::as_str), Some("dialogue.v2"));
    assert_pairs(&schema, &v2_pairs());
    assert_pairs(&schema, &v1_pairs());
    assert_eq!(schema.pointer("/$defs/id/pattern").and_then(Value::as_str), Some(ID_PATTERN));
    for def in ["ending", "checkpoint"] {
        let not = format!("/$defs/{def}/properties/id/allOf/1/not/const");
        assert_eq!(schema.pointer(&not).and_then(Value::as_str), Some(ABANDONED), "{not}");
    }
}

// 1.10.26y AC26; 28.10.26f AC17 (the dialogue graph's bounds, below)
#[test]
fn limits_match_the_frozen_schema() {
    let spec = frozen();
    dialogue_limits_match(&spec);
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
