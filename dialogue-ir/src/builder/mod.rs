//! The Rust dialogue builder (ADR 28.10.26g D1). It builds K1's
//! [`DialogueIr`] directly, over K1's own types, and holds parts in the order
//! the author adds them: edge order is meaningful (K1 auto-crosses the first
//! open condition-less edge; N2's judge takes the first matching edge).
//!
//! A predicate is K1 D3's text, never a closure: a closure has no text.
//!
//! Named endings (ADR 3.11.26g §11.5): a dialogue that declares none ends on
//! `success` / `failure` and is written as `dialogue.v1`; one that calls
//! [`Dialogue::ending`] declares exactly the endings it names and is written
//! as `dialogue.v2` when v1 cannot say them.

mod fact;
mod parts;
mod procedural;
mod procedural_ends;
mod procedural_parts;

use std::collections::BTreeMap;

use crate::ir::{DialogueIr, Ending, FactDecl, Pedagogy};
use crate::limits::ABANDONED;
use crate::path::{index, member};
use crate::{IrError, validate, version};

pub use crate::levels::Level;
pub use fact::{Fact, FactDefault, MAX_SAFE_INTEGER, builder_number};
pub use parts::{Checkpoint, Edge, Npc};
// The procedural builder (ADR 3.11.26j D8), and the atom words it takes.
pub use crate::procedural::AtomKind;
pub use procedural::*;
pub use procedural_ends::Termination;
pub use procedural_parts::{Forbid, Goal, Topic};

/// Shorthands for the three v1 ends. `Success` and `Failure` are ordinary
/// ending ids in v2; `Abandoned` is built in.
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
            Outcome::Abandoned => ABANDONED,
        }
    }
}

/// An edge's `to`: a checkpoint id, an ending id, or an [`Outcome`].
pub struct Target(String);

impl Target {
    /// A declared ending, by id.
    pub fn ending(id: &str) -> Self {
        Target(id.into())
    }
}

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
            schema: String::new(),
            id: id.into(),
            title: None,
            pedagogy: Pedagogy::default(),
            npcs: Vec::new(),
            facts: Vec::new(),
            endings: Vec::new(),
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

    /// Declares a named ending; `label` is what a person reads, `""` for none.
    pub fn ending(mut self, id: &str, label: &str) -> Self {
        let label = (!label.is_empty()).then(|| label.to_string());
        self.ir.endings.push(Ending { id: id.into(), label });
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
        if ir.endings.is_empty() {
            ir.endings = version::v1_endings();
        }
        ir.schema = version::static_schema(&ir).into();
        validate(&ir)?;
        Ok(ir)
    }
}
