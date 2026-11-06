//! The procedural builder's parts (ADR 3.11.26j D8): a topic, a goal and a
//! forbidden subject, each over the crate's own spec type. The termination
//! rules are `procedural_ends.rs`'s.

use crate::procedural::{Forbidden, Goal as SpecGoal, Topic as SpecTopic};

/// Something that may come up: with whom, when, and what a Reveal on it discloses.
pub struct Topic(SpecTopic);

impl Topic {
    pub fn new(id: &str, about: &str) -> Self {
        Topic(SpecTopic { id: id.into(), about: about.into(), npcs: None, when: None, discloses: Vec::new() })
    }

    /// A K1 predicate over facts; the topic is open only while it holds.
    pub fn when(mut self, predicate: &str) -> Self {
        self.0.when = Some(predicate.into());
        self
    }

    /// The cast ids that may raise it; never called, every cast member.
    pub fn npcs<'a>(mut self, ids: impl IntoIterator<Item = &'a str>) -> Self {
        self.0.npcs = Some(ids.into_iter().map(String::from).collect());
        self
    }

    /// The facts a Reveal beat on this topic discloses: within `disclosable`.
    pub fn discloses<'a>(mut self, keys: impl IntoIterator<Item = &'a str>) -> Self {
        self.0.discloses.extend(keys.into_iter().map(String::from));
        self
    }

    pub(super) fn into_spec(self) -> SpecTopic {
        self.0
    }
}

/// A subject never to raise, for every cast member or for some.
pub struct Forbid(Forbidden);

impl Forbid {
    pub fn new(about: &str) -> Self {
        Forbid(Forbidden { about: about.into(), npcs: None })
    }

    pub fn npcs<'a>(mut self, ids: impl IntoIterator<Item = &'a str>) -> Self {
        self.0.npcs = Some(ids.into_iter().map(String::from).collect());
        self
    }

    pub(super) fn into_spec(self) -> Forbidden {
        self.0
    }
}

/// A named success target: the game decides it (`predicate`), the judge
/// does (`when`), or both.
pub struct Goal(SpecGoal);

impl Goal {
    pub fn new(id: &str, describe: &str) -> Self {
        Goal(SpecGoal { id: id.into(), describe: describe.into(), predicate: None, when: None })
    }

    pub fn predicate(mut self, text: &str) -> Self {
        self.0.predicate = Some(text.into());
        self
    }

    pub fn when(mut self, text: &str) -> Self {
        self.0.when = Some(text.into());
        self
    }

    pub(super) fn into_spec(self) -> SpecGoal {
        self.0
    }
}
