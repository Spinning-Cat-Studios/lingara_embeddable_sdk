//! The within-spec check (ADR 3.11.26g §6, §8): AC6 and AC9.

use serde_json::Value;

use super::procedural_tests::{case, ferryman, ferryman_value, fragment};
use crate::procedural::{AtomKind, Beat, Exit, GAME_WORDING, PlanFragment, ProceduralDialogue, expand, seed, within_spec, within_spec_restored};
use crate::{Document, document_from_value};

/// The vector document with a case's `spec_patch` merged in, unvalidated.
fn patched(spec_patch: &Value) -> ProceduralDialogue {
    let mut value = ferryman_value();
    if let Some(patch) = spec_patch.as_object() {
        for (key, member) in patch {
            value["procedural"][key] = member.clone();
        }
    }
    match document_from_value(&value).unwrap() {
        Document::Procedural(doc) => doc,
        other => panic!("{other:?}"),
    }
}

/// One case's answer: `ok` or the refusal's rule word.
fn answer(c: &Value) -> String {
    let doc = patched(&c["spec_patch"]);
    let npc = c["npc"].as_str().unwrap();
    let frag = fragment(&c["fragment"]);
    let result = if c["mode"] == "restore" {
        within_spec_restored(&doc, npc, &frag, &GAME_WORDING)
    } else {
        let frontier = c.get("frontier").map(fragment);
        let open: Vec<&str> = c["open_topics"].as_array().unwrap().iter().map(|t| t.as_str().unwrap()).collect();
        within_spec(&doc, npc, frontier.as_ref(), &open, c["prefix"].as_str().unwrap(), &frag, &GAME_WORDING)
    };
    result.map_or_else(|e| e.rule.as_str().to_string(), |()| "ok".into())
}

// 3.11.26g AC9
#[test]
fn fragment_vectors_answer_their_rule() {
    let all: Value = serde_json::from_str(&super::builder_tests::vector("fragments.json")).unwrap();
    let cases = all["cases"].as_array().unwrap();
    for c in cases {
        assert_eq!(answer(c), c["expect"].as_str().unwrap(), "{}", c["name"]);
    }
    let expected: Vec<&str> = cases.iter().map(|c| c["expect"].as_str().unwrap()).collect();
    for rule in ["plan_anchor", "plan_atom", "plan_params", "plan_reference", "plan_topic_closed", "pool_escape", "plan_horizon", "plan_shape", "plan_ids", "plan_exits", "plan_text", "outcome_name"] {
        assert!(expected.contains(&rule), "no case answers {rule}");
    }
    assert_eq!(answer(&case("w10_railroad_declines_every_detour")), "plan_exits");
    // `seed`'s output is the `seed` case's fragment, and it passes with no frontier.
    let doc = ferryman();
    assert_eq!(seed(&doc, "ma_ren", "s1"), fragment(&case("seed")["fragment"]));
    for name in ["accepted_replan", "seed", "restored_mid_run"] {
        assert_eq!(answer(&case(name)), "ok", "{name}");
    }
}

fn beat(id: String, atom: AtomKind, depth: u8) -> Beat {
    Beat { id, atom, depth, sketch: false, topic: None, goal: None, ending: None, angle: None, exits: vec![] }
}

/// Every atom × topic (or none) × depth up to the horizon, as a chain under
/// an Open anchor, each with and without a detour onto its topic: a fixed
/// enumeration, no RNG.
fn enumeration(doc: &ProceduralDialogue) -> Vec<PlanFragment> {
    let plan_depth = doc.procedural.horizon.plan_depth;
    let topics: Vec<Option<String>> = std::iter::once(None).chain(doc.procedural.topics.iter().map(|t| Some(t.id.clone()))).collect();
    let mut out = Vec::new();
    for atom in [AtomKind::Open, AtomKind::Elicit, AtomKind::Reveal, AtomKind::Practise, AtomKind::Close] {
        for topic in &topics {
            for depth in 1..=plan_depth {
                for detour in [false, true] {
                    out.push(chain(atom, topic.clone(), depth, plan_depth, detour));
                }
            }
        }
    }
    out
}

fn chain(atom: AtomKind, topic: Option<String>, depth: u8, plan_depth: u8, detour: bool) -> PlanFragment {
    let id = |n: u8| format!("e1_{n}");
    let mut beats = vec![beat(id(0), AtomKind::Open, 0)];
    for d in 1..depth {
        beats.push(Beat { topic: Some("cargo".into()), ..beat(id(d), AtomKind::Elicit, d) });
    }
    let mut target = Beat { topic: topic.clone(), ..beat(id(depth), atom, depth) };
    target.goal = (atom == AtomKind::Practise).then(|| "fare_agreed".into());
    target.ending = (atom == AtomKind::Close).then(|| "crossed".into());
    if atom != AtomKind::Close {
        let sketch = depth == plan_depth;
        beats_push_close(&mut beats, &mut target, id(depth + 1), depth + 1, sketch);
    }
    beats.push(target);
    for d in 0..depth {
        let to = id(d + 1);
        beats[usize::from(d)].exits.push(Exit { to, detour: None });
    }
    if let (true, Some(t)) = (detour, topic) {
        beats[0].exits.push(Exit { to: "e1_9".into(), detour: Some(t.clone()) });
        let mut aside = Beat { topic: Some(t), ..beat("e1_9".into(), AtomKind::Reveal, 1) };
        beats_push_close(&mut beats, &mut aside, "e1_8".into(), 2, plan_depth == 1);
        beats.push(aside);
    }
    PlanFragment { anchor: id(0), beats }
}

fn beats_push_close(beats: &mut Vec<Beat>, target: &mut Beat, id: String, depth: u8, sketch: bool) {
    target.exits.push(Exit { to: id.clone(), detour: None });
    beats.push(Beat { sketch, ending: Some("crossed".into()), ..beat(id, AtomKind::Close, depth) });
}

// 3.11.26g AC6
#[test]
fn no_accepted_fragment_discloses_outside_the_pool() {
    let doc = ferryman();
    let open: Vec<&str> = doc.procedural.topics.iter().map(|t| t.id.as_str()).collect();
    let mut accepted = 0;
    for frag in enumeration(&doc) {
        if within_spec(&doc, "old_li", None, &open, "e1", &frag, &GAME_WORDING).is_err() {
            continue;
        }
        accepted += 1;
        let ir = expand(&doc, "old_li", &frag).unwrap();
        for checkpoint in &ir.checkpoints {
            assert!(checkpoint.may_disclose.iter().all(|k| doc.procedural.disclosable.contains(k)), "{frag:?}");
        }
    }
    assert!(accepted > 20, "only {accepted} fragments accepted");
}
