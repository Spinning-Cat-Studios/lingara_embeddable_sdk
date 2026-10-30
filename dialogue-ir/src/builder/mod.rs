//! The Rust dialogue builder (ADR 28.10.26g D1). It builds K1's
//! [`DialogueIr`] directly, over K1's own types, and holds parts in the order
//! the author adds them: edge order is meaningful (K1 auto-crosses the first
//! open condition-less edge; N2's judge takes the first matching edge).
//!
//! A predicate is K1 D3's text, never a closure: a closure has no text.

mod fact;
mod parts;

use std::collections::BTreeMap;

use crate::ir::{DialogueIr, FactDecl, Pedagogy, SCHEMA_V1};
use crate::path::{index, member};
use crate::{IrError, validate};

pub use crate::levels::Level;
pub use fact::{Fact, FactDefault, MAX_SAFE_INTEGER, builder_number};
pub use parts::{Checkpoint, Edge, Npc};

/// Where an edge ends a dialogue instead of naming a checkpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Success,
    Failure,
    Abandoned,
}

impl Outcome {
    fn as_str(self) -> &'static str {
        match self {
            Outcome::Success => "success",
            Outcome::Failure => "failure",
            Outcome::Abandoned => "abandoned",
        }
    }
}

/// An edge's `to`: a checkpoint id or an [`Outcome`].
pub struct Target(String);

impl From<Outcome> for Target {
    fn from(outcome: Outcome) -> Self {
        Target(outcome.as_str().into())
    }
}

impl From<&str> for Target {
    fn from(checkpoint: &str) -> Self {
        Target(checkpoint.into())
    }
}

/// A dialogue under construction.
pub struct Dialogue {
    ir: DialogueIr,
    defaults: Vec<Option<FactDefault>>,
}

impl Dialogue {
    pub fn new(id: &str, start: &str) -> Self {
        let ir = DialogueIr {
            schema: SCHEMA_V1.into(),
            id: id.into(),
            title: None,
            pedagogy: Pedagogy::default(),
            npcs: Vec::new(),
            facts: Vec::new(),
            start: start.into(),
            checkpoints: Vec::new(),
            extensions: BTreeMap::new(),
        };
        Dialogue { ir, defaults: Vec::new() }
    }

    pub fn title(mut self, title: &str) -> Self {
        self.ir.title = Some(title.into());
        self
    }

    pub fn pedagogy(mut self, pedagogy: Pedagogy) -> Self {
        self.ir.pedagogy = pedagogy;
        self
    }

    pub fn npc(mut self, npc: Npc) -> Self {
        self.ir.npcs.push(npc.into_profile());
        self
    }

    pub fn fact(mut self, fact: Fact) -> Self {
        let (key, kind, default) = fact.into_parts();
        self.ir.facts.push(FactDecl { key, kind, default: None });
        self.defaults.push(default);
        self
    }

    pub fn checkpoint(mut self, checkpoint: Checkpoint) -> Self {
        self.ir.checkpoints.push(checkpoint.into_ir());
        self
    }

    /// D2's builder ranges, then K1's full `validate`, so a built dialogue is
    /// never one the core refuses.
    pub fn build(self) -> Result<DialogueIr, IrError> {
        let mut ir = self.ir;
        for (i, (fact, default)) in ir.facts.iter_mut().zip(self.defaults).enumerate() {
            fact.default = default.map(|d| d.into_value(&member(&index("facts", i), "default"))).transpose()?;
        }
        validate(&ir)?;
        Ok(ir)
    }
}
