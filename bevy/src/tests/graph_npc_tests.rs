//! A graph NPC (ADR 28.10.26g D6): AC11.

use bevy::app::App;
use bevy::ecs::entity::Entity;
use lingara_embed::dialogue_ir::DialogueIr;
use lingara_embed::dialogue_ir::build::{Checkpoint, Dialogue, Edge, Fact, Npc, Outcome};
use lingara_embed::{Command, CrossedVia, DialogueOutcome, DialogueSource, EmbedEvent, ErrorKind, FactValue};

use super::fake_core::{Fake, app, read, write};
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
    let ended = EmbedEvent::DialogueEnded { dialogue: id, outcome: DialogueOutcome::Success, checkpoint: "haggle".into(), edge: Some("deal".into()) };
    fake.push(ended);
    fake.push(EmbedEvent::CheckpointReached { dialogue: 999, checkpoint: "elsewhere".into(), edge: None, via: CrossedVia::Auto });
    app.update();
    let reached = read::<CheckpointReached>(&mut app);
    assert_eq!(reached, [CheckpointReached { entity: npc, checkpoint: "haggle".into(), edge: None, via: CrossedVia::Start }]);
    assert_eq!(read::<DialogueEnded>(&mut app), [DialogueEnded { entity: npc, outcome: DialogueOutcome::Success }]);
    assert_eq!(transcript(&app, npc).checkpoint.as_deref(), Some("haggle"));
    assert_eq!(transcript(&app, free).checkpoint, None);

    assert!(free_id != id);
    a_fact_without_a_dialogue_fails_and_removal_closes(&mut app, &fake, npc, id);

    // A cast member the IR does not have is refused at construction.
    assert_eq!(NpcDialogue::graph(&ir, "elena", "zh", 2).unwrap_err().kind, ErrorKind::InvalidInput);
    assert_eq!(NpcDialogue::graph(&ir, "mei", "zh", 10).unwrap_err().kind, ErrorKind::InvalidInput);
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
