//! `NpcTranscript` (D7): K1's turn, mirrored for the game to draw. Only the
//! plugin makes one. `turns` is for display; what the server receives is
//! K1's window, which the plugin never edits.

use bevy::ecs::component::Component;
use lingara_embed::{DialogueId, DialoguePhase, EmbedError, ErrorKind, NpcTurnUpdate, RequestId};

/// One finished exchange.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Turn {
    pub line: String,
    /// The NPC's reply, uncut.
    pub reply: String,
    /// The level the line was asked at.
    pub level: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InFlight {
    request: RequestId,
    line: String,
    level: u8,
}

#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct NpcTranscript {
    /// K1's `idle`, `sending` or `streaming`.
    pub phase: DialoguePhase,
    /// The reply so far: `Some("")` from `Opened` (the NPC is thinking).
    pub streaming: Option<String>,
    /// `(code, message)`: `truncated`, or a safety replacement. The turn goes on.
    pub notice: Option<(String, String)>,
    /// Every finished turn of the session, until the game clears it.
    pub turns: Vec<Turn>,
    /// A graph NPC's checkpoint: the last `CheckpointReached`, or the one
    /// its story ended at. `None` for a free-form NPC.
    pub checkpoint: Option<String>,
    id: Option<DialogueId>,
    pub(crate) sent_level: u8,
    pub(crate) sent_scene: Option<String>,
    in_flight: Option<InFlight>,
}

impl NpcTranscript {
    /// Opened (or waiting for a Connect, with no id), keeping `kept`'s turns.
    pub(crate) fn opened(kept: Option<Self>, id: Option<DialogueId>, level: u8, scene: Option<String>) -> Self {
        let (turns, checkpoint) = kept.map(|k| (k.turns, k.checkpoint)).unwrap_or_default();
        Self { phase: DialoguePhase::Idle, streaming: None, notice: None, turns, checkpoint, id, sent_level: level, sent_scene: scene, in_flight: None }
    }

    /// The `DialogueId`, for `Lingara::core()`; `None` while no handle is live.
    pub fn id(&self) -> Option<DialogueId> {
        self.id
    }

    /// Drops the id and any turn in flight; returns the id it had.
    pub(crate) fn detach(&mut self) -> Option<DialogueId> {
        self.end();
        self.id.take()
    }

    pub(crate) fn begin(&mut self, request: RequestId, line: String, level: u8) {
        self.phase = DialoguePhase::Sending;
        self.notice = None;
        self.in_flight = Some(InFlight { request, line, level });
    }

    fn end(&mut self) {
        self.phase = DialoguePhase::Idle;
        self.streaming = None;
        self.in_flight = None;
    }

    /// One update of the turn in flight; a finished reply and its level.
    pub(crate) fn apply(&mut self, request: RequestId, update: NpcTurnUpdate) -> Option<(String, u8)> {
        let flight = self.in_flight.as_ref().filter(|f| f.request == request)?;
        match update {
            NpcTurnUpdate::Opened => {
                self.phase = DialoguePhase::Streaming;
                self.streaming = Some(String::new());
            }
            NpcTurnUpdate::Fragment { text } => self.streaming.get_or_insert_default().push_str(&text),
            NpcTurnUpdate::Notice { code, message } => self.notice = Some((code, message)),
            NpcTurnUpdate::Finished { text } => {
                let (line, level) = (flight.line.clone(), flight.level);
                self.turns.push(Turn { line, reply: text.clone(), level });
                self.end();
                return Some((text, level));
            }
            NpcTurnUpdate::Cancelled => self.end(),
            _ => {}
        }
        None
    }

    /// A K1 error for `request`: the turn ends with nothing added.
    pub(crate) fn fail(&mut self, request: RequestId) {
        if self.in_flight.as_ref().is_some_and(|f| f.request == request) {
            self.end();
        }
    }
}

/// An `NpcSay` or `NpcCancel` on an entity with no live dialogue.
pub(crate) fn no_dialogue(live: bool) -> EmbedError {
    match live {
        true => EmbedError::new(ErrorKind::UnknownId, "this entity has no NpcDialogue with a live dialogue"),
        false => crate::core::closed(),
    }
}
