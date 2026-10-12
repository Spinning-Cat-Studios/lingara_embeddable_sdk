//! `LingaraPlugin` (D3, D4): the resources, the Messages, the three system
//! sets and the observers. With `autostart`, `build` starts the core there.

use bevy::app::{App, Plugin, PostUpdate, PreUpdate};
use bevy::ecs::resource::Resource;
use bevy::ecs::schedule::{IntoScheduleConfigs, SystemSet};

use crate::config::LingaraConfig;
use crate::core::{StartFn, start_embed};
use crate::messages::*;
use crate::{dialogue, frame, lifecycle, practice, token};

/// Where the plugin's systems run, so a game can order its own against them.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LingaraSystems {
    /// `PreUpdate`: at most `max_events_per_frame` core events, as `LingaraRawEvent`s.
    Drain,
    /// `PreUpdate`, after Drain: the typed Messages, the components, `LingaraStatus`.
    Apply,
    /// `PostUpdate`: this frame's Messages from the game, to the core.
    Send,
}

/// Adds Lingara to a Bevy app. Every system returns at once: the core does
/// its I/O on its own thread, never on Bevy's.
pub struct LingaraPlugin {
    config: LingaraConfig,
    start: StartFn,
}

impl LingaraPlugin {
    pub fn new(config: LingaraConfig) -> Self {
        Self { config, start: start_embed() }
    }

    /// The tests' scripted core in place of `Embed::start`.
    #[cfg(test)]
    pub(crate) fn with_start(config: LingaraConfig, start: StartFn) -> Self {
        Self { config, start }
    }
}

/// What a `LingaraConnect` starts the core with.
#[derive(Resource)]
pub(crate) struct Settings {
    pub config: LingaraConfig,
    pub start: StartFn,
}

impl Plugin for LingaraPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Settings { config: self.config.clone(), start: self.start.clone() })
            .init_resource::<LingaraStatus>()
            .init_resource::<frame::Routes>()
            .init_resource::<frame::Drained>();
        add_messages(app);
        app.configure_sets(PreUpdate, (LingaraSystems::Drain, LingaraSystems::Apply).chain())
            .configure_sets(PostUpdate, LingaraSystems::Send)
            .add_systems(PreUpdate, frame::drain.in_set(LingaraSystems::Drain))
            .add_systems(PreUpdate, frame::apply.in_set(LingaraSystems::Apply))
            .add_systems(PostUpdate, send_systems().in_set(LingaraSystems::Send))
            .add_observer(dialogue::on_insert)
            .add_observer(dialogue::on_discard)
            .add_observer(practice::on_remove);
        if self.config.autostart {
            lifecycle::start(app.world_mut());
        }
    }
}

/// Connect first, then every inbound Message, then disconnect and exit.
fn send_systems() -> impl IntoScheduleConfigs<bevy::ecs::system::ScheduleSystem, ()> {
    (
        lifecycle::connect,
        token::send,
        frame::send_world,
        practice::start,
        practice::input,
        dialogue::forward_changes,
        dialogue::say,
        dialogue::cancel,
        lifecycle::disconnect,
        lifecycle::exit,
    )
        .chain()
}

fn add_messages(app: &mut App) {
    app.add_message::<LingaraRawEvent>()
        .add_message::<LingaraTokenNeeded>()
        .add_message::<LingaraEventReceived>()
        .add_message::<WorldContextSent>()
        .add_message::<NpcReplied>()
        .add_message::<PracticeFinished>()
        .add_message::<LingaraError>()
        .add_message::<LingaraConnect>()
        .add_message::<LingaraDisconnect>()
        .add_message::<ProvideToken>()
        .add_message::<TokenUnavailable>()
        .add_message::<SetTokenEndpoint>()
        .add_message::<WorldContextChanged>()
        .add_message::<StartPractice>()
        .add_message::<PracticeInput>()
        .add_message::<NpcSay>()
        .add_message::<NpcCancel>();
}
