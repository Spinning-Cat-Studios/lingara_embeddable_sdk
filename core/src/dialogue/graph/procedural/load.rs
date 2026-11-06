//! Opening a procedural run (ADR 3.11.26j D1, D9): fresh, it waits in
//! `loading` for its first plan, falling back to the seed, and never fails
//! for the planner's sake; restored from a saved `frontier`, it starts at
//! once and asks only if its beat is exhausted. Also where every plan answer
//! lands, under the dialogue map's lock.

use std::collections::BTreeMap;
use std::sync::Arc;

use lingara_dialogue_ir::procedural::{GAME_WORDING, PlanFragment, ProceduralDialogue, expand, seed, within_spec_restored};
use serde_json::Value;

use super::plan::Landed;
use super::{ProceduralRun, Slot, beat_of};
use crate::dialogue::Dialogue;
use crate::dialogue::graph::session::open_run;
use crate::dialogue::graph::{Pending, Step};
use crate::dialogue::window::Window;
use crate::error::EmbedError;
use crate::shared::Shared;
use crate::sync::lock;
use crate::types::{CrossedVia, DialogueId, DialoguePhase, EmbedEvent, FactValue, GraphSession, ProceduralProgress, TracedBeat};

fn invalid(error: impl std::fmt::Display) -> EmbedError {
    EmbedError::invalid(error.to_string())
}

/// What a load opened: the dialogue, the steps it emits at once, and the plan
/// body to send, if any.
pub(in crate::dialogue) struct Opened {
    pub(in crate::dialogue) dialogue: Dialogue,
    pub(in crate::dialogue) steps: Vec<Step>,
    pub(in crate::dialogue) plan: Option<Value>,
}

/// The cast id the run speaks as: the session's, else the only one. A wrong
/// one is refused by `open_run`, which reads the same cast.
fn npc_id(doc: &ProceduralDialogue, session: &GraphSession) -> String {
    session.npc.clone().or_else(|| (doc.npcs.len() == 1).then(|| doc.npcs[0].id.clone())).unwrap_or_default()
}

/// Opens a run of `doc` for `session`, the facts `held` while a stored
/// document was fetched applied after the session's.
pub(in crate::dialogue) fn open(doc: Arc<ProceduralDialogue>, mut session: GraphSession, held: &[(String, FactValue)]) -> Result<Opened, EmbedError> {
    let (frontier, progress) = (session.frontier.take(), session.progress.take());
    let npc = npc_id(&doc, &session);
    match frontier {
        Some(frontier) => restore(doc, npc, session, held, Saved { frontier, progress }),
        None if progress.is_some() => Err(invalid("`progress` restores only with its `frontier`")),
        None if session.checkpoint.is_some() => Err(invalid("a procedural run resumes at a `checkpoint` only with its saved `frontier`")),
        None => fresh(doc, npc, session, held),
    }
}

/// D1: the session is checked over the seed's expansion now, so a bad cast
/// member or fact is refused before anything is sent, and the `start` plan
/// reads its topics and goals over the same facts.
fn fresh(doc: Arc<ProceduralDialogue>, npc: String, session: GraphSession, held: &[(String, FactValue)]) -> Result<Opened, EmbedError> {
    let seeded = seed(&doc, &npc, "p1");
    let ir = expand(&doc, &npc, &seeded).map_err(invalid)?;
    let (state, config) = open_run(Arc::new(ir), session.clone(), held)?;
    let window = Window::from_supplied(session.window.clone());
    let progress = ProceduralProgress { next_prefix: 1, ..ProceduralProgress::default() };
    let mut run = ProceduralRun::new(doc, npc, seeded, progress);
    let plan = run.ask("start", &state, &window);
    let mut dialogue = Dialogue::loading(config, Pending { session, held: held.to_vec() });
    dialogue.procedural = Some(run);
    Ok(Opened { dialogue, steps: Vec::new(), plan: Some(plan) })
}

/// What a game saved from `DialogueSnapshot.procedural`.
struct Saved {
    frontier: String,
    progress: Option<ProceduralProgress>,
}

/// D9: a saved frontier, checked by `within_spec_restored`, at its required
/// checkpoint, with its counters and trace.
fn restore(doc: Arc<ProceduralDialogue>, npc: String, session: GraphSession, held: &[(String, FactValue)], saved: Saved) -> Result<Opened, EmbedError> {
    let Saved { frontier: text, progress } = saved;
    let frontier: PlanFragment = serde_json::from_str(&text).map_err(|e| invalid(format!("the saved frontier does not read as a fragment: {e}")))?;
    within_spec_restored(&doc, &npc, &frontier, &GAME_WORDING).map_err(|e| invalid(format!("the saved frontier is refused: {e}")))?;
    let Some(checkpoint) = session.checkpoint.clone() else { return Err(invalid("a saved `frontier` restores only with its `checkpoint`")) };
    let ir = expand(&doc, &npc, &frontier).map_err(invalid)?;
    if !ir.checkpoints.iter().any(|c| c.id == checkpoint) {
        return Err(invalid(format!("the saved frontier's expansion has no checkpoint `{checkpoint}`")));
    }
    let current = beat_of(&checkpoint).to_owned();
    let progress = match progress {
        Some(saved) => checked(saved, &current, doc.procedural.termination.max_beats)?,
        None => first_progress(&frontier, &current),
    };
    let mut progress = progress;
    progress.next_prefix = progress.next_prefix.max(past_prefixes(&frontier)).max(1);
    let (mut state, config) = open_run(Arc::new(ir), session, held)?;
    let mut run = ProceduralRun::new(doc, npc, frontier, progress);
    let mut dialogue = Dialogue::new(config, None);
    let mut steps = state.start();
    let plan = run.settle(&mut state, &checkpoint, &mut steps, false, &dialogue.window);
    dialogue.phase = if state.ended() { DialoguePhase::Ended } else { DialoguePhase::Idle };
    (dialogue.graph, dialogue.procedural) = (Some(state), Some(run));
    Ok(Opened { dialogue, steps, plan })
}

/// A frontier restored with no `progress`: zero counters, and a one-entry
/// trace on the current beat (D13).
fn first_progress(frontier: &PlanFragment, current: &str) -> ProceduralProgress {
    let beat = frontier.beat(current);
    let entry = TracedBeat {
        id: current.to_owned(),
        atom: beat.map(|b| b.atom),
        topic: beat.and_then(|b| b.topic.clone()),
        goal: beat.and_then(|b| b.goal.clone()),
        ending: beat.and_then(|b| b.ending.clone()),
        left_by_edge: None,
        via: CrossedVia::Start,
        entered_turn: 0,
        left_turn: None,
        plan_seq: 1,
    };
    ProceduralProgress { trace: vec![entry], frontier_seq: 1, ..ProceduralProgress::default() }
}

/// D13's restore check: a trace that is this run's path, closed but for its
/// current beat, within the termination bound.
fn checked(saved: ProceduralProgress, current: &str, max_beats: u16) -> Result<ProceduralProgress, EmbedError> {
    let trace = &saved.trace;
    let refuse = |why: &str| Err(invalid(format!("the saved progress is refused: {why}")));
    if trace.len() != usize::from(saved.beats) + 1 {
        return refuse("its trace does not hold one entry per beat crossed plus the current one");
    }
    if trace.len() > usize::from(max_beats) + 1 {
        return refuse("its trace is longer than max_beats + 1");
    }
    let (last, closed) = trace.split_last().expect("the length was checked");
    if closed.iter().any(|e| e.left_turn.is_none()) || last.left_turn.is_some() || last.id != current {
        return refuse("only its last entry may be open, and it must be the current beat");
    }
    if trace.windows(2).any(|w| w[1].plan_seq < w[0].plan_seq) || trace.iter().any(|e| e.plan_seq > saved.frontier_seq) {
        return refuse("its plan_seq decreases or passes frontier_seq");
    }
    Ok(saved)
}

/// One past the highest `N` of any `pN_` id in `frontier`: a prefix it holds
/// is never sent again, which 3.11.26i refuses with a `400`.
fn past_prefixes(frontier: &PlanFragment) -> u16 {
    let n = |id: &str| id.strip_prefix('p')?.split('_').next()?.parse::<u16>().ok();
    frontier.beats.iter().filter_map(|b| n(&b.id)).max().map_or(1, |high| high.saturating_add(1))
}

/// Where every plan answer lands, under the map's lock: the first one opens
/// the run, any later one goes through D5. `None` when the dialogue is gone.
pub(crate) fn land(shared: &Shared, dialogue: DialogueId, landed: Landed) -> Option<(Vec<Step>, Option<Value>)> {
    let mut map = lock(&shared.dialogues.map);
    let d = map.get_mut(&dialogue)?;
    if d.phase != DialoguePhase::Loading {
        let (run, graph) = (d.procedural.as_mut()?, d.graph.as_mut()?);
        let landed = run.land(graph, landed, d.turn.is_some(), &d.window);
        if graph.ended() {
            d.phase = DialoguePhase::Ended;
        }
        return Some(landed);
    }
    match started(d, landed) {
        Ok(opened) => Some(opened),
        Err(error) => {
            map.remove(&dialogue);
            shared.outbox.push_later(EmbedEvent::DialogueLoadFailed { dialogue, error });
            None
        }
    }
}

/// The first frontier, planned or seeded: the run starts on its anchor with
/// the session's facts, then those set while it loaded. Level and scene set
/// while loading are kept.
fn started(d: &mut Dialogue, landed: Landed) -> Result<(Vec<Step>, Option<Value>), EmbedError> {
    let (Some(mut run), Some(pending)) = (d.procedural.take(), d.pending.take()) else { return Ok((Vec::new(), None)) };
    let Slot::InFlight(asked) = std::mem::replace(&mut run.slot, Slot::Idle) else { return Ok((Vec::new(), None)) };
    let (ir, mut steps) = run.first(asked, landed)?;
    let mut session = pending.session;
    (session.level, session.scene, session.checkpoint) = (d.config.level, d.config.scene.clone(), None);
    session.facts = session.facts.into_iter().chain(pending.held).collect::<BTreeMap<_, _>>();
    let (mut state, config) = open_run(Arc::new(ir), session, &[])?;
    let anchor = state.current().id.clone();
    run.asked_here = true;
    let _ = run.enter(beat_of(&anchor), CrossedVia::Start);
    let mut started = state.start();
    let plan = run.settle(&mut state, &anchor, &mut started, false, &d.window);
    steps.extend(started);
    // No turn runs while loading, so the window is still the session's.
    *d = Dialogue::new(config, None);
    d.phase = if state.ended() { DialoguePhase::Ended } else { DialoguePhase::Idle };
    (d.graph, d.procedural) = (Some(state), Some(run));
    Ok((steps, plan))
}
