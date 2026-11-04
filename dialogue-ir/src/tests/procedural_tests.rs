//! The procedural kind (ADR 3.11.26g §4, §5, §7): AC5, AC7 and AC24.

use serde_json::{Value, json};

use super::builder_tests::vector;
use crate::procedural::{AtomText, AtomWording, GAME_WORDING, PlanFragment, ProceduralDialogue, expand, expand_with, within_spec};
use crate::{Document, DialogueIr, Rule, Strictness, document_from_value, from_json, load_document, validate, validate_with};

pub(super) fn ferryman_value() -> Value {
    serde_json::from_str(&vector("ferryman.procedural.json")).unwrap()
}

pub(super) fn ferryman() -> ProceduralDialogue {
    match load_document(&vector("ferryman.procedural.json")).unwrap().0 {
        Document::Procedural(doc) => doc,
        other => panic!("not procedural: {other:?}"),
    }
}

/// One case of `fragments.json`, by name.
pub(super) fn case(name: &str) -> Value {
    let all: Value = serde_json::from_str(&vector("fragments.json")).unwrap();
    all["cases"].as_array().unwrap().iter().find(|c| c["name"] == name).unwrap_or_else(|| panic!("no case {name}")).clone()
}

pub(super) fn fragment(value: &Value) -> PlanFragment {
    serde_json::from_value(value.clone()).unwrap()
}

fn spec_refusal(mutate: impl FnOnce(&mut Value)) -> (Rule, String) {
    let mut value = ferryman_value();
    mutate(&mut value["procedural"]);
    let err = document_from_value(&value).and_then(|doc| validate_with(&doc, Strictness::Load)).expect_err("refused");
    (err.rule, err.path)
}

// 3.11.26g AC5
#[test]
fn spec_rules_refuse_their_counter_examples() {
    type Case = (Rule, &'static str, Box<dyn FnOnce(&mut Value)>);
    let cases: Vec<Case> = vec![
        (Rule::IdPattern, "procedural.topics[0].id", Box::new(|p| p["topics"][0]["id"] = json!("Cargo"))),
        (Rule::DuplicateId, "procedural.topics[1].id", Box::new(|p| p["topics"][1]["id"] = json!("cargo"))),
        (Rule::UnknownTarget, "procedural.topics[4].npcs[0]", Box::new(|p| p["topics"][4]["npcs"] = json!(["nobody"]))),
        (Rule::UndeclaredFact, "procedural.disclosable[0]", Box::new(|p| p["disclosable"][0] = json!("ghost"))),
        (Rule::PoolEscape, "procedural.topics[0].discloses[0]", Box::new(|p| p["topics"][0]["discloses"] = json!(["fare_paid"]))),
        (Rule::PredicateUnparsed, "procedural.topics[3].when", Box::new(|p| p["topics"][3]["when"] = json!("fare_paid =="))),
        (Rule::MissingMember, "procedural.goals[0]", Box::new(|p| p["goals"][0] = json!({ "id": "fare_agreed", "describe": "a fare" }))),
        (Rule::AtomSet, "procedural.atoms", Box::new(|p| drop(p["atoms"].as_array_mut().unwrap().pop()))),
        (Rule::NumberRange, "procedural.horizon.plan_depth", Box::new(|p| p["horizon"]["plan_depth"] = json!(4))),
        (Rule::UnknownTarget, "procedural.termination.on_limit", Box::new(|p| p["termination"]["on_limit"] = json!("nowhere"))),
        (Rule::TextMultiline, "procedural.premise", Box::new(|p| p["premise"] = json!("Cross the river\nbefore dark"))),
    ];
    for (rule, path, mutate) in cases {
        assert_eq!(spec_refusal(mutate), (rule, path.to_string()), "{path}");
    }
    // The vector loads as procedural, and the graph reader refuses it by name.
    let (doc, warnings) = load_document(&vector("ferryman.procedural.json")).unwrap();
    assert!(matches!(doc, Document::Procedural(_)) && warnings.is_empty(), "{warnings:?}");
    let err = from_json(&vector("ferryman.procedural.json")).unwrap_err();
    assert_eq!((err.rule, err.message.as_str()), (Rule::KindMismatch, "is a procedural dialogue: use load_document"));
    // A member of the other kind is `kind_mismatch`.
    let mut mixed = ferryman_value();
    mixed["start"] = json!("p1_0");
    assert_eq!(document_from_value(&mixed).unwrap_err().rule, Rule::KindMismatch);
}

fn accepted_replan(npc: &str) -> DialogueIr {
    expand(&ferryman(), npc, &fragment(&case("accepted_replan")["fragment"])).unwrap()
}

// 3.11.26g AC7
#[test]
fn expansion_matches_the_templates() {
    let ir = accepted_replan("ma_ren");
    assert_eq!(ir.to_canonical_json(), vector("ferryman.expanded.json").trim_end());
    assert!(validate(&ir).is_ok());
    GAME_WORDING.check().unwrap();

    let goal = |id: &str| ir.checkpoint(id).unwrap().goal.clone();
    assert_eq!(goal("p2_1"), "Find out from the player what the player is carrying across");
    assert_eq!(goal("p3_1"), "Work with the player on the price of the crossing until a fare both sides accept");
    assert_eq!(goal("p3_1_again"), "Respond to the player's attempt and invite another");
    assert_eq!(goal("p3_2"), "Talk about the flood on the upper river; share what you know when it fits, as a warning");
    assert_eq!(goal("p3_3"), "Wrap up the conversation and say goodbye");
    let when = |checkpoint: &str, edge: &str| ir.checkpoint(checkpoint).unwrap().edges.iter().find(|e| e.id == edge).unwrap().when.clone();
    assert_eq!(when("p2_1", "p2_1_x0").as_deref(), Some("The player has said something about what the player is carrying across."));
    assert_eq!(when("p2_1", "p2_1_x1").as_deref(), Some("The player brings up the flood on the upper river."));
    assert_eq!(when("p3_2", "p3_2_x0").as_deref(), Some("The character has shared what they know about the flood on the upper river."));
    assert_eq!(when("p3_3", "p3_3_end").as_deref(), Some("The player says goodbye or the conversation has come to a natural end."));
    assert_eq!(ir.checkpoint("p3_2").unwrap().may_disclose, ["flood_rumour"]);

    // Practise with both `goal.when` and `goal.predicate`.
    let ids = |id: &str| ir.checkpoint(id).unwrap().edges.iter().map(|e| e.id.as_str()).collect::<Vec<_>>();
    assert_eq!(ids("p3_1"), ["p3_1_x0", "p3_1_ok", "p3_1_try"]);
    assert_eq!(ids("p3_1_again"), ["p3_1_again_x0", "p3_1_again_ok", "p3_1_retry"]);
    let steer = |ir: &DialogueIr, id: &str| ir.checkpoint(id).unwrap().steer_to.clone();
    assert_eq!((steer(&ir, "p3_1").as_deref(), steer(&ir, "p3_1_again").as_deref()), (Some("p3_1_x0"), Some("p3_1_again_x0")));
    let medium = accepted_replan("old_li");
    assert!(medium.checkpoints.iter().all(|c| c.steer_to.is_none()));
}

/// A test-only wording: every template re-worded, Practise's goal without ‹goal›.
const PLAIN: AtomWording = AtomWording {
    open: AtomText { label: "Hello", goal: "Say hello‹angle›", done_when: "Hello said.", again_goal: None, try_when: None, retry_when: None },
    elicit: AtomText { label: "Ask", goal: "Ask about ‹topic›‹angle›", done_when: "Told about ‹topic›.", again_goal: None, try_when: None, retry_when: None },
    reveal: AtomText { label: "Tell", goal: "Tell about ‹topic›‹angle›", done_when: "Heard about ‹topic›.", again_goal: None, try_when: None, retry_when: None },
    practise: AtomText {
        label: "Drill",
        goal: "Drill ‹topic›‹angle›",
        done_when: "",
        again_goal: Some("Drill again"),
        try_when: Some("A try."),
        retry_when: Some("Another try."),
    },
    close: AtomText { label: "Bye", goal: "Say goodbye‹angle›", done_when: "Goodbye said.", again_goal: None, try_when: None, retry_when: None },
    detour_when: "Brings up ‹topic›.",
};

// 3.11.26g AC24
#[test]
fn wording_is_separate_from_kind() {
    PLAIN.check().unwrap();
    let doc = ferryman();
    let replan = case("accepted_replan");
    let fragment = fragment(&replan["fragment"]);
    let (game, plain) = (expand(&doc, "ma_ren", &fragment).unwrap(), expand_with(&doc, "ma_ren", &fragment, &PLAIN).unwrap());
    let structure = |ir: &DialogueIr| {
        let edges = |c: &crate::Checkpoint| c.edges.iter().map(|e| (e.id.clone(), e.to.clone(), e.predicate.clone())).collect::<Vec<_>>();
        ir.checkpoints.iter().map(|c| (c.id.clone(), edges(c), c.may_disclose.clone(), c.steer_to.clone())).collect::<Vec<_>>()
    };
    assert_eq!(structure(&game), structure(&plain));
    assert_ne!(game.checkpoint("p2_1").unwrap().goal, plain.checkpoint("p2_1").unwrap().goal);
    let frontier = super::procedural_tests::fragment(&replan["frontier"]);
    let answer = |w: &AtomWording| within_spec(&doc, "ma_ren", Some(&frontier), &[], "p3", &fragment, w).map_err(|e| e.rule);
    assert_eq!(answer(&GAME_WORDING), answer(&PLAIN));

    // A Close template naming ‹topic› fails, and W12 runs before W1.
    let bad = AtomWording { close: AtomText { goal: "Leave ‹topic›", ..PLAIN.close }, ..PLAIN };
    assert_eq!(bad.check().unwrap_err().rule, Rule::AtomWording);
    let mut anchorless = fragment.clone();
    anchorless.anchor = "nowhere".into();
    assert_eq!(within_spec(&doc, "ma_ren", Some(&frontier), &[], "p3", &anchorless, &bad).unwrap_err().rule, Rule::AtomWording);
}
