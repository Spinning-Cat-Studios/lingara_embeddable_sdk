//! Connect, disconnect, exit and close (D3). A handle lives from a
//! successful start until its one `Closed` is drained. While none is live,
//! inbound Messages are refused, NPCs wait for the next Connect, and
//! practice sessions are detached rather than carried across.

use bevy::app::AppExit;
use bevy::ecs::message::{MessageCursor, MessageReader, Messages};
use bevy::ecs::system::{Local, ResMut};
use bevy::ecs::world::World;
use lingara_embed::CloseReason;

use crate::core::Lingara;
use crate::messages::{LingaraConnect, LingaraDisconnect, LingaraError, LingaraPhase, LingaraStatus};
use crate::plugin::Settings;
use crate::{dialogue, practice};

/// Starts the core. A refused config is `StartFailed` and a `LingaraError`;
/// nothing panics.
pub(crate) fn start(world: &mut World) {
    let settings = world.resource::<Settings>();
    let (config, source, starter) = (settings.config.core_config(), settings.config.token_source.clone(), settings.start.clone());
    match starter(config, source) {
        Ok(core) => {
            world.insert_resource(Lingara::new(core));
            world.resource_mut::<LingaraStatus>().phase = LingaraPhase::Running;
            dialogue::open_pending(world);
        }
        Err(error) => {
            world.resource_mut::<LingaraStatus>().phase = LingaraPhase::StartFailed(error.clone());
            world.write_message(LingaraError { error, entity: None });
        }
    }
}

/// A `LingaraConnect` while a handle exists (live or closing) is ignored.
pub(crate) fn connect(world: &mut World, mut cursor: Local<MessageCursor<LingaraConnect>>) {
    let asked = cursor.read(world.resource::<Messages<LingaraConnect>>()).count() > 0;
    if asked && !world.contains_resource::<Lingara>() {
        start(world);
    }
}

/// `close()` now; the resource goes when Drain delivers `Closed`.
pub(crate) fn disconnect(mut disconnects: MessageReader<LingaraDisconnect>, lingara: Option<ResMut<Lingara>>) {
    if disconnects.read().count() > 0 && let Some(mut lingara) = lingara {
        lingara.close();
    }
}

/// `AppExit` closes the core once, without waiting for it.
pub(crate) fn exit(mut exits: MessageReader<AppExit>, lingara: Option<ResMut<Lingara>>) {
    if exits.read().count() > 0 && let Some(mut lingara) = lingara {
        lingara.close();
    }
}

/// The core's one `Closed`: the last status, then the handle goes, every
/// NPC waits for the next Connect, and every unfinished session detaches.
pub(crate) fn on_closed(world: &mut World, reason: CloseReason) {
    let last = world.remove_resource::<Lingara>().map(|l| l.handle().status());
    let mut status = world.resource_mut::<LingaraStatus>();
    status.phase = LingaraPhase::Closed(reason);
    status.embed = last.or(status.embed.take());
    world.resource_mut::<crate::frame::Routes>().requests.clear();
    world.resource_mut::<crate::frame::Routes>().dialogues.clear();
    dialogue::drop_all(world);
    practice::detach_all(world);
}
