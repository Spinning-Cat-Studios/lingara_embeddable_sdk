//! The frontier itself (ADR 3.11.26j D2, D6): the runner's one new method,
//! what a crossing remembers, and the termination rules in order.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use lingara_dialogue_ir::procedural::{PlanFragment, expand};
use lingara_dialogue_ir::{DialogueIr, Document, FactValue, document_from_value};
use serde_json::{Value, json};

use crate::dialogue::graph::GraphState;
use crate::tests::procedural_fixture::*;
use crate::types::{AtomKind, EmbedEvent, GraphSession, PlanOutcome};

fn expanded(doc: &Value, fragment: &Value) -> Arc<DialogueIr> {
    let Ok(Document::Procedural(doc)) = document_from_value(doc) else { panic!("a procedural document") };
    let fragment: PlanFragment = serde_json::from_value(fragment.clone()).unwrap();
    Arc::new(expand(&doc, "keeper", &fragment).unwrap())
}

fn at(ir: &Arc<DialogueIr>, id: &str) -> usize {
    ir.checkpoints.iter().position(|c| c.id == id).unwrap()
}

fn ended(events: &[EmbedEvent]) -> Option<(String, Option<String>)> {
    events.iter().find_map(|e| match e {
        EmbedEvent::DialogueEnded { outcome, edge, .. } => Some((outcome.clone(), edge.clone())),
        _ => None,
    })
}

fn planned_count(events: &[EmbedEvent]) -> usize {
    events.iter().filter(|e| matches!(e, EmbedEvent::DialoguePlanned { .. })).count()
}

// 3.11.26j AC1
#[test]
fn rebase_repoints_and_never_crosses() {
    let doc = tea_stall();
    let first = expanded(&doc, &first_plan());
    let mut state = GraphState::new(Arc::clone(&first), 0, at(&first, "p1_2"));
    state.set_fact("paid", FactValue::Boolean(true)).unwrap();
    let replanned = expanded(&doc, &replan_from_practise("p1_2", "p2"));
    assert!(state.rebase(Arc::clone(&replanned), "p1_2"));
    assert_eq!(state.current().id, "p1_2", "the same checkpoint id, in the new graph");
    assert!(state.current().edges.iter().any(|e| e.to == "p2_1"), "with the new exits");
    assert_eq!(state.set_facts().get("paid"), Some(&FactValue::Boolean(true)), "the facts are kept");
    assert!(!state.ended(), "and no edge was crossed, though `p1_2_ok` is open");

    let seeded = expanded(&doc, &json!({ "anchor": "p1_0", "beats": [to(beat("p1_0", "open", 0), "p1_1"), ending(beat("p1_1", "close", 1), "sold")] }));
    assert!(!state.rebase(seeded, "p1_2"), "a graph without the checkpoint is refused");
    assert!(state.current().edges.iter().any(|e| e.to == "p2_1"), "and nothing changed");

    // A run on `‹beat›_again` asks from `‹beat›`, and stays on `_again` after the replan.
    let resumed = GraphSession { checkpoint: Some("p1_2_again".into()), frontier: Some(first_plan().to_string()), ..session() };
    let run = scripted(&doc, resumed, vec![planned(replan_from_practise("p1_2", "p2"))], vec![]);
    run.wait(|e| matches!(e, EmbedEvent::DialoguePlanned { outcome: PlanOutcome::Accepted, .. }));
    assert_eq!(run.plans()[0]["current"], "p1_2", "an exhausted Practise beat, asked from its beat id");
    assert_eq!(run.here(), "p1_2_again");
    assert_eq!(run.frontier(), replan_from_practise("p1_2", "p2"));
}

/// The first plan, then an Elicit on tea per crossing: each frontier walks
/// `X` → `pN_1` → `pN_2` → a sketch, and each answer re-anchors on the beat
/// just entered.
fn chain(anchor: &str, prefix: &str) -> Value {
    let (a, b, c) = (format!("{prefix}_1"), format!("{prefix}_2"), format!("{prefix}_3"));
    fragment(anchor, vec![
        to(on(beat(anchor, "elicit", 0), "tea"), &a),
        to(on(beat(&a, "elicit", 1), "tea"), &b),
        to(on(beat(&b, "elicit", 2), "tea"), &c),
        on(sketch(beat(&c, "elicit", 3)), "tea"),
    ])
}

fn chained_run(crossings: usize) -> Scripted {
    let doc = tea_stall_ending(json!({ "max_turns": 200, "max_beats": 64 }));
    let mut beats = vec!["p1_0".to_owned(), "p1_1".to_owned()];
    let mut plans = vec![planned(fragment("p1_0", vec![
        to(beat("p1_0", "open", 0), "p1_1"),
        to(on(beat("p1_1", "elicit", 1), "tea"), "p1_2"),
        to(on(beat("p1_2", "elicit", 2), "tea"), "p1_3"),
        on(sketch(beat("p1_3", "elicit", 3)), "tea"),
    ]))];
    for k in 1..crossings {
        plans.push(planned(chain(&beats[k], &format!("p{}", k + 1))));
        beats.push(format!("p{}_1", k + 1));
    }
    let turns = (0..crossings).map(|k| crossing(&format!("{}_x0", beats[k]), &beats[k + 1])).collect();
    scripted(&doc, session(), plans, turns)
}

// 3.11.26j AC3
#[test]
fn crossings_are_remembered_not_pruned() {
    let run = chained_run(17);
    for _ in 0..17 {
        run.say();
    }
    let progress = run.procedural().progress;
    assert_eq!(progress.beats, 17);
    assert_eq!(progress.visited.len(), 16, "the last 16, oldest first");
    assert!(progress.visited.iter().all(|v| v.atom == AtomKind::Elicit && v.topic.as_deref() == Some("tea")), "the first beat, the Open, fell off");

    // A Practise loop is a crossing inside one beat: no beat, no visit, no plan.
    let looped = scripted(&tea_stall_ending(json!({ "max_turns": 20, "max_beats": 6 })), session(), vec![planned(first_plan()), kept("unavailable"), kept("unavailable")], vec![
        crossing("p1_0_x0", "p1_1"),
        crossing("p1_1_x0", "p1_2"),
        vec![],
        crossing("p1_2_try", "p1_2_again"),
    ]);
    (0..3).for_each(|_| drop(looped.say()));
    assert_eq!(looped.procedural().progress.since_crossing, 1, "the third turn crossed nothing");
    looped.say();
    let progress = looped.procedural().progress;
    assert_eq!((progress.beats, progress.visited.len(), progress.since_crossing), (2, 2, 0), "the loop reset the stall count alone");
    assert_eq!(looped.plans().len(), 3, "start, crossed, exhausted: the loop asked nothing");
    assert_eq!(looped.frontier(), first_plan(), "a crossing never prunes the frontier; only an answer replaces it");
}

/// Termination without a success goal and without `on_limit`, so the limits
/// end on the default `abandoned` and the stall rule on `walked_off`.
fn bounded(max_turns: u16, max_beats: u16, stall: u8) -> Value {
    tea_stall_ending(json!({ "max_turns": max_turns, "max_beats": max_beats, "stall": { "turns": stall, "ending": "walked_off" } }))
}

fn seeded(doc: &Value, turns: Vec<Vec<crate::api::TurnFrame>>) -> Scripted {
    scripted(doc, session(), vec![kept("unavailable")], turns)
}

// 3.11.26j AC7
#[test]
fn termination_rules_end_in_order() {
    // An ending edge: the seed's Close, `p1_1_end`, onto the success ending.
    let run = seeded(&tea_stall(), vec![crossing("p1_0_x0", "p1_1"), crossing("p1_1_end", "sold")]);
    run.say();
    assert_eq!(ended(&run.say()), Some(("sold".into(), Some("p1_1_end".into()))));
    // The success goal's predicate, on an idle fact change: no edge.
    let run = seeded(&tea_stall(), vec![]);
    run.embed.dialogue_set_fact(run.d, "paid", FactValue::Boolean(true)).unwrap();
    assert_eq!(ended(&run.quiet()), Some(("sold".into(), None)));
    // `max_beats`, then `max_turns` before the stall when both are reached; `on_limit` defaults to `abandoned`.
    assert_eq!(ended(&seeded(&bounded(20, 1, 6), vec![crossing("p1_0_x0", "p1_1")]).say()), Some(("abandoned".into(), None)));
    let run = seeded(&bounded(3, 6, 3), vec![]);
    (0..2).for_each(|_| drop(run.say()));
    assert_eq!(ended(&run.say()), Some(("abandoned".into(), None)), "max_turns is checked before the stall");
    let run = seeded(&bounded(20, 6, 3), vec![]);
    (0..2).for_each(|_| drop(run.say()));
    assert_eq!(ended(&run.say()), Some(("walked_off".into(), None)));
    stall_restarts_only_on_an_accepted_replan();
    ended_runs_drop_their_answers();
}

/// On a sketch: an accepted replan gives the run new road, a kept one does not.
fn stall_restarts_only_on_an_accepted_replan() {
    for (answer, ends_at) in [(planned(fragment("p1_3", vec![ending(beat("p1_3", "close", 0), "sold")])), 8), (kept("unavailable"), 6)] {
        let turns = vec![crossing("p1_0_x0", "p1_1"), crossing("p1_1_x0", "p1_2"), crossing("p1_2_x0", "p1_3")];
        let run = scripted(&bounded(40, 12, 3), session(), vec![planned(first_plan()), kept("unavailable"), kept("unavailable"), answer], turns);
        (0..2).for_each(|_| drop(run.say()));
        run.hold_plans(true);
        run.say();
        run.hold_plans(false);
        assert_eq!(run.here(), "p1_3", "on the sketch, its plan in flight");
        (0..2).for_each(|_| drop(run.say()));
        run.release_plan();
        run.quiet();
        let ended_at = (6..=8).find(|_| ended(&run.say()).is_some());
        assert_eq!(ended_at, Some(ends_at), "turns 4–5 on the sketch, then the answer");
    }
}

fn ended_runs_drop_their_answers() {
    // Ended at `max_turns` with an answer held: dropped, never rebased.
    let run = scripted(&bounded(2, 12, 6), session(), vec![planned(first_plan()), planned(replan_from_elicit("p1_1", "p2"))], vec![crossing("p1_0_x0", "p1_1")]);
    run.hold_plans(true);
    run.say();
    run.api.hold_turns.store(true, Ordering::SeqCst);
    run.embed.dialogue_say(run.d, "你好").unwrap();
    run.release_plan();
    run.quiet();
    run.api.release_turns.notify_one();
    let events = run.quiet();
    assert!(ended(&events).is_some() && planned_count(&events) == 0, "{events:?}");
    assert_eq!(run.frontier(), first_plan());

    // Ended by success while a plan is in flight: the late answer is dropped, and nothing more is asked.
    let run = scripted(&tea_stall(), session(), vec![planned(first_plan()), planned(replan_from_elicit("p1_1", "p2"))], vec![crossing("p1_0_x0", "p1_1")]);
    run.hold_plans(true);
    run.say();
    run.embed.dialogue_set_fact(run.d, "paid", FactValue::Boolean(true)).unwrap();
    run.release_plan();
    let events = run.quiet();
    assert!(ended(&events).is_some() && planned_count(&events) == 0, "{events:?}");
    assert_eq!(run.plans().len(), 2);
}
