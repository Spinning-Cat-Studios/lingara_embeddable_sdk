//! `dialogue.v2`'s static half (ADR 3.11.26g §1–§3, §9, §11): AC1–AC4 and
//! AC14–AC16.

use serde_json::{Value, json};

use super::builder_tests::vector;
use crate::disposition::{Drive, Following, Stance, drive, following};
use crate::ir::{Disposition, Ending};
use crate::procedural::ProceduralDialogue;
use crate::version::write_schema;
use crate::{
    Document, IrError, Rule, SCHEMA_V1, SCHEMA_V2, Strictness, Warning, WarningCode, document_from_value, from_json, load_document,
    validate_with,
};

/// The v2 worked example, as a value.
fn missing_brother() -> Value {
    serde_json::from_str(&vector("missing-brother.v2.json")).unwrap()
}

fn check(value: &Value, strictness: Strictness) -> Result<Vec<Warning>, IrError> {
    validate_with(&document_from_value(value)?, strictness)
}

fn refusal(value: &Value) -> (Rule, String) {
    let err = check(value, Strictness::Load).expect_err("refused");
    (err.rule, err.path)
}

fn mutated(mutate: impl FnOnce(&mut Value)) -> Value {
    let mut value = missing_brother();
    mutate(&mut value);
    value
}

fn number(x: f64) -> serde_json::Number {
    serde_json::Number::from_f64(x).unwrap()
}

// 3.11.26g AC1
#[test]
fn v1_reads_as_v2_and_writes_back_as_v1() {
    for name in ["two-keepers.canonical.json", "two-keepers.comparison.json", "missing-brother.comparison.json"] {
        let text = vector(name);
        let ir = from_json(&text).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(ir.endings, [Ending::bare("success"), Ending::bare("failure")], "{name}");
        assert!(ir.npcs.iter().all(|n| n.disposition.is_none()) && ir.checkpoints.iter().all(|c| c.steer_to.is_none()));
        let written = if name.ends_with(".comparison.json") { ir.comparison_form() } else { ir.to_canonical_json() };
        assert_eq!(written, text.trim_end(), "{name}");
        assert!(written.contains(r#""schema":"dialogue.v1""#) && !written.contains("endings"), "{name}");
    }
}

// 3.11.26g AC2
#[test]
fn the_write_rule_names_v1_only_when_v1_can_say_it() {
    let v1 = from_json(&vector("two-keepers.canonical.json")).unwrap();
    assert_eq!(write_schema(&Document::Static(v1.clone())), SCHEMA_V1);
    type Flip = (&'static str, Box<dyn Fn(&mut crate::DialogueIr)>);
    let flips: Vec<Flip> = vec![
        ("label", Box::new(|ir| ir.endings[0].label = Some("Won".into()))),
        ("named ending", Box::new(|ir| ir.endings.push(Ending::bare("fled")))),
        ("disposition", Box::new(|ir| ir.npcs[0].disposition = Some(Disposition { suggestibility: number(0.5), initiative: number(0.5) }))),
        ("steer_to", Box::new(|ir| ir.checkpoints[0].steer_to = Some("asks_about_brother".into()))),
    ];
    for (what, flip) in flips {
        let mut ir = v1.clone();
        flip(&mut ir);
        assert_eq!(write_schema(&Document::Static(ir.clone())), SCHEMA_V2, "{what}");
        assert!(ir.to_canonical_json().contains(r#""kind":"static","#), "{what}");
    }
    // A v2 document declaring only `[success]` is written as v1 (§11.2).
    let mut only_success = v1.clone();
    only_success.endings.truncate(1);
    assert_eq!(write_schema(&Document::Static(only_success)), SCHEMA_V1);
    let procedural: ProceduralDialogue = match load_document(&vector("ferryman.procedural.json")).unwrap().0 {
        Document::Procedural(doc) => doc,
        other => panic!("{other:?}"),
    };
    assert_eq!(write_schema(&Document::Procedural(procedural)), SCHEMA_V2);
    // The v2 worked example's bytes (§10).
    let v2 = from_json(&vector("missing-brother.v2.json")).unwrap();
    assert_eq!(v2.to_canonical_json(), vector("missing-brother.v2.canonical.json"));
}

// 3.11.26g AC3
#[test]
fn stance_bands_and_detour_caps() {
    assert_eq!([drive(0.33), drive(1.0 / 3.0), drive(2.0 / 3.0), drive(0.9)], [Drive::Low, Drive::Medium, Drive::High, Drive::Railroad]);
    assert_eq!(following(0.95), Following::High);
    assert_eq!(Stance::of(None), Stance { following: Following::Medium, drive: Drive::Medium });
    let stance = |s: f64, i: f64| Stance::of(Some(&Disposition { suggestibility: number(s), initiative: number(i) }));
    assert_eq!([stance(0.1, 0.5), stance(0.5, 0.5), stance(0.9, 0.5)].map(|s| s.max_detours()), [0, 1, 2]);
    assert_eq!(stance(0.9, 0.95).max_detours(), 0, "railroad overrides following");
}

// 3.11.26g AC4
#[test]
fn steer_to_and_disposition_rules() {
    assert!(check(&missing_brother(), Strictness::Load).unwrap().is_empty());
    let no_when = mutated(|v| {
        v["checkpoints"][1]["edges"][0].as_object_mut().unwrap().remove("when");
        v["checkpoints"][1]["steer_to"] = json!("gate");
    });
    assert_eq!(refusal(&no_when), (Rule::SteerTarget, "checkpoints[1].steer_to".into()));
    let other = mutated(|v| v["checkpoints"][0]["steer_to"] = json!("gate"));
    assert_eq!(refusal(&other), (Rule::SteerTarget, "checkpoints[0].steer_to".into()));
    let range = mutated(|v| v["npcs"][0]["disposition"]["initiative"] = json!(1.5));
    assert_eq!(refusal(&range), (Rule::NumberRange, "npcs[0].disposition.initiative".into()));
    let contradiction = mutated(|v| v["npcs"][0]["disposition"]["suggestibility"] = json!(0.9));
    let warnings = check(&contradiction, Strictness::Load).unwrap();
    assert_eq!(warnings.len(), 1);
    assert_eq!((warnings[0].code, warnings[0].message.as_str()), (WarningCode::StanceContradiction, "railroad wins: detours are declined"));
    let weak = mutated(|v| v["npcs"][0]["disposition"]["initiative"] = json!(0.1));
    let codes: Vec<_> = check(&weak, Strictness::Load).unwrap().into_iter().map(|w| (w.code, w.path)).collect();
    assert_eq!(codes, [(WarningCode::SteerWeak, "checkpoints[0].steer_to".to_string())]);
}

// 3.11.26g AC14
#[test]
fn save_strictness_refuses_what_load_warns() {
    let two_lines = mutated(|v| v["checkpoints"][0]["goal"] = json!("Elena is wary\nand asks who the player is"));
    let warnings = check(&two_lines, Strictness::Load).unwrap();
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].code.as_str(), "text_multiline");
    let err = check(&two_lines, Strictness::Save).unwrap_err();
    assert_eq!((err.rule, err.path.as_str()), (Rule::TextMultiline, warnings[0].path.as_str()));
    // Every warning carries its code.
    let orphan = mutated(|v| v["checkpoints"].as_array_mut().unwrap().push(json!({ "id": "orphan", "goal": "Nobody comes" })));
    let codes: Vec<_> = check(&orphan, Strictness::Load).unwrap().into_iter().map(|w| w.code.as_str()).collect();
    assert_eq!(codes, ["checkpoint_unreachable"]);
}

// 3.11.26g AC15
#[test]
fn endings_shape_and_namespace() {
    let cases: Vec<(Rule, &str, Value)> = vec![
        (Rule::MissingMember, "endings", mutated(|v| drop(v.as_object_mut().unwrap().remove("endings")))),
        (Rule::Bound, "endings", mutated(|v| v["endings"] = json!([]))),
        (Rule::Bound, "endings", mutated(|v| v["endings"] = json!((0..9).map(|i| json!({ "id": format!("e{i}") })).collect::<Vec<_>>()))),
        (Rule::IdPattern, "endings[0].id", mutated(|v| v["endings"][0]["id"] = json!("Found"))),
        (Rule::Bound, "endings[1].label", mutated(|v| v["endings"][1]["label"] = json!("l".repeat(81)))),
        (Rule::OutcomeName, "checkpoints[1].id", mutated(|v| v["endings"][0]["id"] = json!("ask_about_brother"))),
        (Rule::OutcomeName, "endings[0].id", mutated(|v| v["endings"][0]["id"] = json!("abandoned"))),
        (Rule::UnknownTarget, "checkpoints[0].edges[1].to", mutated(|v| v["checkpoints"][0]["edges"][1]["to"] = json!("success"))),
    ];
    for (rule, path, value) in cases {
        assert_eq!(refusal(&value), (rule, path.to_string()), "{path}");
    }
    // `abandoned` needs no declaration (the worked example draws an edge to it).
    assert!(missing_brother()["endings"].as_array().unwrap().iter().all(|e| e["id"] != "abandoned"));
    assert!(check(&missing_brother(), Strictness::Load).is_ok());
}

// 3.11.26g AC16
#[test]
fn reachability_counts_abandoned_only_in_v1() {
    let only_abandoned = |schema: &str| {
        let mut value = json!({
            "schema": schema, "id": "walk_away", "npcs": [{ "id": "npc", "name": "Npc" }], "start": "a",
            "checkpoints": [{ "id": "a", "goal": "Talk", "edges": [{ "id": "leave", "to": "abandoned", "when": "the player leaves" }] }]
        });
        if schema == SCHEMA_V2 {
            value["endings"] = json!([{ "id": "done" }]);
        }
        value
    };
    let v2 = check(&only_abandoned(SCHEMA_V2), Strictness::Load).unwrap();
    assert_eq!(v2.iter().map(|w| (w.code, w.message.as_str())).collect::<Vec<_>>(), [(WarningCode::OutcomeUnreachable, "no ending reachable")]);
    assert!(check(&only_abandoned(SCHEMA_V1), Strictness::Load).unwrap().is_empty());
    let mut nowhere = only_abandoned(SCHEMA_V1);
    nowhere["checkpoints"][0]["edges"] = json!([]);
    let codes: Vec<_> = check(&nowhere, Strictness::Load).unwrap().into_iter().map(|w| (w.code, w.path)).collect();
    assert_eq!(codes, [(WarningCode::OutcomeUnreachable, "start".to_string())]);
}
