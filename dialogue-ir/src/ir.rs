//! The `dialogue.v1` IR as Rust reads it (ADR 28.10.26f D1). The shape is
//! Backend ADR 28.10.26e §10's, unchanged.
//!
//! Serialising one writes the normal form: `pedagogy`, `facts`,
//! `may_disclose` and `edges` always written, every other optional omitted
//! when absent, never a `null`, arrays in author order, numbers as written.
//!
//! Every nested type is closed. Only [`DialogueIr`] keeps unknown members, its
//! top-level `x-` ones, because serde cannot combine `deny_unknown_fields`
//! with `flatten`. [`crate::from_json`] refuses any other unknown member, with
//! its path, before it deserialises.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};

/// The one `schema` string this version reads.
pub const SCHEMA_V1: &str = "dialogue.v1";

/// How much the NPC adapts its speech to the learner (N1). Per dialogue.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Pedagogy {
    /// The speech level is `min(NPC speaking, learner level)`.
    #[default]
    Subtle,
    /// The NPC's own level decides.
    None,
}

/// The three types a fact may be declared with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum FactType {
    String,
    Number,
    Boolean,
}

/// A fact's value: a declaration's `default`, or what the game sets.
///
/// A number is held as the [`Number`] it was written as, so `0` stays `0` and
/// `0.4` stays `0.4` when the IR is written back.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum FactValue {
    Boolean(bool),
    Number(Number),
    String(String),
}

impl FactValue {
    /// The type this value would satisfy.
    pub fn fact_type(&self) -> FactType {
        match self {
            FactValue::Boolean(_) => FactType::Boolean,
            FactValue::Number(_) => FactType::Number,
            FactValue::String(_) => FactType::String,
        }
    }
}

/// One entry of an NPC's language profile (N1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct NpcLanguage {
    pub code: String,
    pub speaking: u8,
    pub comprehension: u8,
}

/// A member of the cast. A run opens with one of them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct NpcProfile {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub persona: Option<String>,
    /// Absent means N1's native speaker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub languages: Option<Vec<NpcLanguage>>,
}

/// A fact the game tracks. It must be declared to be read by a predicate or
/// named in `may_disclose`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct FactDecl {
    pub key: String,
    #[serde(rename = "type")]
    pub kind: FactType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<FactValue>,
}

/// A way out of a checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Edge {
    pub id: String,
    /// A checkpoint id, or one of [`crate::limits::OUTCOMES`].
    pub to: String,
    /// The game condition, in this crate's predicate language.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub predicate: Option<String>,
    /// The conversational condition the server's judge reads. An edge
    /// without one is crossed by the core alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
}

/// A beat of the story.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Checkpoint {
    pub id: String,
    pub goal: String,
    /// Fact keys the NPC may voice here.
    #[serde(default)]
    pub may_disclose: Vec<String>,
    #[serde(default)]
    pub edges: Vec<Edge>,
}

/// One `dialogue.v1` document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DialogueIr {
    /// Always [`SCHEMA_V1`].
    pub schema: String,
    /// The author's slug. Not a store id.
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default)]
    pub pedagogy: Pedagogy,
    pub npcs: Vec<NpcProfile>,
    #[serde(default)]
    pub facts: Vec<FactDecl>,
    pub start: String,
    pub checkpoints: Vec<Checkpoint>,
    /// Top-level `x-` members (P1's `x-layout`), kept as written and ignored.
    #[serde(flatten)]
    pub extensions: BTreeMap<String, Value>,
}

impl DialogueIr {
    /// The declaration of `key`, if there is one.
    pub fn fact(&self, key: &str) -> Option<&FactDecl> {
        self.facts.iter().find(|fact| fact.key == key)
    }

    /// The checkpoint named `id`, if there is one.
    pub fn checkpoint(&self, id: &str) -> Option<&Checkpoint> {
        self.checkpoints.iter().find(|checkpoint| checkpoint.id == id)
    }
}
