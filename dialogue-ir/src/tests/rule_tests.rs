//! The rule words (ADR 3.11.26g §9): AC10.

use serde_json::Value;

use crate::text::TextFault;
use crate::{Rule, WarningCode};

fn fixture() -> Value {
    let path = format!("{}/src/tests/fixtures/backend_ir_rules.json", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))).unwrap()
}

fn words(value: &Value) -> Vec<&str> {
    value.as_array().unwrap().iter().map(|w| w.as_str().unwrap()).collect()
}

// 3.11.26g AC10
#[test]
fn rule_words_match_backend() {
    let backend = fixture();
    let ours: Vec<&str> = Rule::ALL.iter().map(|r| r.as_str()).collect();
    for word in words(&backend["rules"]) {
        assert!(ours.contains(&word), "Backend's rule {word} has no Rule");
    }
    let v2 = [
        "kind_mismatch", "steer_target", "pool_escape", "atom_set", "atom_wording", "plan_anchor", "plan_atom", "plan_params",
        "plan_reference", "plan_topic_closed", "plan_horizon", "plan_shape", "plan_ids", "plan_exits", "plan_text",
    ];
    let published = words(&backend["rules"]);
    let added: Vec<&str> = ours.iter().copied().filter(|w| !published.contains(w)).collect();
    assert_eq!(added, v2);

    let warning_codes = [
        WarningCode::CheckpointUnreachable,
        WarningCode::OutcomeUnreachable,
        WarningCode::StanceContradiction,
        WarningCode::SteerWeak,
        WarningCode::Text(TextFault::Blank),
        WarningCode::Text(TextFault::LineBreak),
    ];
    let ours: Vec<&str> = warning_codes.iter().map(|w| w.as_str()).collect();
    let backend_only = words(&backend["backend_only_warnings"]);
    for word in words(&backend["warnings"]).into_iter().filter(|w| !backend_only.contains(w)) {
        assert!(ours.contains(&word), "Backend's warning {word} has no WarningCode");
    }
    assert!(backend_only.iter().all(|w| !ours.contains(w)), "a Backend-only word is emitted here");
    assert_eq!(&ours[2..4], ["stance_contradiction", "steer_weak"]);
}
