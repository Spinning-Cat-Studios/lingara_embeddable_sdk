//! A small procedural document and the fragments the tests plan over it
//! (ADR 3.11.26j). The keeper is Railroad (initiative 0.95), so every
//! expansion fills `steer_to`; `lanterns` is in no `may_disclose`, so it must
//! never leave the device; `night` opens only when `warm`; `errands` is the
//! boy's alone.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use serde_json::{Value, json};

use crate::api::{PlanAnswer, TurnFrame};
use crate::embed::Embed;
use crate::error::EmbedError;
use crate::tests::fake_api::FakeApi;
use crate::tests::{SECS, collect, config, unpaused, wait_for};
use crate::types::{DialogueId, DialogueSnapshot, DialogueSource, EmbedEvent, GraphSession, NpcTurnUpdate, ProceduralSnapshot};

/// One scripted plan answer: `None` never answers.
pub type PlanScript = Option<Result<PlanAnswer, EmbedError>>;

/// A run over `fake_api` in real time, its plans and turns scripted in order.
pub struct Scripted {
    pub api: Arc<FakeApi>,
    pub embed: Embed,
    pub d: DialogueId,
}

/// Loads `doc` for `session` and waits for its first `CheckpointReached`.
pub fn scripted(doc: &Value, session: GraphSession, plans: Vec<PlanScript>, turns: Vec<Vec<TurnFrame>>) -> Scripted {
    let api = Arc::new(FakeApi::default());
    *api.plan_answers.lock().unwrap() = plans.into();
    *api.turn_scripts.lock().unwrap() = turns.into();
    let embed = unpaused(config("http://127.0.0.1:9"), &api);
    let d = embed.dialogue_load(source(doc), session).unwrap();
    let run = Scripted { api, embed, d };
    run.wait(|e| matches!(e, EmbedEvent::CheckpointReached { .. }));
    run
}

impl Scripted {
    pub fn wait(&self, matches: impl Fn(&EmbedEvent) -> bool) -> (EmbedEvent, Vec<EmbedEvent>) {
        let d = self.d;
        wait_for(&self.embed, 5 * SECS, move |e| dialogue_of(e) == Some(d) && matches(e))
    }

    /// Says a line and waits for its `Finished`, then for the queue to go
    /// quiet: everything the settle point emitted, and any plan it landed.
    pub fn say(&self) -> Vec<EmbedEvent> {
        self.embed.dialogue_say(self.d, "你好").unwrap();
        let (_, before) = self.wait(|e| matches!(e, EmbedEvent::NpcTurn { update: NpcTurnUpdate::Finished { .. }, .. }));
        before.into_iter().chain(self.quiet()).collect()
    }

    /// Every event for the next 100 ms.
    pub fn quiet(&self) -> Vec<EmbedEvent> {
        collect(&self.embed, SECS / 10)
    }

    pub fn plans(&self) -> Vec<Value> {
        self.api.plan_bodies.lock().unwrap().clone()
    }

    pub fn snapshot(&self) -> DialogueSnapshot {
        self.embed.dialogue_snapshot(self.d).unwrap()
    }

    pub fn procedural(&self) -> ProceduralSnapshot {
        self.snapshot().procedural.expect("a procedural run")
    }

    pub fn here(&self) -> String {
        self.snapshot().graph.expect("a running graph").checkpoint
    }

    pub fn frontier(&self) -> Value {
        serde_json::from_str(&self.procedural().frontier).unwrap()
    }

    pub fn hold_plans(&self, held: bool) {
        self.api.hold_plans.store(held, Ordering::SeqCst);
    }

    pub fn release_plan(&self) {
        self.api.release_plans.notify_one();
    }
}

/// The dialogue an event is about, if any.
pub fn dialogue_of(event: &EmbedEvent) -> Option<DialogueId> {
    match event {
        EmbedEvent::NpcTurn { dialogue, .. }
        | EmbedEvent::CheckpointReached { dialogue, .. }
        | EmbedEvent::DialogueEnded { dialogue, .. }
        | EmbedEvent::DialoguePlanned { dialogue, .. }
        | EmbedEvent::DialogueLoaded { dialogue, .. }
        | EmbedEvent::DialogueLoadFailed { dialogue, .. } => Some(*dialogue),
        _ => None,
    }
}

/// A session restoring `snapshot` at its checkpoint, with its facts and window.
pub fn restoring(snapshot: &DialogueSnapshot) -> GraphSession {
    let graph = snapshot.graph.clone().expect("a graph");
    let procedural = snapshot.procedural.clone().expect("a procedural run");
    GraphSession {
        checkpoint: Some(graph.checkpoint),
        facts: graph.facts,
        window: snapshot.window.clone(),
        frontier: Some(procedural.frontier),
        progress: Some(procedural.progress),
        ..session()
    }
}

pub fn tea_stall() -> Value {
    json!({
        "schema": "dialogue.v2", "kind": "procedural", "id": "tea_stall", "pedagogy": "none",
        "endings": [{ "id": "sold" }, { "id": "walked_off" }],
        "npcs": [
            { "id": "keeper", "name": "Mei", "persona": "a tea seller", "disposition": { "suggestibility": 0.5, "initiative": 0.95 } },
            { "id": "boy", "name": "Bo" }
        ],
        "facts": [
            { "key": "paid", "type": "boolean", "default": false },
            { "key": "warm", "type": "boolean", "default": false },
            { "key": "lanterns", "type": "number", "default": 3 },
            { "key": "rumour", "type": "string", "default": "the river rose" }
        ],
        "procedural": {
            "premise": "The player wants a cup of tea",
            "disclosable": ["rumour"],
            "topics": [
                { "id": "tea", "about": "the tea on offer" },
                { "id": "river", "about": "the river", "discloses": ["rumour"] },
                { "id": "night", "about": "the night market", "when": "warm" },
                { "id": "errands", "about": "the boy's errands", "npcs": ["boy"] }
            ],
            "goals": [{ "id": "paid_up", "describe": "the tea is paid for", "predicate": "paid", "when": "The player pays." }],
            "atoms": [{ "atom": "open" }, { "atom": "elicit" }, { "atom": "reveal" }, { "atom": "practise" }, { "atom": "close" }],
            "horizon": { "plan_depth": 2, "sketch_depth": 1, "max_nodes": 8 },
            "termination": {
                "max_turns": 20, "max_beats": 6,
                "success": { "goal": "paid_up", "ending": "sold" },
                "stall": { "turns": 3, "ending": "walked_off" },
                "on_limit": "walked_off"
            }
        }
    })
}

/// `tea_stall` with its `termination` replaced.
pub fn tea_stall_ending(termination: Value) -> Value {
    let mut doc = tea_stall();
    doc["procedural"]["termination"] = termination;
    doc
}

pub fn source(doc: &Value) -> DialogueSource {
    DialogueSource::Ir { ir_json: doc.to_string() }
}

pub fn session() -> GraphSession {
    GraphSession {
        npc: Some("keeper".into()),
        source_lang: "en".into(),
        target_lang: "zh".into(),
        level: 2,
        scene: None,
        facts: BTreeMap::new(),
        checkpoint: None,
        window: vec![],
        frontier: None,
        progress: None,
    }
}

/// One beat: `(id, atom, depth)`, with its topic, goal, ending, sketch flag
/// and done exit set by the caller.
pub fn beat(id: &str, atom: &str, depth: u8) -> Value {
    json!({ "id": id, "atom": atom, "depth": depth })
}

fn with(mut beat: Value, key: &str, value: Value) -> Value {
    beat[key] = value;
    beat
}

pub fn on(beat: Value, topic: &str) -> Value {
    with(beat, "topic", json!(topic))
}

pub fn to(beat: Value, next: &str) -> Value {
    with(beat, "exits", json!([{ "to": next }]))
}

pub fn sketch(beat: Value) -> Value {
    with(beat, "sketch", json!(true))
}

pub fn goal(beat: Value, goal: &str) -> Value {
    with(beat, "goal", json!(goal))
}

pub fn ending(beat: Value, ending: &str) -> Value {
    with(beat, "ending", json!(ending))
}

pub fn fragment(anchor: &str, beats: Vec<Value>) -> Value {
    json!({ "anchor": anchor, "beats": beats })
}

/// The first plan: Open `p1_0` → Elicit tea `p1_1` → Practise `p1_2` → a
/// Close sketch `p1_3`.
pub fn first_plan() -> Value {
    fragment("p1_0", vec![
        to(beat("p1_0", "open", 0), "p1_1"),
        to(on(beat("p1_1", "elicit", 1), "tea"), "p1_2"),
        to(goal(on(beat("p1_2", "practise", 2), "tea"), "paid_up"), "p1_3"),
        ending(sketch(beat("p1_3", "close", 3)), "sold"),
    ])
}

/// A replan anchored on `anchor` (an Elicit on tea), under `prefix`: Elicit
/// → Reveal river → Close.
pub fn replan_from_elicit(anchor: &str, prefix: &str) -> Value {
    let (reveal, close) = (format!("{prefix}_1"), format!("{prefix}_2"));
    fragment(anchor, vec![
        to(on(beat(anchor, "elicit", 0), "tea"), &reveal),
        to(on(beat(&reveal, "reveal", 1), "river"), &close),
        ending(beat(&close, "close", 2), "sold"),
    ])
}

/// A replan anchored on a Practise beat `anchor`, under `prefix`: Practise →
/// Close.
pub fn replan_from_practise(anchor: &str, prefix: &str) -> Value {
    let close = format!("{prefix}_1");
    fragment(anchor, vec![to(goal(on(beat(anchor, "practise", 0), "tea"), "paid_up"), &close), ending(beat(&close, "close", 1), "sold")])
}

pub fn planned(fragment: Value) -> Option<Result<PlanAnswer, crate::error::EmbedError>> {
    Some(Ok(PlanAnswer::Planned { fragment }))
}

pub fn kept(reason: &str) -> Option<Result<PlanAnswer, crate::error::EmbedError>> {
    Some(Ok(PlanAnswer::Kept { reason: reason.into() }))
}

/// A turn whose judge crosses `edge` to `to`.
pub fn crossing(edge: &str, to: &str) -> Vec<TurnFrame> {
    vec![TurnFrame::Delta("好".into()), TurnFrame::Transition { edge: edge.into(), to: to.into() }]
}
