//! The cast and the story's parts: an NPC, a checkpoint and its edges.

use serde_json::Number;

use crate::ir::{Checkpoint as IrCheckpoint, Disposition, Edge as IrEdge, NpcLanguage, NpcProfile};
use crate::levels::Level;

use super::{Target, builder_number};

/// A member of the cast.
pub struct Npc(NpcProfile);

impl Npc {
    pub fn new(id: &str, name: &str) -> Self {
        Npc(NpcProfile { id: id.into(), name: name.into(), persona: None, languages: None, disposition: None })
    }

    pub fn persona(mut self, persona: &str) -> Self {
        self.0.persona = Some(persona.into());
        self
    }

    /// One language the NPC speaks, at its speaking and comprehension levels.
    pub fn speaks(mut self, code: &str, speaking: impl Into<Level>, comprehension: impl Into<Level>) -> Self {
        let entry = NpcLanguage { code: code.into(), speaking: speaking.into().0, comprehension: comprehension.into().0 };
        self.0.languages.get_or_insert_with(Vec::new).push(entry);
        self
    }

    /// How the NPC steers (ADR 3.11.26j D8): how readily it follows the
    /// player off its goal, and how hard it drives toward it, each 0–1. One
    /// call makes a static dialogue `dialogue.v2` (3.11.26g §2's write rule).
    /// A number outside D2's builder range is held as 2, which
    /// `number_range` refuses at build.
    pub fn disposition(mut self, suggestibility: f64, initiative: f64) -> Self {
        let held = |x| builder_number(x).unwrap_or_else(|| Number::from(2));
        self.0.disposition = Some(Disposition { suggestibility: held(suggestibility), initiative: held(initiative) });
        self
    }

    pub(super) fn into_profile(self) -> NpcProfile {
        self.0
    }
}

/// A beat of the story.
pub struct Checkpoint(IrCheckpoint);

impl Checkpoint {
    pub fn new(id: &str, goal: &str) -> Self {
        Checkpoint(IrCheckpoint { id: id.into(), goal: goal.into(), may_disclose: Vec::new(), edges: Vec::new(), steer_to: None, label: None })
    }

    /// A fact key the NPC may voice here.
    pub fn discloses(mut self, key: &str) -> Self {
        self.0.may_disclose.push(key.into());
        self
    }

    pub fn edge(mut self, edge: Edge) -> Self {
        self.0.edges.push(edge.0);
        self
    }

    pub(super) fn into_ir(self) -> IrCheckpoint {
        self.0
    }
}

/// A way out of a checkpoint.
pub struct Edge(IrEdge);

impl Edge {
    pub fn new(id: &str, to: impl Into<Target>) -> Self {
        Edge(IrEdge { id: id.into(), to: to.into().0, predicate: None, when: None })
    }

    /// The game condition, in K1's predicate language (`trust >= 0.4`).
    pub fn predicate(mut self, text: &str) -> Self {
        self.0.predicate = Some(text.into());
        self
    }

    /// The conversational condition the server's judge reads.
    pub fn when(mut self, text: &str) -> Self {
        self.0.when = Some(text.into());
        self
    }
}
