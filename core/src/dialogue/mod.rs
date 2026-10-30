//! The NPC dialogue session (D8): one turn in flight per dialogue, the
//! window kept between turns, and B3's four frames mapped to `NpcTurn`
//! updates. A turn has no idempotency key and each one is billed, so the
//! core never re-sends one: a game that wants a retry says the line again.
//!
//! A graph dialogue (ADR 28.10.26f, `graph/`) is the same session holding
//! one more thing: where its story is. Its turn sends the checkpoint and the
//! offered edges, holds N2's `transition` until `done`, and crosses it then.
//! `request` builds what a turn sends; `turn` runs one.

pub(crate) mod graph;
mod request;
mod turn;
pub(crate) mod window;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::task::AbortHandle;

use crate::commands::Dispatch;
use crate::dialogue::graph::{GraphState, Pending, Step};
// `check` is also `graph::session`'s: a loaded run's config is held to it.
use crate::dialogue::request::{body, check, check_line};
use crate::dialogue::window::Window;
use crate::embed::Embed;
use crate::error::{EmbedError, ErrorKind};
use crate::limits;
use crate::runtime;
use crate::sync::lock;
use crate::types::{Command, CommandOutcome, DialogueConfig, DialogueId, DialoguePhase, DialogueSnapshot, EmbedEvent, NpcTurnUpdate, RequestId};
use crate::world;

struct Turn {
    request: RequestId,
    line: String,
    abort: Option<AbortHandle>,
    /// The edge ids this turn offered the judge: the only ones it may cross.
    sent: Vec<String>,
}

struct Dialogue {
    config: DialogueConfig,
    window: Window,
    phase: DialoguePhase,
    turn: Option<Turn>,
    graph: Option<GraphState>,
    /// A stored graph still being fetched (ADR 28.10.26i): `Some` exactly while `Loading`.
    pending: Option<Pending>,
}

impl Dialogue {
    fn new(config: DialogueConfig, graph: Option<GraphState>) -> Self {
        let window = Window::from_supplied(config.window.clone());
        let phase = if graph.as_ref().is_some_and(GraphState::ended) { DialoguePhase::Ended } else { DialoguePhase::Idle };
        Self { config, window, phase, turn: None, graph, pending: None }
    }

    fn loading(config: DialogueConfig, pending: Pending) -> Self {
        let window = Window::from_supplied(config.window.clone());
        Self { config, window, phase: DialoguePhase::Loading, turn: None, graph: None, pending: Some(pending) }
    }

    /// At rest, with no turn in flight: crosses whatever edges with no
    /// `when` the facts now open, and ends the dialogue if its story did.
    fn settle(&mut self) -> Vec<Step> {
        let Some(graph) = self.graph.as_mut() else { return Vec::new() };
        let steps = graph.auto_run();
        if graph.ended() {
            self.phase = DialoguePhase::Ended;
        }
        steps
    }
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

/// What a turn command answers while a stored graph is fetched.
fn loading() -> EmbedError {
    EmbedError::new(ErrorKind::InvalidState, "the dialogue is still loading: there is no turn to send or cancel")
}

impl Embed {
    pub fn dialogue_open(&self, config: DialogueConfig) -> Result<DialogueId, EmbedError> {
        self.shared.live()?;
        check(&config)?;
        let id = self.shared.ids.next();
        lock(&self.shared.dialogues.map).insert(id, Dialogue::new(config, None));
        Ok(id)
    }

    /// Sends one line; the reply arrives as `NpcTurn` events with the
    /// returned id. Each turn spends one of the player's NPC cells.
    pub fn dialogue_say(&self, dialogue: DialogueId, line: &str) -> Result<RequestId, EmbedError> {
        self.shared.live()?;
        let line = check_line(line)?;
        let request = self.shared.ids.next();
        let body = self.shared.dialogues.with(dialogue, |d| {
            match d.phase {
                DialoguePhase::Ended => return Err(EmbedError::new(ErrorKind::InvalidState, "the dialogue's story has ended")),
                DialoguePhase::Loading => return Err(loading()),
                _ => {}
            }
            if d.phase != DialoguePhase::Idle {
                return Err(EmbedError::new(ErrorKind::Busy, "a turn is already in flight for this dialogue"));
            }
            let (body, sent) = body(d, &line);
            d.phase = DialoguePhase::Sending;
            d.turn = Some(Turn { request, line: line.clone(), abort: None, sent });
            Ok(body)
        })?;
        let abort = runtime::spawn(&self.shared, turn::turn(Arc::clone(&self.shared), dialogue, request, body));
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

    /// Drops the turn in flight, discarding its partial reply and any
    /// transition it held.
    pub fn dialogue_cancel(&self, dialogue: DialogueId) -> Result<(), EmbedError> {
        self.shared.live()?;
        let (cancelled, steps) = self.shared.dialogues.with(dialogue, |d| {
            if d.phase == DialoguePhase::Loading {
                return Err(loading());
            }
            let Some(turn) = d.turn.take() else { return Ok((None, Vec::new())) };
            d.phase = DialoguePhase::Idle;
            Ok((Some(turn), d.settle()))
        })?;
        if let Some(turn) = cancelled {
            turn.abort.iter().for_each(AbortHandle::abort);
            self.shared.outbox.push_later(EmbedEvent::NpcTurn { dialogue, request: turn.request, update: NpcTurnUpdate::Cancelled });
        }
        graph::emit(&self.shared, dialogue, steps);
        Ok(())
    }

    /// Closes the dialogue. A graph whose story has not ended ends
    /// `abandoned`; one still loading had no story, and emits nothing.
    pub fn dialogue_close(&self, dialogue: DialogueId) -> Result<(), EmbedError> {
        self.shared.live()?;
        {
            let mut map = lock(&self.shared.dialogues.map);
            if map.get(&dialogue).is_some_and(|d| d.phase == DialoguePhase::Loading) {
                map.remove(&dialogue);
                return Ok(());
            }
        }
        self.dialogue_cancel(dialogue)?;
        let removed = lock(&self.shared.dialogues.map).remove(&dialogue);
        if let Some(step) = removed.and_then(|d| d.graph).and_then(|mut graph| graph.abandon()) {
            graph::emit(&self.shared, dialogue, vec![step]);
        }
        Ok(())
    }

    /// The window is the game's to keep: pass it to the next `dialogue_open`
    /// (or, with `graph`, `dialogue_load`) to carry a conversation across a restart.
    pub fn dialogue_snapshot(&self, dialogue: DialogueId) -> Option<DialogueSnapshot> {
        let snapshot = |d: &mut Dialogue| {
            let c = &d.config;
            let graph = d.graph.as_ref().map(GraphState::snapshot);
            Ok(DialogueSnapshot { phase: d.phase, npc: c.npc.clone(), scene: c.scene.clone(), level: c.level, window: d.window.entries().to_vec(), graph })
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
            other => return self.exec_graph(other),
        })
    }
}
