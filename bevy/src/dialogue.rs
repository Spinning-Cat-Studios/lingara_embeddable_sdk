//! NPC dialogue (D7): `NpcDialogue` on an entity is a K1 dialogue, and
//! `NpcTranscript` beside it mirrors the turn. One turn is in flight per
//! NPC; a line said while one is refused by K1 with `Busy`. The plugin keeps
//! no queue and never retries: a turn is billed and has no idempotency key.

mod npc;
mod transcript;

use bevy::ecs::entity::Entity;
use bevy::ecs::lifecycle::{Discard, Insert};
use bevy::ecs::message::MessageReader;
use bevy::ecs::observer::On;
use bevy::ecs::query::Changed;
use bevy::ecs::system::{Commands, Query, Res, ResMut};
use bevy::ecs::world::World;
use lingara_embed::{Command, DialogueId, NpcTurnUpdate, RequestId};

pub use npc::NpcDialogue;
pub use transcript::{NpcTranscript, Turn};

use crate::core::{Lingara, id_of};
use crate::frame::{Link, Routes};
use crate::messages::{LingaraError, NpcCancel, NpcReplied, NpcSay};
use crate::plugin::Settings;

/// Opens `entity`'s dialogue on the live handle, or leaves it for the next
/// Connect. `fresh` (an insert) starts a new transcript; a reopen after a
/// Connect keeps the displayed turns.
pub(crate) fn open(world: &mut World, entity: Entity, fresh: bool) {
    let Some(dialogue) = world.get::<NpcDialogue>(entity) else { return };
    let (level, scene) = (dialogue.level(), dialogue.scene().map(str::to_owned));
    let config = dialogue.config(world.resource::<Settings>().config.source_lang.as_deref());
    let opened = match world.get_resource::<Lingara>().filter(|l| l.is_live()) {
        Some(lingara) => lingara.run(Command::DialogueOpen { config }).and_then(id_of).map(Some),
        None => Ok(None),
    };
    match opened {
        Ok(id) => {
            if let Some(id) = id {
                world.resource_mut::<Routes>().dialogues.insert(id, entity);
            }
            let kept = if fresh { None } else { world.entity_mut(entity).take::<NpcTranscript>() };
            world.entity_mut(entity).insert(NpcTranscript::opened(kept, id, level, scene));
        }
        Err(error) => {
            world.entity_mut(entity).remove::<NpcTranscript>();
            world.write_message(LingaraError { error, entity: Some(entity) });
        }
    }
}

/// Every NPC without a live dialogue, opened on a fresh handle.
pub(crate) fn open_pending(world: &mut World) {
    let mut npcs = world.query::<(bevy::ecs::entity::Entity, &NpcDialogue, Option<&NpcTranscript>)>();
    let pending: Vec<Entity> = npcs.iter(world).filter(|(_, _, t)| t.is_none_or(|t| t.id().is_none())).map(|(e, _, _)| e).collect();
    for entity in pending {
        open(world, entity, false);
    }
}

/// Inserting `NpcDialogue`, over an old one too, opens a dialogue.
pub(crate) fn on_insert(event: On<Insert<NpcDialogue>>, mut commands: Commands) {
    let entity = event.event().entity;
    commands.queue(move |world: &mut World| open(world, entity, true));
}

/// Removal, despawn and replacement all close the dialogue first.
pub(crate) fn on_discard(event: On<Discard<NpcDialogue>>, mut transcripts: Query<&mut NpcTranscript>, lingara: Option<Res<Lingara>>, mut routes: ResMut<Routes>) {
    let Ok(mut transcript) = transcripts.get_mut(event.event().entity) else { return };
    let Some(id) = transcript.detach() else { return };
    routes.dialogues.remove(&id);
    if let Some(lingara) = lingara {
        let _ = lingara.run(Command::DialogueClose { dialogue: id });
    }
}

/// A real change of level or scene, forwarded once; the insert frame and a
/// `&mut` access that changed nothing send nothing.
pub(crate) fn forward_changes(mut npcs: Query<(Entity, &NpcDialogue, &mut NpcTranscript), Changed<NpcDialogue>>, mut link: Link) {
    for (entity, dialogue, mut transcript) in &mut npcs {
        let Some(id) = transcript.id() else { continue };
        if dialogue.level() != transcript.sent_level && link.run(Command::DialogueSetLevel { dialogue: id, level: dialogue.level() }, Some(entity)).is_some() {
            transcript.sent_level = dialogue.level();
        }
        let scene = dialogue.scene().map(str::to_owned);
        if scene != transcript.sent_scene && link.run(Command::DialogueSetScene { dialogue: id, scene: scene.clone() }, Some(entity)).is_some() {
            transcript.sent_scene = scene;
        }
    }
}

pub(crate) fn say(mut lines: MessageReader<NpcSay>, mut npcs: Query<(&NpcDialogue, &mut NpcTranscript)>, mut link: Link) {
    for NpcSay { entity, line } in lines.read() {
        let found = npcs.get_mut(*entity).ok().and_then(|(d, t)| t.id().map(|id| (id, d.level(), t)));
        let Some((id, level, mut transcript)) = found else {
            let error = transcript::no_dialogue(link.is_live());
            link.fail(error, Some(*entity));
            continue;
        };
        if let Some(request) = link.run(Command::DialogueSay { dialogue: id, line: line.clone() }, Some(*entity)).and_then(|o| id_of(o).ok()) {
            link.routes.requests.insert(request, *entity);
            transcript.begin(request, line.clone(), level);
        }
    }
}

pub(crate) fn cancel(mut cancels: MessageReader<NpcCancel>, transcripts: Query<&NpcTranscript>, mut link: Link) {
    for NpcCancel { entity } in cancels.read() {
        match transcripts.get(*entity).ok().and_then(NpcTranscript::id) {
            Some(id) => _ = link.run(Command::DialogueCancel { dialogue: id }, Some(*entity)),
            None => {
                let error = transcript::no_dialogue(link.is_live());
                link.fail(error, Some(*entity));
            }
        }
    }
}

/// One K1 `NpcTurn` update onto its transcript.
pub(crate) fn on_turn(world: &mut World, dialogue: DialogueId, request: RequestId, update: NpcTurnUpdate) {
    if matches!(update, NpcTurnUpdate::Finished { .. } | NpcTurnUpdate::Cancelled) {
        world.resource_mut::<Routes>().requests.remove(&request);
    }
    let Some(&entity) = world.resource::<Routes>().dialogues.get(&dialogue) else { return };
    let Some(mut transcript) = world.get_mut::<NpcTranscript>(entity) else { return };
    if let Some((text, level)) = transcript.apply(request, update) {
        world.write_message(NpcReplied { entity, text, level });
    }
}

/// A K1 `Error` that ends `request`'s turn: nothing is added.
pub(crate) fn end_failed_turn(world: &mut World, entity: Entity, request: RequestId) {
    if let Some(mut transcript) = world.get_mut::<NpcTranscript>(entity) {
        transcript.fail(request);
    }
}

/// Every transcript, once the handle is gone: its id dropped, `idle`.
pub(crate) fn drop_all(world: &mut World) {
    let mut transcripts = world.query::<&mut NpcTranscript>();
    for mut transcript in transcripts.iter_mut(world) {
        transcript.detach();
    }
}
