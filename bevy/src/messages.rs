//! D6's Messages and `LingaraStatus`. Every payload is `lingara-embed`'s own
//! type where it has one: the plugin adds names, never shapes.

use bevy::ecs::entity::Entity;
use bevy::ecs::message::Message;
use bevy::ecs::resource::Resource;
use lingara_embed::{
    CloseReason, EmbedError, EmbedEvent, EmbedStatus, Header, LingaraEvent, PracticeSnapshot, PracticeSource, Reaction, RequestId,
    Secret, TokenReason, WorldContext,
};

use crate::practice::PracticeAction;

// ── Out: what the plugin writes for the game ──

/// Every `EmbedEvent`, unmapped, in queue order, before its typed Message.
/// The escape hatch for a variant this plugin does not map.
#[derive(Message, Debug, Clone, PartialEq)]
pub struct LingaraRawEvent(pub EmbedEvent);

/// The Poll source needs a token: ask your server, then send `ProvideToken`
/// (or `TokenUnavailable`).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LingaraTokenNeeded {
    pub request: RequestId,
    pub reason: TokenReason,
}

/// One event off the stream. Every type the core does not type, `practice.completed`
/// included, arrives as `Other` with its raw `data_json`.
#[derive(Message, Debug, Clone, PartialEq)]
pub struct LingaraEventReceived(pub LingaraEvent);

/// The `202` for a `WorldContextChanged`.
#[derive(Message, Debug, Clone, PartialEq)]
pub struct WorldContextSent {
    pub request: RequestId,
    pub event_id: String,
    pub reaction: Option<Reaction>,
}

/// An NPC finished a reply: the full text, uncut, and the level it was asked at.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct NpcReplied {
    pub entity: Entity,
    pub text: String,
    pub level: u8,
}

/// A practice session reached a terminal phase. Written once per session.
#[derive(Message, Debug, Clone, PartialEq)]
pub struct PracticeFinished {
    pub entity: Entity,
    pub snapshot: PracticeSnapshot,
}

/// Every failure, with the entity it belongs to when the plugin knows it.
/// One from a call made in `LingaraSystems::Send` arrives the next frame.
#[derive(Message, Debug, Clone, PartialEq)]
pub struct LingaraError {
    pub error: EmbedError,
    pub entity: Option<Entity>,
}

// ── In: what the game writes ──

/// Starts the handle when `autostart` is off, or after a `LingaraDisconnect`.
#[derive(Message, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LingaraConnect;

/// Closes the handle; the `Lingara` resource goes once `Closed` is drained.
#[derive(Message, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LingaraDisconnect;

/// The handoff body's two fields, for the Poll source. Private fields, no
/// `Clone`, and a redacting `Debug`: nothing can read the token back out.
#[derive(Message)]
pub struct ProvideToken {
    token: Secret,
    expires_at: String,
}

impl ProvideToken {
    pub fn new(token: impl Into<Secret>, expires_at: impl Into<String>) -> Self {
        Self { token: token.into(), expires_at: expires_at.into() }
    }

    pub(crate) fn parts(&self) -> (Secret, String) {
        (self.token.clone(), self.expires_at.clone())
    }
}

impl std::fmt::Debug for ProvideToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProvideToken").field("token", &self.token).field("expires_at", &self.expires_at).finish()
    }
}

/// Your server cannot give this player a token: every waiting call fails now.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct TokenUnavailable {
    pub reason: String,
}

/// Re-points the Http source, for a player who signs in mid-session. Each
/// header value is a `Secret`, so `Debug` redacts it.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct SetTokenEndpoint {
    pub url: String,
    pub headers: Vec<Header>,
}

/// `world.context_changed`. Each send spends one of the player's inbound
/// cells: send it on a scene change, never from a per-frame system.
#[derive(Message, Debug, Clone, PartialEq)]
pub struct WorldContextChanged(pub WorldContext);

/// Starts a practice session on `entity`, which then holds `PracticeSession`.
#[derive(Message, Debug, Clone, PartialEq)]
pub struct StartPractice {
    pub entity: Entity,
    pub source: PracticeSource,
}

/// One learner action on the session `entity` holds.
#[derive(Message, Debug, Clone, PartialEq)]
pub struct PracticeInput {
    pub entity: Entity,
    pub action: PracticeAction,
}

/// The player says `line` to the NPC on `entity`.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct NpcSay {
    pub entity: Entity,
    pub line: String,
}

/// Drops the reply in flight. The turn is still billed.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct NpcCancel {
    pub entity: Entity,
}

// ── Status ──

/// Where the plugin's handle is. There is no `Reconnecting`: a stream that
/// is retrying reads `Connecting` in `embed.stream`.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum LingaraPhase {
    #[default]
    NotStarted,
    StartFailed(EmbedError),
    Running,
    Closed(CloseReason),
}

/// Read it every frame. `embed` is the core's own status, refreshed once a
/// frame by `LingaraSystems::Apply`; keep its `cursor` and pass it back as
/// `resume_cursor` to resume the stream with no gap after a restart.
#[derive(Resource, Debug, Clone, PartialEq, Default)]
pub struct LingaraStatus {
    pub phase: LingaraPhase,
    pub embed: Option<EmbedStatus>,
}
