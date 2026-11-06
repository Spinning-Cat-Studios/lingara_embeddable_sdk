//! The Rust procedural builder (ADR 3.11.26j D8): a [`ProceduralDialogue`]
//! in 3.11.26g's canonical form, always `dialogue.v2`. The cast, facts and
//! endings are the static builder's own parts; what is new is the box the
//! planner fills. Atoms are an enum, never free text. `build()` runs the
//! crate's own validation, so a built document is one the core loads.

use std::collections::BTreeMap;

use serde_json::Number;

use crate::document::Document;
use crate::ir::{Ending, FactDecl, Pedagogy, SCHEMA_V2};
use crate::path::{index, member};
use crate::procedural::{AtomKind, AtomUse, Horizon, ProceduralDialogue, ProceduralSpec};
use crate::validate::{Strictness, validate_with};
use crate::IrError;

use super::procedural_ends::Termination;
use super::procedural_parts::{Forbid, Goal, Topic};
use super::{Fact, FactDefault, Npc, builder_number};

/// A procedural dialogue under construction.
pub struct Procedural {
    doc: ProceduralDialogue,
    defaults: Vec<Option<FactDefault>>,
}

impl Procedural {
    /// `premise` is what the whole conversation is about, one line.
    pub fn new(id: &str, premise: &str) -> Self {
        let procedural = ProceduralSpec {
            premise: premise.into(),
            disclosable: Vec::new(),
            topics: Vec::new(),
            forbidden: Vec::new(),
            goals: Vec::new(),
            atoms: Vec::new(),
            horizon: Horizon::default(),
            termination: Termination::new(1, 1).into_spec(),
        };
        let doc = ProceduralDialogue {
            schema: SCHEMA_V2.into(),
            id: id.into(),
            title: None,
            pedagogy: Pedagogy::default(),
            npcs: Vec::new(),
            facts: Vec::new(),
            endings: Vec::new(),
            procedural,
            extensions: BTreeMap::new(),
        };
        Procedural { doc, defaults: Vec::new() }
    }

    pub fn title(mut self, title: &str) -> Self {
        self.doc.title = Some(title.into());
        self
    }

    pub fn pedagogy(mut self, pedagogy: Pedagogy) -> Self {
        self.doc.pedagogy = pedagogy;
        self
    }

    pub fn npc(mut self, npc: Npc) -> Self {
        self.doc.npcs.push(npc.into_profile());
        self
    }

    pub fn fact(mut self, fact: Fact) -> Self {
        let (key, kind, default) = fact.into_parts();
        self.doc.facts.push(FactDecl { key, kind, default: None });
        self.defaults.push(default);
        self
    }

    /// Declares a named ending; `label` is what a person reads, `""` for none.
    pub fn ending(mut self, id: &str, label: &str) -> Self {
        let label = (!label.is_empty()).then(|| label.to_string());
        self.doc.endings.push(Ending { id: id.into(), label });
        self
    }

    /// The fact keys a Reveal beat may disclose: the knowledge pool.
    pub fn disclosable<'a>(mut self, keys: impl IntoIterator<Item = &'a str>) -> Self {
        self.doc.procedural.disclosable.extend(keys.into_iter().map(String::from));
        self
    }

    pub fn topic(mut self, topic: Topic) -> Self {
        self.doc.procedural.topics.push(topic.into_spec());
        self
    }

    /// Something never to raise: planner guidance, not a guarantee.
    pub fn forbid(mut self, forbid: Forbid) -> Self {
        self.doc.procedural.forbidden.push(forbid.into_spec());
        self
    }

    pub fn goal(mut self, goal: Goal) -> Self {
        self.doc.procedural.goals.push(goal.into_spec());
        self
    }

    /// The atoms the planner may compose, in order; `open` and `close` are required.
    pub fn atoms(mut self, atoms: impl IntoIterator<Item = AtomKind>) -> Self {
        self.doc.procedural.atoms.extend(atoms.into_iter().map(|atom| AtomUse { atom, prefer: None }));
        self
    }

    /// A planner hint, 0–1, on an atom already listed.
    pub fn prefer(mut self, atom: AtomKind, prefer: f64) -> Self {
        let number = builder_number(prefer).unwrap_or_else(|| Number::from(2));
        self.doc.procedural.atoms.iter_mut().filter(|a| a.atom == atom).for_each(|a| a.prefer = Some(number.clone()));
        self
    }

    pub fn horizon(mut self, plan_depth: u8, sketch_depth: u8, max_nodes: u8) -> Self {
        self.doc.procedural.horizon = Horizon { plan_depth, sketch_depth, max_nodes };
        self
    }

    pub fn termination(mut self, termination: Termination) -> Self {
        self.doc.procedural.termination = termination.into_spec();
        self
    }

    /// The fact defaults in D2's builder range, then the crate's own
    /// procedural validation (spec rules 1–7), with its paths and rules.
    pub fn build(self) -> Result<ProceduralDialogue, IrError> {
        let mut doc = self.doc;
        for (i, (fact, default)) in doc.facts.iter_mut().zip(self.defaults).enumerate() {
            fact.default = default.map(|d| d.into_value(&member(&index("facts", i), "default"))).transpose()?;
        }
        validate_with(&Document::Procedural(doc.clone()), Strictness::Load)?;
        Ok(doc)
    }
}
