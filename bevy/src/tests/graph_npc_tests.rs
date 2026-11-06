//! A graph NPC (ADR 28.10.26g D6): AC11.

use bevy::app::App;
use bevy::ecs::entity::Entity;
use lingara_embed::dialogue_ir::build::{AtomKind, Checkpoint, Dialogue, Edge, Fact, Npc, Outcome, Procedural, Termination, Topic};
use lingara_embed::dialogue_ir::procedural::ProceduralDialogue;
use lingara_embed::dialogue_ir::{DialogueIr, Document};
use lingara_embed::{CloseReason, Command, CrossedVia, DialogueSource, EmbedEvent, ErrorKind, FactValue, PlanKeptReason, PlanOutcome, TracedBeat};

use super::fake_core::{Fake, app, error, read, write};
use crate::config::LingaraConfig;
use crate::messages::*;
use crate::{NpcDialogue, NpcTranscript};

/// A two-NPC graph, built with K2's builder.
fn market() -> DialogueIr {
    Dialogue::new("market", "haggle")
        .npc(Npc::new("mei", "Mei").persona("a market seller"))
        .npc(Npc::new("wei", "Old Wei"))
        .fact(Fact::number("trust").default(0))
        .checkpoint(Checkpoint::new("haggle", "Haggle over the price").edge(Edge::new("deal", Outcome::Success).predicate("trust >= 1").when("the player agrees a price")))
        .build()
        .unwrap()
}

fn transcript(app: &App, npc: Entity) -> NpcTranscript {
    app.world().get::<NpcTranscript>(npc).cloned().expect("a transcript")
}

// 28.10.26g AC11
#[test]
fn a_graph_npc_loads_sets_facts_and_hears_its_story() {
    let fake = Fake::default();
    let mut app = app(LingaraConfig::new().source_lang("en"), &fake);
    let ir = market();
    let graph = NpcDialogue::graph(&ir, "mei", "zh", 2).unwrap().with_scene("the night market").unwrap();
    assert_eq!((graph.npc().name.as_str(), graph.npc().persona.as_deref()), ("Mei", Some("a market seller")));
    let npc = app.world_mut().spawn(graph).id();
    let free = app.world_mut().spawn(NpcDialogue::new("Tanaka", "ja", 3).unwrap()).id();
    app.update();

    // Opened with DialogueLoad over the IR's canonical text, never DialogueOpen.
    let loads = fake.sent("dialogue_load");
    let [Command::DialogueLoad { source: DialogueSource::Ir { ir_json }, session }] = loads.as_slice() else { panic!("{loads:?}") };
    assert_eq!(ir_json, &ir.to_canonical_json());
    assert_eq!((session.npc.as_deref(), session.source_lang.as_str(), session.target_lang.as_str(), session.level), (Some("mei"), "en", "zh", 2));
    assert_eq!(session.scene.as_deref(), Some("the night market"));
    assert_eq!(fake.sent("dialogue_open").len(), 1, "the free-form NPC beside it");
    let (id, free_id) = (transcript(&app, npc).id().unwrap(), transcript(&app, free).id().unwrap());

    // NpcSetFact reaches K1 as DialogueSetFact on this NPC's dialogue.
    let one = FactValue::Number(1.into());
    write(&mut app, NpcSetFact { entity: npc, key: "trust".into(), value: one.clone() });
    app.update();
    assert_eq!(fake.sent("dialogue_set_fact"), [Command::DialogueSetFact { dialogue: id, key: "trust".into(), value: one }]);

    // The story's two events, as typed Messages on the right entity only.
    fake.push(EmbedEvent::CheckpointReached { dialogue: id, checkpoint: "haggle".into(), edge: None, via: CrossedVia::Start });
    let ended = EmbedEvent::DialogueEnded { dialogue: id, outcome: "success".into(), checkpoint: "haggle".into(), edge: Some("deal".into()), trace: None };
    fake.push(ended);
    fake.push(EmbedEvent::CheckpointReached { dialogue: 999, checkpoint: "elsewhere".into(), edge: None, via: CrossedVia::Auto });
    app.update();
    let reached = read::<CheckpointReached>(&mut app);
    assert_eq!(reached, [CheckpointReached { entity: npc, checkpoint: "haggle".into(), edge: None, via: CrossedVia::Start }]);
    assert_eq!(read::<DialogueEnded>(&mut app), [DialogueEnded { entity: npc, outcome: "success".into(), trace: None }]);
    assert_eq!(transcript(&app, npc).checkpoint.as_deref(), Some("haggle"));
    assert_eq!(transcript(&app, free).checkpoint, None);

    assert!(free_id != id);
    a_fact_without_a_dialogue_fails_and_removal_closes(&mut app, &fake, npc, id);

    // A cast member the IR does not have is refused at construction.
    assert_eq!(NpcDialogue::graph(&ir, "elena", "zh", 2).unwrap_err().kind, ErrorKind::InvalidInput);
    assert_eq!(NpcDialogue::graph(&ir, "mei", "zh", 10).unwrap_err().kind, ErrorKind::InvalidInput);
}

const STORE_ID: &str = "0b9f6c1e-3c1d-4f7a-9a51-2d1e8b7c6a40";

// 3.11.26f AC5
/// A stored NPC opens `DialogueLoad { Stored }` as cast member `mei`; a blank
/// store id and an over-long cast id are refused at construction; `npc()`
/// names the cast id until `with_display_name`, before and after the load.
#[test]
fn a_stored_npc_opens_by_id() {
    let fake = Fake::default();
    let mut app = app(LingaraConfig::new().source_lang("zh"), &fake);
    let stored = NpcDialogue::stored(STORE_ID, "mei", "en", 3).unwrap();
    assert_eq!(stored.npc().name, "mei");
    let npc = app.world_mut().spawn(stored).id();
    app.update();
    let loads = fake.sent("dialogue_load");
    let [Command::DialogueLoad { source: DialogueSource::Stored { dialogue_id }, session }] = loads.as_slice() else { panic!("{loads:?}") };
    assert_eq!(dialogue_id, STORE_ID);
    assert_eq!((session.npc.as_deref(), session.source_lang.as_str(), session.target_lang.as_str(), session.level), (Some("mei"), "zh", "en", 3));

    let id = transcript(&app, npc).id().unwrap();
    fake.push(EmbedEvent::DialogueLoaded { dialogue: id, dialogue_id: STORE_ID.into(), rev: 4 });
    app.update();
    let dialogue = app.world().get::<NpcDialogue>(npc).unwrap();
    assert_eq!(dialogue.npc().name, "mei", "DialogueLoaded carries no name");

    let named = NpcDialogue::stored(STORE_ID, "mei", "en", 3).unwrap().with_display_name("Mei").unwrap();
    assert_eq!(named.npc().name, "Mei");
    for (store, cast) in [("  ", "mei"), (STORE_ID, ""), (STORE_ID, &"m".repeat(49))] {
        assert_eq!(NpcDialogue::stored(store, cast, "en", 3).unwrap_err().kind, ErrorKind::InvalidInput, "{store:?} {cast:?}");
    }
    assert!(NpcDialogue::stored(STORE_ID, "m".repeat(48), "en", 3).is_ok(), "48 characters is the IR's limit");
    assert_eq!(NpcDialogue::stored(STORE_ID, "mei", "en", 10).unwrap_err().kind, ErrorKind::InvalidInput);
}

// 3.11.26f AC6
/// `DialogueLoaded` and `DialogueLoadFailed` reach the entity typed and still
/// raw; a failure drops the route, so a line is `unknown_id` on the entity,
/// and the next Connect loads it again.
#[test]
fn load_events_reach_the_entity() {
    let fake = Fake::default();
    let mut app = app(LingaraConfig::new().source_lang("zh"), &fake);
    let npc = app.world_mut().spawn(NpcDialogue::stored(STORE_ID, "mei", "en", 3).unwrap()).id();
    app.update();
    let id = transcript(&app, npc).id().unwrap();
    read::<LingaraRawEvent>(&mut app);

    let loaded = EmbedEvent::DialogueLoaded { dialogue: id, dialogue_id: STORE_ID.into(), rev: 2 };
    fake.push(loaded.clone());
    app.update();
    assert_eq!(read::<DialogueLoaded>(&mut app), [DialogueLoaded { entity: npc, dialogue_id: STORE_ID.into(), rev: 2 }]);
    assert!(read::<LingaraRawEvent>(&mut app).contains(&LingaraRawEvent(loaded)));

    let failed = EmbedEvent::DialogueLoadFailed { dialogue: id, error: error(ErrorKind::NotFound) };
    fake.push(failed.clone());
    app.update();
    let failures = read::<DialogueLoadFailed>(&mut app);
    assert_eq!(failures.iter().map(|f| (f.entity, f.error.kind)).collect::<Vec<_>>(), [(npc, ErrorKind::NotFound)]);
    assert!(read::<LingaraRawEvent>(&mut app).contains(&LingaraRawEvent(failed)));
    assert_eq!(transcript(&app, npc).id(), None, "the route and the id are gone");
    assert!(app.world().get::<NpcDialogue>(npc).is_some(), "the component stays");

    read::<LingaraError>(&mut app);
    write(&mut app, NpcSay { entity: npc, line: "hello".into() });
    app.update();
    let errors = read::<LingaraError>(&mut app);
    assert_eq!(errors.iter().map(|e| (e.error.kind, e.entity)).collect::<Vec<_>>(), [(ErrorKind::UnknownId, Some(npc))]);
    assert!(fake.sent("dialogue_say").is_empty(), "no DialogueSay to a dead id");

    write(&mut app, LingaraDisconnect);
    app.update();
    fake.push(EmbedEvent::Closed { reason: CloseReason::Requested });
    app.update();
    write(&mut app, LingaraConnect);
    app.update();
    let loads = fake.sent("dialogue_load");
    assert_eq!(loads.len(), 2, "the next Connect loads it again: {loads:?}");
    assert!(matches!(&loads[1], Command::DialogueLoad { source: DialogueSource::Stored { dialogue_id }, .. } if dialogue_id == STORE_ID));
}

/// A fact for an entity with no dialogue is an error on that entity; removing
/// the graph NPC closes its dialogue, as a free-form NPC's does.
fn a_fact_without_a_dialogue_fails_and_removal_closes(app: &mut App, fake: &Fake, npc: Entity, id: u64) {
    let nobody = app.world_mut().spawn_empty().id();
    write(app, NpcSetFact { entity: nobody, key: "trust".into(), value: FactValue::Number(1.into()) });
    app.update();
    let errors = read::<LingaraError>(app);
    assert_eq!(errors.iter().map(|e| (e.error.kind, e.entity)).collect::<Vec<_>>(), [(ErrorKind::UnknownId, Some(nobody))]);

    app.world_mut().entity_mut(npc).remove::<NpcDialogue>();
    app.update();
    assert_eq!(fake.sent("dialogue_close"), [Command::DialogueClose { dialogue: id }]);
}

/// A small procedural tea stall, built with the Rust procedural builder.
fn tea_stall() -> ProceduralDialogue {
    Procedural::new("tea_stall", "The player wants a cup of tea")
        .npc(Npc::new("mei", "Mei").persona("a tea seller").disposition(0.5, 0.7))
        .ending("sold", "")
        .topic(Topic::new("tea", "the tea on offer"))
        .atoms([AtomKind::Open, AtomKind::Elicit, AtomKind::Close])
        .termination(Termination::new(20, 6))
        .build()
        .unwrap()
}

// 3.11.26j AC15
#[test]
fn a_procedural_npc_on_an_entity() {
    let fake = Fake::default();
    let mut app = app(LingaraConfig::new().source_lang("en"), &fake);
    let doc = tea_stall();
    let npc = app.world_mut().spawn(NpcDialogue::procedural(&doc, "mei", "zh", 2).unwrap()).id();
    app.update();
    let loads = fake.sent("dialogue_load");
    let [Command::DialogueLoad { source: DialogueSource::Ir { ir_json }, session }] = loads.as_slice() else { panic!("{loads:?}") };
    assert_eq!(ir_json, &Document::Procedural(doc.clone()).to_canonical_json(), "the document's canonical text");
    assert_eq!(session.npc.as_deref(), Some("mei"));
    assert!(NpcDialogue::procedural(&doc, "wei", "zh", 2).is_err(), "a cast member the document lacks");

    let id = transcript(&app, npc).id().unwrap();
    let planned = EmbedEvent::DialoguePlanned { dialogue: id, outcome: PlanOutcome::Kept, checkpoints: vec![], reason: Some(PlanKeptReason::Timeout) };
    fake.push(planned);
    let entry = TracedBeat {
        id: "p1_0".into(),
        atom: Some(AtomKind::Open),
        topic: None,
        goal: None,
        ending: None,
        left_by_edge: None,
        via: CrossedVia::Start,
        entered_turn: 0,
        left_turn: Some(6),
        plan_seq: 1,
    };
    fake.push(EmbedEvent::DialogueEnded { dialogue: id, outcome: "abandoned".into(), checkpoint: "p1_0".into(), edge: None, trace: Some(vec![entry.clone()]) });
    app.update();
    let want = DialoguePlanned { entity: npc, outcome: PlanOutcome::Kept, checkpoints: vec![], reason: Some(PlanKeptReason::Timeout) };
    assert_eq!(read::<DialoguePlanned>(&mut app), [want]);
    assert_eq!(read::<DialogueEnded>(&mut app), [DialogueEnded { entity: npc, outcome: "abandoned".into(), trace: Some(vec![entry]) }]);
}

// 3.11.26g AC19
#[test]
fn named_ending_reaches_the_entity() {
    let fake = Fake::default();
    let mut app = app(LingaraConfig::new().source_lang("en"), &fake);
    let npc = app.world_mut().spawn(NpcDialogue::graph(&market(), "mei", "zh", 2).unwrap()).id();
    app.update();
    let id = transcript(&app, npc).id().unwrap();
    fake.push(EmbedEvent::DialogueEnded { dialogue: id, outcome: "fled".into(), checkpoint: "haggle".into(), edge: Some("bolt".into()), trace: None });
    app.update();
    assert_eq!(read::<DialogueEnded>(&mut app), [DialogueEnded { entity: npc, outcome: "fled".into(), trace: None }]);
}
