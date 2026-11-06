//! The dialogue IR as Rust reads it (ADR 28.10.26f D1): `dialogue.v1`, the
//! shape of Backend ADR 28.10.26e §10, and the static half of `dialogue.v2`
//! (ADR 3.11.26g §1), which adds `endings`, `disposition` and `steer_to`.
//! The procedural kind is [`crate::procedural`]'s.
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

/// The first `schema` string, still read and still written (§2).
pub const SCHEMA_V1: &str = "dialogue.v1";
/// The second, for a document v1 cannot say (ADR 3.11.26g).
pub const SCHEMA_V2: &str = "dialogue.v2";

/// Which of the two kinds of checkpointed graph a v2 document is (§1).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum DialogueKind {
    /// Written by its author: `start` and `checkpoints`.
    #[default]
    Static,
    /// Written at runtime inside a [`crate::procedural::ProceduralSpec`].
    Procedural,
}

/// A way a conversation can end (§11.1). A v1 document is read as declaring
/// `success` and `failure`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Ending {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

impl Ending {
    /// An ending with no label.
    pub fn bare(id: &str) -> Self {
        Ending { id: id.into(), label: None }
    }
}

/// An NPC's steering stance (§3). Both in 0..=1, held as the `Number` written,
/// like a [`FactValue`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Disposition {
    /// How readily the NPC follows the player off its goal.
    pub suggestibility: Number,
    /// How hard the NPC drives toward its goal or `steer_to`.
    pub initiative: Number,
}

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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// `dialogue.v2`: the stance (§3). Absent plans as Medium / Medium.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disposition: Option<Disposition>,
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
    /// A checkpoint id, a declared ending, or [`crate::limits::ABANDONED`].
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
    /// `dialogue.v2`: an edge id of this checkpoint with a `when`, where the
    /// NPC drives the conversation (§3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub steer_to: Option<String>,
    /// `dialogue.v2`: a name a person reads (ADR 6.11.26b). Display text only:
    /// nothing references it, and the runner never reads it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// A static graph, `dialogue.v1` or `dialogue.v2`.
///
/// Serialised directly it writes every member; the canonical writer
/// ([`DialogueIr::to_canonical_json`]) writes the schema
/// [`crate::version::write_schema`] chooses, and strips v2's members from a v1
/// write.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DialogueIr {
    /// The schema that was read: informational. Every writer uses
    /// [`crate::version::write_schema`]'s answer instead.
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
    /// The declared endings; a v1 read fills `[success, failure]` (§2).
    #[serde(default)]
    pub endings: Vec<Ending>,
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

    /// Whether `id` ends the story: a declared ending or `abandoned` (§11.3).
    pub fn is_ending(&self, id: &str) -> bool {
        id == crate::limits::ABANDONED || self.endings.iter().any(|ending| ending.id == id)
    }
}
