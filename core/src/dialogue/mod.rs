//! The NPC dialogue session (D8): one turn in flight per dialogue, the
//! window kept between turns, and B3's four frames mapped to `NpcTurn`
//! updates. A turn has no idempotency key and each one is billed, so the
//! core never re-sends one: a game that wants a retry says the line again.

pub(crate) mod window;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};
use tokio::task::AbortHandle;

use crate::api::TurnFrame;
use crate::commands::Dispatch;
use crate::dialogue::window::Window;
use crate::embed::Embed;
use crate::error::{EmbedError, ErrorKind};
use crate::limits;
use crate::runtime;
use crate::shared::Shared;
use crate::sync::lock;
use crate::types::{Command, CommandOutcome, DialogueConfig, DialogueId, DialoguePhase, DialogueSnapshot, EmbedEvent, NpcTurnUpdate, RequestId};
use crate::world;

struct Turn {
    request: RequestId,
    line: String,
    abort: Option<AbortHandle>,
}

struct Dialogue {
    config: DialogueConfig,
    window: Window,
    phase: DialoguePhase,
    turn: Option<Turn>,
}

#[derive(Default)]
pub(crate) struct Dialogues {
    map: Mutex<HashMap<DialogueId, Dialogue>>,
}

impl Dialogues {
    fn with<R>(&self, id: DialogueId, f: impl FnOnce(&mut Dialogue) -> Result<R, EmbedError>) -> Result<R, EmbedError> {
        let mut map = lock(&self.map);
        let dialogue = map.get_mut(&id).ok_or_else(|| EmbedError::new(ErrorKind::UnknownId, format!("no dialogue {id}")))?;
        f(dialogue)
    }
}

fn check(config: &DialogueConfig) -> Result<(), EmbedError> {
    world::npc(&config.npc)?;
    if let Some(scene) = &config.scene {
        world::text("scene", scene, limits::SCENE_MAX_CHARS)?;
    }
    world::lang("source_lang", &config.source_lang)?;
    world::lang("target_lang", &config.target_lang)?;
    world::level("level", config.level)
}

/// 1–500 characters after trimming, on one line.
fn check_line(line: &str) -> Result<String, EmbedError> {
    let line = line.trim();
    if line.contains(['\n', '\r']) {
        return Err(EmbedError::invalid("a dialogue line is one line"));
    }
    world::text("line", line, limits::DIALOGUE_LINE_MAX_CHARS)?;
    Ok(line.to_owned())
}

/// B3's request: the config, the line, and the window as `history`.
fn body(d: &Dialogue, line: &str) -> Value {
    let mut body = json!({
        "npc": world::npc_json(&d.config.npc),
        "source_lang": d.config.source_lang,
        "target_lang": d.config.target_lang,
        "level": d.config.level,
        "line": line,
        "history": d.window.entries(),
    });
    if let Some(scene) = &d.config.scene {
        body["scene"] = json!(scene);
    }
    body
}

/// One turn: `Opened` when the headers are in, then the frames in order.
async fn turn(shared: Arc<Shared>, dialogue: DialogueId, request: RequestId, body: Value) {
    let update = |update| EmbedEvent::NpcTurn { dialogue, request, update };
    let mut frames = match shared.api.dialogue_turn(body).await {
        Ok(frames) => frames,
        Err(error) => return end_turn(&shared, dialogue, request, Err(error)).await,
    };
    let current = shared.dialogues.with(dialogue, |d| {
        let current = d.turn.as_ref().is_some_and(|t| t.request == request);
        if current {
            d.phase = DialoguePhase::Streaming;
        }
        Ok(current)
    });
    if current != Ok(true) {
        return;
    }
    shared.outbox.send(update(NpcTurnUpdate::Opened)).await;
    let mut reply = String::new();
    let result = loop {
        match frames.next().await {
            Some(Ok(TurnFrame::Delta(text))) => {
                reply.push_str(&text);
                shared.outbox.send(update(NpcTurnUpdate::Fragment { text })).await;
            }
            Some(Ok(TurnFrame::Notice { code, message })) => shared.outbox.send(update(NpcTurnUpdate::Notice { code, message })).await,
            Some(Ok(TurnFrame::Other)) => {}
            Some(Err(error)) => break Err(error),
            None => break Ok(reply),
        }
    };
    end_turn(&shared, dialogue, request, result).await;
}

/// Back to `Idle`. Only a finished turn moves the window; a failed one
/// leaves it as it was, so saying the line again sends the same history.
async fn end_turn(shared: &Shared, dialogue: DialogueId, request: RequestId, result: Result<String, EmbedError>) {
    let ended = shared.dialogues.with(dialogue, |d| {
        let Some(turn) = d.turn.take_if(|t| t.request == request) else { return Ok(false) };
        d.phase = DialoguePhase::Idle;
        if let Ok(reply) = &result {
            d.window.push_turn(&turn.line, reply);
        }
        Ok(true)
    });
    if ended != Ok(true) {
        return;
    }
    let event = match result {
        Ok(text) => EmbedEvent::NpcTurn { dialogue, request, update: NpcTurnUpdate::Finished { text } },
        Err(error) => EmbedEvent::Error { error: error.with_request(request) },
    };
    shared.outbox.send(event).await;
}

impl Embed {
    pub fn dialogue_open(&self, config: DialogueConfig) -> Result<DialogueId, EmbedError> {
        self.shared.live()?;
        check(&config)?;
        let window = Window::from_supplied(config.window.clone());
        let id = self.shared.ids.next();
        lock(&self.shared.dialogues.map).insert(id, Dialogue { config, window, phase: DialoguePhase::Idle, turn: None });
        Ok(id)
    }

    /// Sends one line; the reply arrives as `NpcTurn` events with the
    /// returned id. Each turn spends one of the player's NPC cells.
    pub fn dialogue_say(&self, dialogue: DialogueId, line: &str) -> Result<RequestId, EmbedError> {
        self.shared.live()?;
        let line = check_line(line)?;
        let request = self.shared.ids.next();
        let body = self.shared.dialogues.with(dialogue, |d| {
            if d.phase != DialoguePhase::Idle {
                return Err(EmbedError::new(ErrorKind::Busy, "a turn is already in flight for this dialogue"));
            }
            d.phase = DialoguePhase::Sending;
            d.turn = Some(Turn { request, line: line.clone(), abort: None });
            Ok(body(d, &line))
        })?;
        let abort = runtime::spawn(&self.shared, turn(Arc::clone(&self.shared), dialogue, request, body));
        let _ = self.shared.dialogues.with(dialogue, |d| {
            d.turn.iter_mut().filter(|t| t.request == request).for_each(|t| t.abort = Some(abort.clone()));
            Ok(())
        });
        Ok(request)
    }

    pub fn dialogue_set_level(&self, dialogue: DialogueId, level: u8) -> Result<(), EmbedError> {
        self.shared.live()?;
        world::level("level", level)?;
        self.shared.dialogues.with(dialogue, |d| {
            d.config.level = level;
            Ok(())
        })
    }

    pub fn dialogue_set_scene(&self, dialogue: DialogueId, scene: Option<String>) -> Result<(), EmbedError> {
        self.shared.live()?;
        if let Some(scene) = &scene {
            world::text("scene", scene, limits::SCENE_MAX_CHARS)?;
        }
        self.shared.dialogues.with(dialogue, |d| {
            d.config.scene = scene;
            Ok(())
        })
    }

    /// Drops the turn in flight, discarding its partial reply.
    pub fn dialogue_cancel(&self, dialogue: DialogueId) -> Result<(), EmbedError> {
        self.shared.live()?;
        let cancelled = self.shared.dialogues.with(dialogue, |d| {
            d.phase = DialoguePhase::Idle;
            Ok(d.turn.take())
        })?;
        if let Some(turn) = cancelled {
            turn.abort.iter().for_each(AbortHandle::abort);
            self.shared.outbox.push_later(EmbedEvent::NpcTurn { dialogue, request: turn.request, update: NpcTurnUpdate::Cancelled });
        }
        Ok(())
    }

    pub fn dialogue_close(&self, dialogue: DialogueId) -> Result<(), EmbedError> {
        self.dialogue_cancel(dialogue)?;
        lock(&self.shared.dialogues.map).remove(&dialogue);
        Ok(())
    }

    /// The window is the game's to keep: pass it to the next `dialogue_open`
    /// to carry a conversation across a restart.
    pub fn dialogue_snapshot(&self, dialogue: DialogueId) -> Option<DialogueSnapshot> {
        let snapshot = |d: &mut Dialogue| {
            let c = &d.config;
            Ok(DialogueSnapshot { phase: d.phase, npc: c.npc.clone(), scene: c.scene.clone(), level: c.level, window: d.window.entries().to_vec() })
        };
        self.shared.dialogues.with(dialogue, snapshot).ok()
    }

    pub(crate) fn exec_dialogue(&self, command: Command) -> Dispatch {
        let done = |r: Result<(), EmbedError>| r.map(|()| CommandOutcome::Done {});
        Ok(match command {
            Command::DialogueOpen { config } => self.dialogue_open(config).map(|id| CommandOutcome::Id { id }),
            Command::DialogueSay { dialogue, line } => self.dialogue_say(dialogue, &line).map(|id| CommandOutcome::Id { id }),
            Command::DialogueSetLevel { dialogue, level } => done(self.dialogue_set_level(dialogue, level)),
            Command::DialogueSetScene { dialogue, scene } => done(self.dialogue_set_scene(dialogue, scene)),
            Command::DialogueCancel { dialogue } => done(self.dialogue_cancel(dialogue)),
            Command::DialogueClose { dialogue } => done(self.dialogue_close(dialogue)),
            Command::DialogueSnapshot { dialogue } => self.shared.live().map(|()| CommandOutcome::Dialogue { snapshot: self.dialogue_snapshot(dialogue) }),
            other => return Err(other),
        })
    }
}
