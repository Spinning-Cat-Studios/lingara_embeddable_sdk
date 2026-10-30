//! The graph's three commands (ADR 28.10.26f D6): `dialogue_load`,
//! `dialogue_set_fact` and `dialogue_validate`. A loaded graph is a
//! `Dialogue` like any other, so `say`, `cancel`, `close` and `snapshot`
//! take its id unchanged. A stored source (ADR 28.10.26i) hands the load to
//! `stored`, and the dialogue waits in `loading` until it is fetched.

use std::sync::Arc;

use lingara_dialogue_ir::{DialogueIr, limits};

use crate::commands::Dispatch;
use crate::dialogue::graph::stored::{self, Pending};
use crate::dialogue::graph::{GraphState, TARGET, emit};
use crate::dialogue::{Dialogue, check};
use crate::embed::Embed;
use crate::error::{EmbedError, ErrorKind};
use crate::runtime;
use crate::sync::lock;
use crate::types::{Command, CommandOutcome, DialogueConfig, DialogueId, DialoguePhase, DialogueSource, FactValue, GraphSession, Npc};

fn invalid(error: impl std::fmt::Display) -> EmbedError {
    EmbedError::invalid(error.to_string())
}

/// The cast member a run speaks as: `npc` by id, or the only one.
fn cast_member(ir: &DialogueIr, npc: Option<&str>) -> Result<usize, EmbedError> {
    match npc {
        Some(id) => ir.npcs.iter().position(|n| n.id == id).ok_or_else(|| invalid(format!("the dialogue has no npc `{id}`"))),
        None if ir.npcs.len() == 1 => Ok(0),
        None => Err(invalid(format!("the dialogue's cast has {} npcs: name one", ir.npcs.len()))),
    }
}

/// Where a run begins: a saved checkpoint, else the IR's `start`.
fn start_at(ir: &DialogueIr, checkpoint: Option<&str>) -> Result<usize, EmbedError> {
    let id = checkpoint.unwrap_or(&ir.start);
    if limits::OUTCOMES.contains(&id) {
        return Err(invalid(format!("`{id}` is an outcome, not a checkpoint to resume at")));
    }
    ir.checkpoints.iter().position(|c| c.id == id).ok_or_else(|| invalid(format!("the dialogue has no checkpoint `{id}`")))
}

/// The turn settings a run holds: the cast member's profile and the IR's pedagogy.
fn config(state: &GraphState, session: GraphSession) -> DialogueConfig {
    let npc = state.npc();
    DialogueConfig {
        npc: Npc { name: npc.name.clone(), persona: npc.persona.clone() },
        scene: session.scene,
        source_lang: session.source_lang,
        target_lang: session.target_lang,
        level: session.level,
        window: session.window,
        languages: npc.languages.clone().unwrap_or_default(),
        pedagogy: Some(state.ir().pedagogy),
    }
}

/// A run of a validated `ir` for `session`, its seeded facts and then the
/// `held` ones applied in order, before the first auto-crossing: shared by
/// an inline load and a stored one (ADR 28.10.26i D2).
pub(super) fn open_run(ir: Arc<DialogueIr>, session: GraphSession, held: &[(String, FactValue)]) -> Result<(GraphState, DialogueConfig), EmbedError> {
    let npc = cast_member(&ir, session.npc.as_deref())?;
    let mut state = GraphState::new(Arc::clone(&ir), npc, start_at(&ir, session.checkpoint.as_deref())?);
    for (key, value) in session.facts.iter().chain(held.iter().map(|(k, v)| (k, v))) {
        state.set_fact(key, value.clone()).map_err(invalid)?;
    }
    let config = config(&state, session);
    check(&config)?;
    Ok((state, config))
}

/// What a `loading` dialogue answers until its revision arrives: the
/// session's settings, under the cast id it named.
fn loading_config(session: &GraphSession) -> DialogueConfig {
    DialogueConfig {
        npc: Npc { name: session.npc.clone().unwrap_or_default(), persona: None },
        scene: session.scene.clone(),
        source_lang: session.source_lang.clone(),
        target_lang: session.target_lang.clone(),
        level: session.level,
        window: session.window.clone(),
        languages: Vec::new(),
        pedagogy: None,
    }
}

impl Embed {
    /// Loads a `dialogue.v1` graph and opens a run of it. `CheckpointReached
    /// { via: start }` follows the returned id, then any auto crossings. A
    /// stored source answers its id in `loading`, and `DialogueLoaded` or
    /// `DialogueLoadFailed` follows once it is fetched.
    pub fn dialogue_load(&self, source: DialogueSource, session: GraphSession) -> Result<DialogueId, EmbedError> {
        self.shared.live()?;
        let ir_json = match source {
            DialogueSource::Ir { ir_json } => ir_json,
            DialogueSource::Stored { dialogue_id } => return self.load_stored(&dialogue_id, session),
        };
        let (ir, warnings) = lingara_dialogue_ir::load(&ir_json).map_err(invalid)?;
        let (mut state, config) = open_run(Arc::new(ir), session, &[])?;
        warnings.iter().for_each(|warning| self.shared.diag.warn(TARGET, warning.to_string()));
        let steps = state.start();
        let id = self.shared.ids.next();
        lock(&self.shared.dialogues.map).insert(id, Dialogue::new(config, Some(state)));
        emit(&self.shared, id, steps);
        Ok(id)
    }

    /// A malformed id is refused here and sends nothing; the rest of the
    /// session is checked against the fetched IR (ADR 28.10.26i D2).
    fn load_stored(&self, dialogue_id: &str, session: GraphSession) -> Result<DialogueId, EmbedError> {
        let dialogue_id = stored::store_id(dialogue_id)?;
        let id = self.shared.ids.next();
        let config = loading_config(&session);
        lock(&self.shared.dialogues.map).insert(id, Dialogue::loading(config, Pending { session, held: Vec::new() }));
        runtime::spawn(&self.shared, stored::load(Arc::clone(&self.shared), id, dialogue_id));
        Ok(id)
    }

    /// Sets a fact the graph declared. While no turn is in flight, any edge
    /// with no `when` that it opens is crossed now; during a turn, once the
    /// turn ends.
    pub fn dialogue_set_fact(&self, dialogue: DialogueId, key: &str, value: FactValue) -> Result<(), EmbedError> {
        self.shared.live()?;
        let steps = self.shared.dialogues.with(dialogue, |d| {
            if let Some(pending) = d.pending.as_mut() {
                // Held, and checked against the fetched IR (ADR 28.10.26i D2).
                pending.held.push((key.to_owned(), value));
                return Ok(Vec::new());
            }
            let ended = d.phase == DialoguePhase::Ended;
            let Some(graph) = d.graph.as_mut().filter(|_| !ended) else {
                return Err(EmbedError::new(ErrorKind::InvalidState, "only a graph dialogue whose story has not ended holds facts"));
            };
            graph.set_fact(key, value).map_err(invalid)?;
            Ok(if d.turn.is_none() { d.settle() } else { Vec::new() })
        })?;
        emit(&self.shared, dialogue, steps);
        Ok(())
    }

    /// Checks a `dialogue.v1` document as `dialogue_load` would, creating
    /// nothing: for a builder with no Rust of its own.
    pub fn dialogue_validate(&self, ir_json: &str) -> Result<(), EmbedError> {
        self.shared.live()?;
        lingara_dialogue_ir::load(ir_json).map(|_| ()).map_err(invalid)
    }

    /// The graph's commands, after the free-form ones.
    pub(crate) fn exec_graph(&self, command: Command) -> Dispatch {
        let done = |r: Result<(), EmbedError>| r.map(|()| CommandOutcome::Done {});
        Ok(match command {
            Command::DialogueLoad { source, session } => self.dialogue_load(source, session).map(|id| CommandOutcome::Id { id }),
            Command::DialogueSetFact { dialogue, key, value } => done(self.dialogue_set_fact(dialogue, &key, value)),
            Command::DialogueValidate { ir_json } => done(self.dialogue_validate(&ir_json)),
            other => return Err(other),
        })
    }
}
