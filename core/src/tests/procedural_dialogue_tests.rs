//! A procedural run end to end (ADR 3.11.26j): the first frontier, when the
//! core plans, what a plan sends, how an answer is applied, saving and
//! restoring, and the trace. The tests about what reaches the wire run over
//! `fake_lingara` and the real library; the ones about the run's state
//! machine over `fake_api`'s scripted turns and plans.

use std::sync::Arc;

use serde_json::{Value, json};

use crate::embed::Embed;
use crate::error::ErrorKind;
use crate::tests::fake_api::FakeApi;
use crate::tests::fake_lingara::{FakeLingara, PLAN, Reply, dropped, kept as kept_reply, planned as planned_reply, refused};
use crate::tests::procedural_fixture::*;
use crate::tests::{SECS, collect, config, started, unpaused, wait_for};
use crate::types::{CrossedVia, DialogueId, EmbedEvent, PlanKeptReason, PlanOutcome};

fn is_planned(dialogue: DialogueId) -> impl Fn(&EmbedEvent) -> bool {
    move |e| matches!(e, EmbedEvent::DialoguePlanned { dialogue: d, .. } if *d == dialogue)
}

fn reached(dialogue: DialogueId, at: &'static str) -> impl Fn(&EmbedEvent) -> bool {
    move |e| matches!(e, EmbedEvent::CheckpointReached { dialogue: d, checkpoint, .. } if *d == dialogue && checkpoint == at)
}

fn plan_bodies(fake: &FakeLingara) -> Vec<Value> {
    fake.seen().into_iter().filter(|s| s.is("POST", PLAN)).map(|s| s.body).collect()
}

/// A run over `fake_lingara` whose first plan is answered by `first`; turns
/// finish with no transition.
fn over_the_wire(first: Reply) -> (FakeLingara, Embed, DialogueId) {
    let first = std::sync::Mutex::new(Some(first));
    let fake = FakeLingara::start(move |seen, _| {
        if seen.path == PLAN {
            return first.lock().unwrap().take().unwrap_or_else(|| kept_reply("unavailable"));
        }
        Reply::Sse(vec![crate::tests::fake_lingara::frame("done", json!({}))], crate::tests::fake_lingara::After::End)
    });
    let embed = started(config(&fake.url));
    let d = embed.dialogue_load(source(&tea_stall()), session()).unwrap();
    (fake, embed, d)
}

// 3.11.26j AC2
#[test]
fn the_first_frontier_is_planned_or_seeded() {
    let (fake, embed, d) = over_the_wire(planned_reply(&first_plan()));
    assert_eq!(embed.dialogue_snapshot(d).unwrap().phase, crate::types::DialoguePhase::Loading, "a procedural load waits for its first plan");
    let (event, _) = wait_for(&embed, 5 * SECS, is_planned(d));
    assert!(matches!(&event, EmbedEvent::DialoguePlanned { outcome: PlanOutcome::Accepted, checkpoints, reason: None, .. } if checkpoints.contains(&"p1_2_again".to_owned())));
    let (start, between) = wait_for(&embed, 5 * SECS, reached(d, "p1_0"));
    assert!(matches!(start, EmbedEvent::CheckpointReached { via: CrossedVia::Start, edge: None, .. }));
    assert!(!between.iter().any(|e| matches!(e, EmbedEvent::Error { .. })));
    let body = &plan_bodies(&fake)[0];
    assert_eq!((body["reason"].as_str(), body["id_prefix"].as_str(), body["npc"].as_str()), (Some("start"), Some("p1"), Some("keeper")));
    assert!(body.get("current").is_none() && body.get("frontier").is_none(), "{body}");

    // Every failure starts it on the seed, `p1_0` then `p1_1`, and is never an `Error`.
    for (reply, reason) in [
        (kept_reply("unparseable"), PlanKeptReason::Unparseable),
        (refused(503, "unavailable"), PlanKeptReason::Unavailable),
        (refused(404, "not_found"), PlanKeptReason::Unavailable),
        (planned_reply(&fragment("p1_0", vec![beat("p1_0", "close", 0)])), PlanKeptReason::Rejected),
    ] {
        let (_fake, embed, d) = over_the_wire(reply);
        let (event, before) = wait_for(&embed, 5 * SECS, is_planned(d));
        assert!(matches!(&event, EmbedEvent::DialoguePlanned { outcome: PlanOutcome::Kept, checkpoints, reason: Some(r), .. } if checkpoints.is_empty() && *r == reason), "{event:?}");
        let (_, after) = wait_for(&embed, 5 * SECS, reached(d, "p1_0"));
        assert!(!before.iter().chain(&after).any(|e| matches!(e, EmbedEvent::Error { .. })), "a plan never fails the load");
        let graph = embed.dialogue_snapshot(d).unwrap().procedural.unwrap();
        let seeded: Value = serde_json::from_str(&graph.frontier).unwrap();
        assert_eq!(seeded["beats"][1]["id"], "p1_1", "the seed: Open p1_0, then Close p1_1");
    }
}

fn reasons(run: &Scripted) -> Vec<String> {
    run.plans().iter().map(|b| b["reason"].as_str().unwrap_or_default().to_owned()).collect()
}

/// Every `crossed` and `exhausted` body names a beat of the frontier it sends,
/// and the prefixes run `p1`, `p2`, … one per call.
fn bodies_are_well_formed(run: &Scripted) {
    for (n, body) in run.plans().iter().enumerate() {
        assert_eq!(body["id_prefix"], format!("p{}", n + 1));
        if body["reason"] != "start" {
            let current = body["current"].as_str().unwrap();
            assert!(body["frontier"]["beats"].as_array().unwrap().iter().any(|b| b["id"] == current), "{body}");
        }
    }
}

// 3.11.26j AC4
#[test]
fn plans_only_at_start_on_a_crossing_or_exhaustion() {
    let doc = tea_stall_ending(json!({ "max_turns": 20, "max_beats": 6, "stall": { "turns": 6 } }));
    let turns = vec![crossing("p1_0_x0", "p1_1"), vec![], crossing("p1_1_x0", "p1_2")];
    let run = scripted(&doc, session(), vec![planned(first_plan()), kept("unavailable"), kept("unavailable"), kept("unavailable")], turns);
    run.say();
    run.say();
    assert_eq!(reasons(&run), ["start", "crossed"], "a turn with no crossing asks nothing");
    run.say();
    assert_eq!(reasons(&run), ["start", "crossed", "exhausted"], "a crossing onto an exhausted beat asks `exhausted`");
    run.say();
    assert_eq!(run.plans().len(), 3, "once per beat: a kept answer is not re-asked");
    // An idle fact change whose auto-run crosses a beat asks at once: `p1_2_ok` onto the sketch.
    run.embed.dialogue_set_fact(run.d, "paid", lingara_dialogue_ir::FactValue::Boolean(true)).unwrap();
    run.quiet();
    assert_eq!((run.here().as_str(), reasons(&run).last().map(String::as_str)), ("p1_3", Some("exhausted")));
    bodies_are_well_formed(&run);
    a_trigger_in_flight_is_dropped_then_followed_once();
    an_unanswered_run_is_bounded();
}

fn a_trigger_in_flight_is_dropped_then_followed_once() {
    let turns = vec![crossing("p1_0_x0", "p1_1"), crossing("p1_1_x0", "p1_2")];
    let run = scripted(&tea_stall(), session(), vec![planned(first_plan()), planned(replan_from_elicit("p1_1", "p2")), kept("unavailable")], turns);
    run.hold_plans(true);
    run.say();
    run.hold_plans(false);
    run.say();
    assert_eq!(run.plans().len(), 2, "the crossing's trigger was dropped: one call in flight");
    run.release_plan();
    let (event, _) = run.wait(|e| matches!(e, EmbedEvent::DialoguePlanned { .. }));
    assert!(matches!(event, EmbedEvent::DialoguePlanned { outcome: PlanOutcome::Kept, reason: Some(PlanKeptReason::Stale), .. }));
    run.quiet();
    assert_eq!(reasons(&run), ["start", "crossed", "exhausted"], "exactly one follow-up, from the beat the run is on");
    bodies_are_well_formed(&run);
}

/// Against a planner that always keeps, a run ends on the stall rule having
/// made at most `1 + beats` calls.
fn an_unanswered_run_is_bounded() {
    let turns = vec![crossing("p1_0_x0", "p1_1")];
    let run = scripted(&tea_stall_ending(json!({ "max_turns": 20, "max_beats": 6, "stall": { "turns": 3 } })), session(), vec![], turns);
    let ended = (0..5).map(|_| run.say()).find_map(|events| events.into_iter().find(|e| matches!(e, EmbedEvent::DialogueEnded { .. })));
    assert!(ended.is_some(), "the stall rule ends it");
    let progress = run.snapshot().procedural.unwrap().progress;
    assert!(usize::from(progress.plans) <= 1 + usize::from(progress.beats), "{} plans for {} beats", progress.plans, progress.beats);
    assert_eq!(run.plans().len(), usize::from(progress.plans));
}

fn last_planned(events: &[EmbedEvent]) -> Option<(PlanOutcome, Vec<String>, Option<PlanKeptReason>)> {
    events.iter().rev().find_map(|e| match e {
        EmbedEvent::DialoguePlanned { outcome, checkpoints, reason, .. } => Some((*outcome, checkpoints.clone(), *reason)),
        _ => None,
    })
}

/// The first plan, then a Reveal sketch: `p1_2`'s exhausted plan lands on it.
fn to_a_sketch() -> Value {
    fragment("p1_0", vec![
        to(beat("p1_0", "open", 0), "p1_1"),
        to(on(beat("p1_1", "elicit", 1), "tea"), "p1_2"),
        to(on(beat("p1_2", "reveal", 2), "river"), "p1_3"),
        goal(on(sketch(beat("p1_3", "practise", 3)), "tea"), "paid_up"),
    ])
}

// 3.11.26j AC5
#[test]
fn only_a_valid_fresh_fragment_replaces_the_frontier() {
    // Refused here (a topic the keeper may not raise), then a server keep passed through.
    let mut foreign = replan_from_elicit("p1_1", "p2");
    foreign["beats"][1]["topic"] = json!("errands");
    let turns = vec![crossing("p1_0_x0", "p1_1"), crossing("p1_1_x0", "p1_2")];
    let run = scripted(&tea_stall(), session(), vec![planned(first_plan()), planned(foreign), kept("timeout")], turns);
    assert_eq!(last_planned(&run.say()), Some((PlanOutcome::Kept, vec![], Some(PlanKeptReason::Rejected))));
    assert_eq!(run.frontier(), first_plan(), "a refused fragment keeps the frontier");
    assert_eq!(last_planned(&run.say()), Some((PlanOutcome::Kept, vec![], Some(PlanKeptReason::Timeout))), "the server's reason, unchanged");

    // A valid fresh one: replaced whole, rebased onto the checkpoint the run sits on, announced with its ids.
    let run = scripted(&tea_stall(), session(), vec![planned(first_plan()), planned(replan_from_elicit("p1_1", "p2"))], vec![crossing("p1_0_x0", "p1_1")]);
    let (_, ids, _) = last_planned(&run.say()).unwrap();
    assert_eq!(ids, ["p1_1", "p2_1", "p2_2"]);
    assert_eq!((run.here(), run.frontier()), ("p1_1".to_owned(), replan_from_elicit("p1_1", "p2")));
    assert_eq!(run.procedural().progress.since_crossing, 0);

    // Auto-crossed: the replanned Practise has a predicate edge the old sketch had not, and `paid` holds.
    let doc = tea_stall_ending(json!({ "max_turns": 20, "max_beats": 6 }));
    let turns = vec![crossing("p1_0_x0", "p1_1"), crossing("p1_1_x0", "p1_2"), crossing("p1_2_x0", "p1_3")];
    let run = scripted(&doc, session(), vec![planned(to_a_sketch()), kept("unavailable"), kept("unavailable"), planned(replan_from_practise("p1_3", "p4"))], turns);
    run.embed.dialogue_set_fact(run.d, "paid", lingara_dialogue_ir::FactValue::Boolean(true)).unwrap();
    (0..2).for_each(|_| drop(run.say()));
    let events = run.say();
    let auto = events.iter().position(|e| matches!(e, EmbedEvent::CheckpointReached { checkpoint, via: CrossedVia::Auto, .. } if checkpoint == "p4_1"));
    let accepted = events.iter().position(|e| matches!(e, EmbedEvent::DialoguePlanned { outcome: PlanOutcome::Accepted, .. }));
    assert!(matches!((accepted, auto), (Some(a), Some(b)) if a < b), "accepted, then auto-crossed: {events:?}");
}

// 3.11.26j AC8
#[test]
fn a_plan_never_moves_a_streaming_turn() {
    let run = scripted(&tea_stall(), session(), vec![planned(first_plan()), planned(replan_from_elicit("p1_1", "p2"))], vec![crossing("p1_0_x0", "p1_1")]);
    run.hold_plans(true);
    run.say();
    run.api.hold_turns.store(true, std::sync::atomic::Ordering::SeqCst);
    run.embed.dialogue_say(run.d, "你好").unwrap();
    run.quiet();
    run.release_plan();
    run.quiet();
    assert_eq!(run.frontier(), first_plan(), "an answer landing mid-turn is held");
    let sent = run.api.turn_bodies.lock().unwrap().last().cloned().unwrap();
    assert_eq!(sent["edges"][0]["id"], "p1_1_x0", "the turn in flight carries the frontier as it was");
    run.api.release_turns.notify_one();
    let (_, events) = run.wait(|e| matches!(e, EmbedEvent::DialoguePlanned { .. }));
    assert!(events.iter().any(|e| matches!(e, EmbedEvent::NpcTurn { update: crate::types::NpcTurnUpdate::Finished { .. }, .. })), "applied after `done`");
    assert_eq!(run.frontier(), replan_from_elicit("p1_1", "p2"));
}

/// A run over `fake_lingara`: each plan answered by the next of `plans` (then
/// `kept unavailable`), each turn crossing the next of `crossings` (then none).
fn wire(doc: &Value, plans: Vec<Reply>, crossings: Vec<(&'static str, &'static str)>) -> (FakeLingara, Embed, DialogueId) {
    use crate::tests::fake_lingara::{After, frame, transition};
    let plans = std::sync::Mutex::new(std::collections::VecDeque::from(plans));
    let crossings = std::sync::Mutex::new(std::collections::VecDeque::from(crossings));
    let fake = FakeLingara::start(move |seen, _| {
        if seen.path == PLAN {
            return plans.lock().unwrap().pop_front().unwrap_or_else(|| kept_reply("unavailable"));
        }
        let mut frames = vec![frame("delta", json!({ "text": "好" }))];
        frames.extend(crossings.lock().unwrap().pop_front().map(|(edge, to)| transition(edge, to)));
        frames.push(frame("done", json!({})));
        Reply::Sse(frames, After::End)
    });
    let embed = started(config(&fake.url));
    let d = embed.dialogue_load(source(doc), session()).unwrap();
    wait_for(&embed, 5 * SECS, move |e| matches!(e, EmbedEvent::CheckpointReached { dialogue, .. } if *dialogue == d));
    (fake, embed, d)
}

fn say(embed: &Embed, d: DialogueId) -> Vec<EmbedEvent> {
    embed.dialogue_say(d, "你好").unwrap();
    let (_, before) = wait_for(embed, 5 * SECS, move |e| matches!(e, EmbedEvent::NpcTurn { dialogue, update: crate::types::NpcTurnUpdate::Finished { .. }, .. } if *dialogue == d));
    before.into_iter().chain(collect(embed, SECS / 5)).collect()
}

// 3.11.26j AC6
#[test]
fn a_failed_plan_never_fails_the_conversation() {
    let crossings = vec![("p1_0_x0", "p1_1"), ("p1_1_x0", "p1_2"), ("p1_2_x0", "p1_3")];
    for (failure, repeats) in [
        (refused(404, "not_found"), true),
        (refused(403, "embed_needs_metered"), true),
        (refused(400, "invalid_request"), true),
        (refused(422, "safety_input_flagged"), false),
        (refused(429, "rate_limited"), false),
        (refused(503, "unavailable"), false),
        (dropped(), false),
    ] {
        let (fake, embed, d) = wire(&tea_stall(), vec![planned_reply(&first_plan()), failure], crossings.clone());
        let mut events = say(&embed, d);
        let frontier: Value = serde_json::from_str(&embed.dialogue_snapshot(d).unwrap().procedural.unwrap().frontier).unwrap();
        assert_eq!(frontier, first_plan(), "the frontier is kept");
        events.extend(say(&embed, d));
        events.extend(say(&embed, d));
        assert!(!events.iter().any(|e| matches!(e, EmbedEvent::Error { .. })), "never an Error: {events:?}");
        let calls = plan_bodies(&fake).len();
        if repeats {
            assert_eq!(calls, 2, "planning is off for the run, however many crossings follow");
            let off = events.iter().filter(|e| matches!(e, EmbedEvent::Log { message, .. } if message.contains("planning is off"))).count();
            assert_eq!(off, 1, "and it is logged once");
        } else {
            assert_eq!(calls, 4, "never retried, and the next trigger asks again");
        }
    }
}

// 3.11.26j AC9
#[test]
fn no_fact_value_leaves_the_device_in_a_plan() {
    let doc = tea_stall_ending(json!({ "max_turns": 20, "max_beats": 6 }));
    let (fake, embed, d) = wire(&doc, vec![planned_reply(&first_plan())], vec![("p1_0_x0", "p1_1")]);
    embed.dialogue_set_fact(d, "lanterns", lingara_dialogue_ir::FactValue::Number(4242.into())).unwrap();
    embed.dialogue_set_fact(d, "paid", lingara_dialogue_ir::FactValue::Boolean(true)).unwrap();
    say(&embed, d);
    let bodies = plan_bodies(&fake);
    let crossed = &bodies[1];
    let mut keys: Vec<_> = crossed.as_object().unwrap().keys().cloned().collect();
    keys.sort();
    let want = ["beats", "current", "document", "frontier", "goals_met", "history", "id_prefix", "npc", "open_topics", "reason", "turns", "visited"];
    assert_eq!(keys, want, "3.11.26i §2's members, and no `facts`");
    assert_eq!(crossed["open_topics"], json!(["tea", "river"]), "`night` is closed and `errands` is the boy's");
    assert_eq!(crossed["goals_met"], json!(["paid_up"]), "which goals hold, never why");
    assert_eq!((crossed["turns"].as_u64(), crossed["beats"].as_u64()), (Some(1), Some(1)));
    assert_eq!(crossed["visited"], json!([{ "atom": "open" }]));
    assert_eq!(crossed["history"].as_array().map(Vec::len), Some(2), "the window, as the turn sends it");
    let turns: Vec<Value> = fake.seen().into_iter().filter(|s| s.path != PLAN).map(|s| s.body).collect();
    for body in bodies.iter().chain(&turns) {
        assert!(!body.to_string().contains("4242"), "`lanterns` is in no may_disclose: {body}");
    }
}

// 3.11.26j AC22
#[test]
fn a_stored_procedural_document_is_planned() {
    const ID: &str = "3f2a9c1e-7b4d-4e8a-9c0f-1a2b3c4d5e6f";
    let fake = FakeLingara::start(|seen, _| {
        if seen.path == PLAN {
            return planned_reply(&first_plan());
        }
        crate::tests::fake_lingara::exported(&tea_stall(), 3, "\"sha256-tea\"")
    });
    let embed = started(config(&fake.url));
    let d = embed.dialogue_load(crate::types::DialogueSource::Stored { dialogue_id: ID.into() }, session()).unwrap();
    let (_, before) = wait_for(&embed, 5 * SECS, reached(d, "p1_0"));
    let order: Vec<&str> = before.iter().filter_map(|e| match e {
        EmbedEvent::DialogueLoaded { .. } => Some("loaded"),
        EmbedEvent::DialoguePlanned { outcome: PlanOutcome::Accepted, .. } => Some("planned"),
        _ => None,
    }).collect();
    assert_eq!(order, ["loaded", "planned"], "fetched, then planned, then started");
    let seen = fake.seen();
    assert!(seen[0].path.contains(ID) && seen[1].path == PLAN, "{seen:?}");
    assert_eq!(seen[1].body["reason"], "start");
}

// 3.11.26j AC23
#[test]
fn a_hung_plan_times_out_in_the_core() {
    let api = Arc::new(FakeApi::default());
    api.plan_answers.lock().unwrap().push_back(planned(first_plan()));
    *api.plan_delay.lock().unwrap() = Some(std::time::Duration::from_secs(20));
    let embed = crate::tests::paused(config("http://127.0.0.1:9"), &api);
    let d = embed.dialogue_load(source(&tea_stall()), session()).unwrap();
    let (event, _) = wait_for(&embed, 5 * SECS, is_planned(d));
    assert!(matches!(event, EmbedEvent::DialoguePlanned { outcome: PlanOutcome::Kept, reason: Some(PlanKeptReason::Timeout), .. }), "{event:?}");
    wait_for(&embed, 5 * SECS, reached(d, "p1_0"));
    let later = collect(&embed, SECS / 5);
    assert!(!later.iter().any(|e| matches!(e, EmbedEvent::DialoguePlanned { .. })), "the late answer is dropped");
    let seeded: Value = serde_json::from_str(&embed.dialogue_snapshot(d).unwrap().procedural.unwrap().frontier).unwrap();
    assert_eq!(seeded["beats"][1]["atom"], "close", "the seed: Open, then Close");
}

fn invalid(run: &Scripted, doc: &Value, session: crate::types::GraphSession) {
    let err = run.embed.dialogue_load(source(doc), session).unwrap_err();
    assert_eq!(err.kind, ErrorKind::InvalidInput, "{}", err.message);
}

// 3.11.26j AC11
#[test]
fn save_and_restore_a_procedural_run() {
    let saved = scripted(&tea_stall(), session(), vec![planned(first_plan())], vec![crossing("p1_0_x0", "p1_1")]);
    saved.say();
    let snapshot = saved.snapshot();
    let restored = scripted(&tea_stall(), restoring(&snapshot), vec![kept("unavailable")], vec![crossing("p1_1_x0", "p1_2")]);
    restored.quiet();
    assert!(restored.plans().is_empty(), "a beat that is not exhausted asks nothing");
    assert_eq!((restored.here(), restored.frontier()), ("p1_1".to_owned(), first_plan()));
    assert_eq!(restored.procedural().progress, snapshot.procedural.clone().unwrap().progress);
    assert_eq!(restored.snapshot().window, snapshot.window);
    restored.say();
    assert_eq!(restored.plans()[0]["id_prefix"], "p3", "the counters carried: a fresh prefix");
    let again = scripted(&tea_stall(), restoring(&restored.snapshot()), vec![kept("unavailable")], vec![]);
    again.quiet();
    assert_eq!(reasons(&again), ["exhausted"], "restored on an exhausted beat, it asks once");

    // Mid-plan: the old frontier, with `plans` and `next_prefix` counting the lost call.
    let mid = scripted(&tea_stall(), session(), vec![planned(first_plan())], vec![crossing("p1_0_x0", "p1_1")]);
    mid.hold_plans(true);
    mid.say();
    let progress = mid.procedural().progress;
    assert_eq!((progress.plans, progress.next_prefix, mid.frontier()), (2, 3, first_plan()));
    let resumed = scripted(&tea_stall(), restoring(&mid.snapshot()), vec![], vec![]);
    assert!(resumed.plans().is_empty());

    // A frontier anchored on an Elicit, with no `progress`: zero counters, a prefix past its own.
    let bare = crate::types::GraphSession { frontier: Some(replan_from_elicit("p1_1", "p2").to_string()), checkpoint: Some("p1_1".into()), ..session() };
    let bare = scripted(&tea_stall(), bare, vec![kept("unavailable")], vec![crossing("p1_1_x0", "p2_1")]);
    assert_eq!((bare.procedural().progress.beats, bare.procedural().progress.plans), (0, 0));
    bare.say();
    assert_eq!(bare.plans()[0]["id_prefix"], "p3");

    let v1 = json!({ "schema": "dialogue.v1", "id": "plain", "npcs": [{ "id": "keeper", "name": "Mei" }], "start": "a", "checkpoints": [{ "id": "a", "goal": "Talk", "edges": [] }] });
    invalid(&bare, &v1, crate::types::GraphSession { frontier: Some(first_plan().to_string()), checkpoint: Some("a".into()), ..session() });
    invalid(&bare, &tea_stall(), crate::types::GraphSession { progress: Some(Default::default()), ..session() });
    invalid(&bare, &tea_stall(), crate::types::GraphSession { frontier: Some(first_plan().to_string()), ..session() });
    invalid(&bare, &tea_stall(), crate::types::GraphSession { frontier: Some(first_plan().to_string()), checkpoint: Some("p9_9".into()), ..session() });
}

fn trace_of(events: &[EmbedEvent]) -> Option<Vec<crate::types::TracedBeat>> {
    events.iter().find_map(|e| match e {
        EmbedEvent::DialogueEnded { trace, .. } => trace.clone(),
        _ => None,
    })
}

/// `‹id›`: entered `via` at `entered`, left at `left` over `edge`, under frontier `seq`.
fn traced(id: &str, via: CrossedVia, (entered, left): (u16, u16), edge: &str, seq: u16) -> (String, CrossedVia, u16, Option<u16>, Option<String>, u16) {
    (id.into(), via, entered, Some(left), Some(edge.into()), seq)
}

/// The reveal replan anchored on `p1_2`: Reveal → Practise `p3_1` → Close.
fn reveal_then_practise() -> Value {
    fragment("p1_2", vec![
        to(on(beat("p1_2", "reveal", 0), "river"), "p3_1"),
        to(goal(on(beat("p3_1", "practise", 1), "tea"), "paid_up"), "p3_2"),
        ending(beat("p3_2", "close", 2), "sold"),
    ])
}

// 3.11.26j AC26
#[test]
fn the_trace_is_the_crossed_path_across_replans_and_a_restore() {
    let plans = vec![planned(to_a_sketch()), kept("unavailable"), planned(reveal_then_practise()), planned(replan_from_practise("p3_1", "p4"))];
    let turns = vec![
        crossing("p1_0_x0", "p1_1"), crossing("p1_1_x0", "p1_2"), crossing("p1_2_x0", "p3_1"),
        crossing("p3_1_try", "p3_1_again"), crossing("p3_1_retry", "p3_1"), crossing("p3_1_try", "p3_1_again"),
    ];
    let run = scripted(&tea_stall_ending(json!({ "max_turns": 20, "max_beats": 6 })), session(), plans, turns);
    (0..2).for_each(|_| drop(run.say()));
    run.hold_plans(true);
    run.say();
    run.hold_plans(false);
    (0..3).for_each(|_| drop(run.say()));
    run.release_plan();
    run.wait(|e| matches!(e, EmbedEvent::DialoguePlanned { outcome: PlanOutcome::Accepted, .. }));
    assert_eq!(run.here(), "p3_1_again", "the second replan landed on `_again`, and stayed");
    let doc = tea_stall_ending(json!({ "max_turns": 20, "max_beats": 6 }));
    let resumed = scripted(&doc, restoring(&run.snapshot()), vec![], vec![crossing("p3_1_again_x0", "p4_1"), crossing("p4_1_end", "sold")]);
    resumed.say();
    let trace = trace_of(&resumed.say()).expect("a procedural run's end carries its trace");
    let got: Vec<_> = trace.iter().map(|t| (t.id.clone(), t.via, t.entered_turn, t.left_turn, t.left_by_edge.clone(), t.plan_seq)).collect();
    assert_eq!(got, [
        traced("p1_0", CrossedVia::Start, (0, 1), "p1_0_x0", 1),
        traced("p1_1", CrossedVia::Judge, (1, 2), "p1_1_x0", 1),
        traced("p1_2", CrossedVia::Judge, (2, 3), "p1_2_x0", 1),
        traced("p3_1", CrossedVia::Judge, (3, 7), "p3_1_again_x0", 2),
        traced("p4_1", CrossedVia::Judge, (7, 8), "p4_1_end", 3),
    ], "one entry per beat; the Practise loop wrote none");
    assert_eq!((trace[3].atom, trace[3].topic.as_deref(), trace[3].goal.as_deref()), (Some(crate::types::AtomKind::Practise), Some("tea"), Some("paid_up")));
    assert_eq!((trace[4].atom, trace[4].ending.as_deref()), (Some(crate::types::AtomKind::Close), Some("sold")));
    assert_eq!(trace.len(), usize::from(resumed.procedural().progress.beats) + 1);

    let plain = json!({ "schema": "dialogue.v1", "id": "plain", "npcs": [{ "id": "keeper", "name": "Mei" }], "start": "a", "checkpoints": [{ "id": "a", "goal": "Leave", "edges": [{ "id": "go", "to": "success" }] }] });
    let api = Arc::new(FakeApi::default());
    let embed = unpaused(config("http://127.0.0.1:9"), &api);
    embed.dialogue_load(source(&plain), session()).unwrap();
    let (event, _) = wait_for(&embed, 5 * SECS, |e| matches!(e, EmbedEvent::DialogueEnded { .. }));
    assert!(matches!(event, EmbedEvent::DialogueEnded { trace: None, .. }), "a static run has no trace");
}

/// Every string in `value`, recursively.
fn strings(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::String(s) => out.push(s.clone()),
        Value::Array(items) => items.iter().for_each(|v| strings(v, out)),
        Value::Object(map) => map.values().for_each(|v| strings(v, out)),
        _ => {}
    }
}

/// `fragment` with an `angle` on every beat that is not a sketch.
fn angled(mut fragment: Value) -> Value {
    for beat in fragment["beats"].as_array_mut().unwrap() {
        if beat.get("sketch").is_none() {
            beat["angle"] = json!("the price of tea has gone up");
        }
    }
    fragment
}

// 3.11.26j AC27
#[test]
fn the_trace_holds_structure_never_text() {
    let words = |speaker| crate::types::WindowEntry { speaker, text: "a long line about the eel king and his secret".into() };
    let window = (0..8).map(|i| words(if i % 2 == 0 { crate::types::Speaker::Player } else { crate::types::Speaker::Npc })).collect();
    let mut s = crate::types::GraphSession { window, ..session() };
    s.facts.insert("rumour".into(), lingara_dialogue_ir::FactValue::String("a secret the keeper keeps".into()));
    let replan = angled(replan_from_elicit("p1_1", "p2"));
    let run = scripted(&tea_stall(), s, vec![planned(angled(first_plan())), planned(replan.clone())], vec![crossing("p1_0_x0", "p1_1"), crossing("p1_1_x0", "p2_1")]);
    (0..2).for_each(|_| drop(run.say()));
    run.embed.dialogue_close(run.d).unwrap();
    let (event, _) = run.wait(|e| matches!(e, EmbedEvent::DialogueEnded { .. }));
    let EmbedEvent::DialogueEnded { trace: Some(trace), .. } = event else { panic!("traced") };
    let mut allowed = vec!["start".to_owned(), "auto".into(), "judge".into(), "open".into(), "elicit".into(), "reveal".into(), "practise".into(), "close".into()];
    for fragment in [first_plan(), replan] {
        strings(&fragment, &mut allowed);
    }
    let mut found = Vec::new();
    strings(&serde_json::to_value(&trace).unwrap(), &mut found);
    for s in found {
        let edge = s.ends_with("_x0") && allowed.contains(&s.trim_end_matches("_x0").to_owned());
        assert!(!s.contains(' ') && (allowed.contains(&s) || edge), "`{s}` is not an id of the document or its frontiers");
    }
    assert!(run.plans().iter().all(|b| b.get("trace").is_none()), "the planner never reads the trace");
}

fn restore_refused(run: &Scripted, edit: impl Fn(&mut crate::types::ProceduralProgress)) {
    let mut session = restoring(&run.snapshot());
    edit(session.progress.as_mut().unwrap());
    let err = run.embed.dialogue_load(source(&tea_stall()), session).unwrap_err();
    assert_eq!(err.kind, ErrorKind::InvalidInput, "{}", err.message);
}

// 3.11.26j AC28
#[test]
fn the_trace_is_bounded_by_termination() {
    let doc = tea_stall_ending(json!({ "max_turns": 20, "max_beats": 2 }));
    let run = scripted(&doc, session(), vec![planned(first_plan())], vec![crossing("p1_0_x0", "p1_1"), crossing("p1_1_x0", "p1_2")]);
    run.say();
    assert_eq!(trace_of(&run.say()).map(|t| t.len()), Some(3), "max_beats + 1");

    let run = scripted(&tea_stall(), session(), vec![planned(first_plan())], vec![crossing("p1_0_x0", "p1_1")]);
    run.say();
    restore_refused(&run, |p| p.trace.push(p.trace[0].clone()));
    restore_refused(&run, |p| p.beats = 3);
    restore_refused(&run, |p| p.trace[1].left_turn = Some(1));
    restore_refused(&run, |p| p.trace[1].id = "p1_0".into());
    restore_refused(&run, |p| (p.trace[0].plan_seq, p.frontier_seq) = (2, 2));
    restore_refused(&run, |p| p.trace[1].plan_seq = 2);
    let mut long = restoring(&run.snapshot());
    long.progress.as_mut().unwrap().beats = 7;
    let entry = long.progress.as_ref().unwrap().trace[0].clone();
    long.progress.as_mut().unwrap().trace = std::iter::repeat_n(entry, 8).collect();
    assert_eq!(run.embed.dialogue_load(source(&tea_stall_ending(json!({ "max_turns": 20, "max_beats": 6 }))), long).unwrap_err().kind, ErrorKind::InvalidInput, "longer than max_beats + 1");

    // A frontier with no `progress`: one entry on the current beat, `frontier_seq` 1. Restored on
    // the exhausted `p1_2_again`, its accepted replan's next beat is entered under plan_seq 2.
    let bare = crate::types::GraphSession { frontier: Some(first_plan().to_string()), checkpoint: Some("p1_2_again".into()), ..session() };
    let run = scripted(&tea_stall(), bare, vec![planned(replan_from_practise("p1_2", "p2"))], vec![crossing("p1_2_again_x0", "p2_1")]);
    run.wait(|e| matches!(e, EmbedEvent::DialoguePlanned { outcome: PlanOutcome::Accepted, .. }));
    let progress = run.procedural().progress;
    assert_eq!((progress.trace.len(), progress.trace[0].id.as_str(), progress.trace[0].plan_seq), (1, "p1_2", 1));
    run.say();
    let trace = run.procedural().progress.trace;
    assert_eq!(trace.iter().map(|t| t.plan_seq).collect::<Vec<_>>(), [1, 2], "the replan's beat carries plan_seq 2");
}
