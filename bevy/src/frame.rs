//! The frame (D4, D6): Drain writes each core event raw, Apply maps it to
//! its typed Message and onto the entity it belongs to, and `Link` is how a
//! Send system makes one core call. Nothing here blocks: `drain` and every
//! command return at once.

use std::collections::HashMap;

use bevy::ecs::entity::Entity;
use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::ecs::resource::Resource;
use bevy::ecs::system::{ResMut, SystemParam};
use bevy::ecs::world::World;
use lingara_embed::{Command, CommandOutcome, DialogueId, EmbedError, EmbedEvent, RequestId, SessionId};

use crate::core::{Lingara, closed};
use crate::messages::{
    LingaraError, LingaraEventReceived, LingaraRawEvent, LingaraStatus, LingaraTokenNeeded, WorldContextChanged, WorldContextSent,
};
use crate::plugin::Settings;
use crate::{dialogue, lifecycle, practice};

/// The core's ids, to the entities waiting on them. An id with no live
/// entity is dropped: nothing is waiting, and the raw Message still has it.
#[derive(Resource, Default)]
pub(crate) struct Routes {
    pub sessions: HashMap<SessionId, Entity>,
    pub dialogues: HashMap<DialogueId, Entity>,
    /// NPC turns in flight.
    pub requests: HashMap<RequestId, Entity>,
}

/// This frame's events, from Drain to Apply.
#[derive(Resource, Default)]
pub(crate) struct Drained(pub Vec<EmbedEvent>);

pub(crate) fn drain(world: &mut World) {
    let max = world.resource::<Settings>().config.max_events_per_frame;
    let Some(lingara) = world.get_resource::<Lingara>() else { return };
    let events = lingara.handle().drain(max);
    world.write_message_batch(events.iter().cloned().map(LingaraRawEvent));
    world.resource_mut::<Drained>().0 = events;
}

pub(crate) fn apply(world: &mut World) {
    let events = std::mem::take(&mut world.resource_mut::<Drained>().0);
    for event in events {
        route(world, event);
    }
    if let Some(lingara) = world.get_resource::<Lingara>() {
        let status = lingara.handle().status();
        world.resource_mut::<LingaraStatus>().embed = Some(status);
    }
}

/// `Stream` reaches `LingaraStatus` through `status()`; `Log` is already on
/// the `log` facade, which Bevy's `LogPlugin` prints.
fn route(world: &mut World, event: EmbedEvent) {
    match event {
        EmbedEvent::TokenNeeded { request, reason } => {
            world.write_message(LingaraTokenNeeded { request, reason });
        }
        EmbedEvent::Lingara { event } => {
            world.write_message(LingaraEventReceived(event));
        }
        EmbedEvent::WorldSent { request, event_id, reaction } => {
            world.write_message(WorldContextSent { request, event_id, reaction });
        }
        EmbedEvent::Practice { session, snapshot } => practice::on_snapshot(world, session, snapshot),
        EmbedEvent::NpcTurn { dialogue, request, update } => dialogue::on_turn(world, dialogue, request, update),
        EmbedEvent::Error { error } => on_error(world, error),
        EmbedEvent::Closed { reason } => lifecycle::on_closed(world, reason),
        _ => {}
    }
}

/// A core error lands on the entity whose turn it ends, when it ends one.
fn on_error(world: &mut World, error: EmbedError) {
    let entity = error.request.and_then(|r| world.resource_mut::<Routes>().requests.remove(&r));
    if let (Some(entity), Some(request)) = (entity, error.request) {
        dialogue::end_failed_turn(world, entity, request);
    }
    world.write_message(LingaraError { error, entity });
}

/// One core call from a Send system. A refusal becomes a `LingaraError`
/// with the entity, and `None`.
#[derive(SystemParam)]
pub(crate) struct Link<'w> {
    pub lingara: Option<ResMut<'w, Lingara>>,
    pub routes: ResMut<'w, Routes>,
    errors: MessageWriter<'w, LingaraError>,
}

impl Link<'_> {
    pub fn run(&mut self, command: Command, entity: Option<Entity>) -> Option<CommandOutcome> {
        let result = match &self.lingara {
            Some(lingara) => lingara.run(command),
            None => Err(closed()),
        };
        result.map_err(|error| self.fail(error, entity)).ok()
    }

    pub fn fail(&mut self, error: EmbedError, entity: Option<Entity>) {
        self.errors.write(LingaraError { error, entity });
    }

    pub fn is_live(&self) -> bool {
        self.lingara.as_ref().is_some_and(|l| l.is_live())
    }
}

/// `WorldContextChanged` → `send_context`, the context unchanged.
pub(crate) fn send_world(mut messages: MessageReader<WorldContextChanged>, mut link: Link) {
    for WorldContextChanged(context) in messages.read() {
        link.run(Command::SendContext { context: context.clone() }, None);
    }
}
