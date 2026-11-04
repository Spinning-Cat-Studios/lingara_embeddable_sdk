//! Expansion: a fragment is a static graph (ADR 3.11.26g §7). Pure: over a
//! fragment that passed `within_spec` and a wording that passed `check()` it
//! cannot fail, and the result is an ordinary `dialogue.v2` [`DialogueIr`]
//! the K1 runner and the N2 judge walk unchanged. Also [`seed`] (§6).

use std::collections::BTreeMap;

use super::atoms::{AGAIN, END, EXIT, OK, RETRY, TRY};
use super::wording::fill;
use super::{AtomKind, AtomText, AtomWording, Beat, Exit, GAME_WORDING, PlanFragment, ProceduralDialogue};
use crate::disposition::Stance;
use crate::ir::{Checkpoint, DialogueIr, Edge, SCHEMA_V2};
use crate::IrError;

/// [`expand_with`] under [`GAME_WORDING`].
pub fn expand(doc: &ProceduralDialogue, npc: &str, fragment: &PlanFragment) -> Result<DialogueIr, IrError> {
    expand_with(doc, npc, fragment, &GAME_WORDING)
}

/// The fragment as a static graph: `start` is the anchor, and each beat is
/// one checkpoint, or two for a planned Practise beat. `steer_to` is set when
/// the run NPC's drive is High or Railroad. Its only refusal is the
/// wording's own `atom_wording`.
pub fn expand_with(doc: &ProceduralDialogue, npc: &str, fragment: &PlanFragment, wording: &AtomWording) -> Result<DialogueIr, IrError> {
    wording.check()?;
    let steers = Stance::of(doc.npc(npc).and_then(|n| n.disposition.as_ref())).steers();
    let words = Words { doc, wording, steers };
    Ok(DialogueIr {
        schema: SCHEMA_V2.into(),
        id: doc.id.clone(),
        title: doc.title.clone(),
        pedagogy: doc.pedagogy,
        npcs: doc.npcs.clone(),
        facts: doc.facts.clone(),
        endings: doc.endings.clone(),
        start: fragment.anchor.clone(),
        checkpoints: fragment.beats.iter().flat_map(|beat| words.beat(beat)).collect(),
        extensions: BTreeMap::new(),
    })
}

/// `‹prefix›_0`, an Open whose done exit leads to a Close `‹prefix›_1` on the
/// success rule's ending, or on the first declared ending. It passes
/// `within_spec` with no frontier, so a run always has a frontier to walk.
pub fn seed(doc: &ProceduralDialogue, _npc: &str, prefix: &str) -> PlanFragment {
    let success = doc.procedural.termination.success.as_ref().map(|s| s.ending.clone());
    let ending = success.or_else(|| doc.endings.first().map(|e| e.id.clone())).unwrap_or_else(|| crate::limits::ABANDONED.into());
    let (open, close) = (format!("{prefix}_0"), format!("{prefix}_1"));
    let beat = |id: &str, atom, depth| Beat { id: id.into(), atom, depth, sketch: false, topic: None, goal: None, ending: None, angle: None, exits: vec![] };
    let mut anchor = beat(&open, AtomKind::Open, 0);
    anchor.exits.push(Exit { to: close.clone(), detour: None });
    let mut end = beat(&close, AtomKind::Close, 1);
    end.ending = Some(ending);
    PlanFragment { anchor: open, beats: vec![anchor, end] }
}

struct Words<'a> {
    doc: &'a ProceduralDialogue,
    wording: &'a AtomWording,
    steers: bool,
}

impl Words<'_> {
    fn about(&self, topic: Option<&String>) -> String {
        topic.and_then(|t| self.doc.topic(t)).map(|t| t.about.clone()).unwrap_or_default()
    }

    fn beat(&self, beat: &Beat) -> Vec<Checkpoint> {
        let text = self.wording.text(beat.atom);
        let about = self.about(beat.topic.as_ref());
        let describe = beat.goal.as_ref().and_then(|g| self.doc.goal(g)).map(|g| g.describe.clone()).unwrap_or_default();
        let goal = fill(text.goal, &about, &describe, beat.angle.as_deref());
        let may_disclose = match (beat.atom, beat.topic.as_ref().and_then(|t| self.doc.topic(t))) {
            (AtomKind::Reveal, Some(topic)) => topic.discloses.clone(),
            _ => Vec::new(),
        };
        let mut first = Checkpoint { id: beat.id.clone(), goal, may_disclose, edges: Vec::new(), steer_to: None };
        if beat.sketch {
            return vec![first];
        }
        match beat.atom {
            AtomKind::Close => {
                let ending = beat.ending.clone().unwrap_or_default();
                first.edges.push(edge(format!("{}{END}", beat.id), ending, None, Some(fill(text.done_when, "", "", None))));
                vec![first]
            }
            AtomKind::Practise => self.practise(beat, text, first, &about, &describe),
            _ => {
                first.edges = self.exits(beat, Some(fill(text.done_when, &about, "", None)), None);
                self.steer(&mut first, &format!("{}{EXIT}0", beat.id));
                vec![first]
            }
        }
    }

    /// The done exit (k = 0, if it has a `when`), Practise's predicate edge,
    /// then each detour (k = 1, 2, …), in exit order.
    fn exits(&self, beat: &Beat, done_when: Option<String>, done_predicate: Option<String>) -> Vec<Edge> {
        let mut edges = Vec::new();
        let detours = beat.exits.iter().filter(|e| e.detour.is_some());
        if let Some(done) = beat.done_exit() {
            if done_when.is_some() {
                edges.push(edge(format!("{}{EXIT}0", beat.id), done.to.clone(), None, done_when));
            }
            if done_predicate.is_some() {
                edges.push(edge(format!("{}{OK}", beat.id), done.to.clone(), done_predicate, None));
            }
        }
        for (k, exit) in detours.enumerate() {
            let topic = exit.detour.as_ref().and_then(|t| self.doc.topic(t));
            let when = fill(self.wording.detour_when, &self.about(exit.detour.as_ref()), "", None);
            let predicate = topic.and_then(|t| t.when.clone());
            edges.push(edge(format!("{}{EXIT}{}", beat.id, k + 1), exit.to.clone(), predicate, Some(when)));
        }
        edges
    }

    fn practise(&self, beat: &Beat, text: &AtomText, mut first: Checkpoint, about: &str, describe: &str) -> Vec<Checkpoint> {
        let goal = beat.goal.as_ref().and_then(|g| self.doc.goal(g));
        let (when, predicate) = (goal.and_then(|g| g.when.clone()), goal.and_then(|g| g.predicate.clone()));
        let again_id = format!("{}{AGAIN}", beat.id);
        let filled = |t: Option<&str>| fill(t.unwrap_or_default(), about, describe, None);
        first.edges = self.exits(beat, when.clone(), predicate.clone());
        first.edges.push(edge(format!("{}{TRY}", beat.id), again_id.clone(), None, Some(filled(text.try_when))));
        let mut again = Checkpoint { id: again_id.clone(), goal: filled(text.again_goal), may_disclose: Vec::new(), edges: Vec::new(), steer_to: None };
        let done_only = Beat { exits: beat.done_exit().cloned().into_iter().collect(), id: again_id, ..beat.clone() };
        again.edges = self.exits(&done_only, when.clone(), predicate);
        again.edges.push(edge(format!("{}{RETRY}", beat.id), beat.id.clone(), None, Some(filled(text.retry_when))));
        if when.is_some() {
            self.steer(&mut first, &format!("{}{EXIT}0", beat.id));
            self.steer(&mut again, &format!("{}{AGAIN}{EXIT}0", beat.id));
        }
        vec![first, again]
    }

    /// §7: a driving NPC is told the done exit's `when` edge.
    fn steer(&self, checkpoint: &mut Checkpoint, edge_id: &str) {
        if self.steers && checkpoint.edges.iter().any(|e| e.id == edge_id) {
            checkpoint.steer_to = Some(edge_id.into());
        }
    }
}

fn edge(id: String, to: String, predicate: Option<String>, when: Option<String>) -> Edge {
    Edge { id, to, predicate, when }
}
