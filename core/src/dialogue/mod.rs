//! The NPC dialogue session (D8): one turn in flight per dialogue, the
//! window kept between turns, and B3's four frames mapped to `NpcTurn`
//! updates. A turn has no idempotency key and each one is billed, so the
//! core never re-sends one: a game that wants a retry says the line again.
//!
//! A graph dialogue (ADR 28.10.26f, `graph/`) is the same session holding
//! one more thing: where its story is. Its turn sends the checkpoint and the
//! offered edges, holds N2's `transition` until `done`, and crosses it then.
//! `request` builds what a turn sends; `turn` runs one. A procedural graph
//! (ADR 3.11.26j, `graph/procedural/`) holds its frontier beside the runner
//! and plans at the settle point.

pub(crate) mod graph;
mod request;
mod turn;
pub(crate) mod window;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::task::AbortHandle;

use crate::commands::Dispatch;
use serde_json::Value;

use crate::dialogue::graph::{GraphState, Pending, ProceduralRun, Step};
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
    /// A stored graph still being fetched (ADR 28.10.26i), or a procedural
    /// run waiting for its first plan (ADR 3.11.26j D1): `Some` exactly while `Loading`.
    pending: Option<Pending>,
    /// The rest of a procedural run: its frontier, counters and plan slot.
    procedural: Option<ProceduralRun>,
}

impl Dialogue {
    fn new(config: DialogueConfig, graph: Option<GraphState>) -> Self {
        let window = Window::from_supplied(config.window.clone());
        let phase = if graph.as_ref().is_some_and(GraphState::ended) { DialoguePhase::Ended } else { DialoguePhase::Idle };
        Self { config, window, phase, turn: None, graph, pending: None, procedural: None }
    }

    fn loading(config: DialogueConfig, pending: Pending) -> Self {
        let window = Window::from_supplied(config.window.clone());
        Self { config, window, phase: DialoguePhase::Loading, turn: None, graph: None, pending: Some(pending), procedural: None }
    }

    /// Where the story is now: what [`Dialogue::settle`] counts crossings from.
    fn here(&self) -> String {
        self.graph.as_ref().map(|g| g.current().id.clone()).unwrap_or_default()
    }

    /// At rest, with no turn in flight: after `crossing` (a judged edge, or
    /// none), crosses whatever edges with no `when` the facts now open, and
    /// ends the dialogue if its story did. A procedural run then runs the rest
    /// of ADR 3.11.26j D3's settle point; `finished` says a turn ended. Answers
    /// the steps, and the plan body to send, if any.
    fn settle(&mut self, before: &str, mut steps: Vec<Step>, finished: bool) -> (Vec<Step>, Option<Value>) {
        let Some(graph) = self.graph.as_mut() else { return (steps, None) };
        steps.extend(graph.auto_run());
        let plan = self.procedural.as_mut().and_then(|run| run.settle(graph, before, &mut steps, finished, &self.window));
        if graph.ended() {
            self.phase = DialoguePhase::Ended;
        }
        (steps, plan)
    }

    /// A close of an unfinished story: `abandoned`, with a procedural run's trace.
    fn abandon(mut self) -> Option<Step> {
        let graph = self.graph.as_mut()?;
        match self.procedural.as_mut() {
            Some(run) => run.abandon(graph),
            None => graph.abandon(),
        }
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
        let (cancelled, (steps, plan)) = self.shared.dialogues.with(dialogue, |d| {
            if d.phase == DialoguePhase::Loading {
                return Err(loading());
            }
            let Some(turn) = d.turn.take() else { return Ok((None, (Vec::new(), None))) };
            d.phase = DialoguePhase::Idle;
            let here = d.here();
            Ok((Some(turn), d.settle(&here, Vec::new(), false)))
        })?;
        if let Some(turn) = cancelled {
            turn.abort.iter().for_each(AbortHandle::abort);
            self.shared.outbox.push_later(EmbedEvent::NpcTurn { dialogue, request: turn.request, update: NpcTurnUpdate::Cancelled });
        }
        graph::emit(&self.shared, dialogue, steps);
        plan.into_iter().for_each(|body| graph::procedural::plan::spawn(&self.shared, dialogue, body));
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
        if let Some(step) = removed.and_then(Dialogue::abandon) {
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
            // A run still loading has no frontier of its own yet (ADR 3.11.26j D9).
            let procedural = d.procedural.as_ref().filter(|_| d.graph.is_some()).map(ProceduralRun::snapshot);
            Ok(DialogueSnapshot { phase: d.phase, npc: c.npc.clone(), scene: c.scene.clone(), level: c.level, window: d.window.entries().to_vec(), graph, procedural })
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
