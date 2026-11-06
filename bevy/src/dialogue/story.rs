//! A graph NPC's story events (ADR 28.10.26g D6, 28.10.26i D5, 3.11.26j
//! D10): each K1 event onto the entity's transcript and out as a typed
//! Message. `frame.rs` routes them here through `dialogue::`.

use bevy::ecs::world::World;
use lingara_embed::{CrossedVia, DialogueId, EmbedError, PlanKeptReason, PlanOutcome, TracedBeat};

use super::NpcTranscript;
use crate::frame::Routes;
use crate::messages::{CheckpointReached, DialogueEnded, DialogueLoadFailed, DialogueLoaded, DialoguePlanned};

/// K1's `CheckpointReached`, onto the transcript and out as a Message.
pub(crate) fn on_checkpoint(world: &mut World, dialogue: DialogueId, checkpoint: String, edge: Option<String>, via: CrossedVia) {
    let Some(&entity) = world.resource::<Routes>().dialogues.get(&dialogue) else { return };
    if let Some(mut transcript) = world.get_mut::<NpcTranscript>(entity) {
        transcript.checkpoint = Some(checkpoint.clone());
    }
    world.write_message(CheckpointReached { entity, checkpoint, edge, via });
}

/// K1's `DialogueEnded`: the story is over; the dialogue stays open until
/// the component goes, as a free-form one does. A procedural run's carries
/// its trace.
pub(crate) fn on_ended(world: &mut World, dialogue: DialogueId, outcome: String, checkpoint: String, trace: Option<Vec<TracedBeat>>) {
    let Some(&entity) = world.resource::<Routes>().dialogues.get(&dialogue) else { return };
    if let Some(mut transcript) = world.get_mut::<NpcTranscript>(entity) {
        transcript.checkpoint = Some(checkpoint);
    }
    world.write_message(DialogueEnded { entity, outcome, trace });
}

/// A procedural NPC's plan landed (ADR 3.11.26j D10), out as a Message.
pub(crate) fn on_planned(world: &mut World, dialogue: DialogueId, outcome: PlanOutcome, checkpoints: Vec<String>, reason: Option<PlanKeptReason>) {
    let Some(&entity) = world.resource::<Routes>().dialogues.get(&dialogue) else { return };
    world.write_message(DialoguePlanned { entity, outcome, checkpoints, reason });
}

/// K1's `DialogueLoaded` for a stored dialogue, out as a Message.
pub(crate) fn on_loaded(world: &mut World, dialogue: DialogueId, dialogue_id: String, rev: u64) {
    let Some(&entity) = world.resource::<Routes>().dialogues.get(&dialogue) else { return };
    world.write_message(DialogueLoaded { entity, dialogue_id, rev });
}

/// K1's `DialogueLoadFailed`: the core has dropped the id, so the route goes
/// and the transcript detaches. A later `NpcSay` is then `unknown_id` on the
/// entity, and the next Connect's `open_pending` loads it again.
pub(crate) fn on_load_failed(world: &mut World, dialogue: DialogueId, error: EmbedError) {
    let Some(entity) = world.resource_mut::<Routes>().dialogues.remove(&dialogue) else { return };
    if let Some(mut transcript) = world.get_mut::<NpcTranscript>(entity) {
        transcript.detach();
    }
    world.write_message(DialogueLoadFailed { entity, error });
}
