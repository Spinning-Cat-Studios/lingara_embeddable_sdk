//! The mirrors of K1's graph dialogue records (ADR 28.10.26f D6, D8), under
//! `types.rs`'s rules. The graph's three methods are `dialogue.rs`'s.
//!
//! K1's `FactValue` is untagged (a JSON boolean, number or string), which a
//! UniFFI enum cannot say, so the mirror is a three-arm enum with a
//! hand-written serde that writes and reads the bare JSON value. A whole
//! number crosses as an integer, so `0` reaches K1 as `0`, not `0.0`; a NaN
//! or an infinity writes `null`, which K1 refuses as `invalid_input`.

use std::collections::HashMap;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use crate::types::{EmbedException, WindowEntry};

/// A fact's value: what the game sets, and what a snapshot holds.
#[derive(Debug, Clone, PartialEq, uniffi::Enum)]
pub enum FactValue {
    Boolean { value: bool },
    Number { value: f64 },
    Text { value: String },
}

impl Serialize for FactValue {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            FactValue::Boolean { value } => serializer.serialize_bool(*value),
            FactValue::Number { value } if value.fract() == 0.0 && value.abs() < 9.0e15 => serializer.serialize_i64(*value as i64),
            FactValue::Number { value } => serializer.serialize_f64(*value),
            FactValue::Text { value } => serializer.serialize_str(value),
        }
    }
}

impl<'de> Deserialize<'de> for FactValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match Value::deserialize(deserializer)? {
            Value::Bool(value) => Ok(FactValue::Boolean { value }),
            Value::Number(n) => n.as_f64().map(|value| FactValue::Number { value }).ok_or_else(|| D::Error::custom(format!("{n} is not a finite number"))),
            Value::String(value) => Ok(FactValue::Text { value }),
            other => Err(D::Error::custom(format!("a fact is a boolean, a number or a string, not {other}"))),
        }
    }
}

macro_rules! word {
    ($($item:item)*) => { $(
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
        #[serde(rename_all = "snake_case")]
        $item
    )* };
}

word! {
    /// What became of a procedural run's plan (ADR 3.11.26j D9).
    pub enum PlanOutcome { Accepted, Kept }

    /// Why a run kept its frontier.
    pub enum PlanKeptReason { Stale, Rejected, Unavailable, Timeout, Unparseable }

    /// The five beat templates a procedural dialogue composes.
    pub enum AtomKind { Open, Elicit, Reveal, Practise, Close }
}

/// A dialogue document of either kind in the crate's canonical form (ADR
/// 3.11.26j D8): the bytes `DialogueLoad` sends. The Kotlin DSL writes plain
/// JSON and canonicalises it here, so the JVM has no canonical writer of its own.
#[uniffi::export]
pub fn canonical_dialogue(ir_json: String) -> Result<String, EmbedException> {
    let value: Value = serde_json::from_str(&ir_json).map_err(|e| crate::convert::invalid("ir_json", e))?;
    let doc = lingara_embed::dialogue_ir::document_from_value(&value).map_err(|e| crate::convert::invalid("ir_json", e))?;
    Ok(doc.to_canonical_json())
}

macro_rules! record {
    ($($item:item)*) => { $(
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
        $item
    )* };
}

record! {
    /// One of an NPC's languages: levels 1–9.
    pub struct NpcLanguage {
        pub code: String,
        pub speaking: i32,
        pub comprehension: i32,
    }

    /// One run of a graph: the cast member, the learner's settings, and
    /// what a saved story resumes with.
    pub struct GraphSession {
        #[serde(default)]
        pub npc: Option<String>,
        pub source_lang: String,
        pub target_lang: String,
        pub level: i32,
        #[serde(default)]
        pub scene: Option<String>,
        #[serde(default)]
        pub facts: HashMap<String, FactValue>,
        #[serde(default)]
        pub checkpoint: Option<String>,
        #[serde(default)]
        pub window: Vec<WindowEntry>,
        /// A procedural run's saved frontier, a `PlanFragment` as JSON text
        /// (ADR 3.11.26j D9); `checkpoint` is required with it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[uniffi(default)]
        pub frontier: Option<String>,
        /// The saved run's counters and trace; only with `frontier`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[uniffi(default)]
        pub progress: Option<ProceduralProgress>,
    }

    /// A beat a procedural run crossed, as the planner reads it.
    pub struct VisitedBeat {
        pub atom: AtomKind,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub topic: Option<String>,
    }

    /// One beat of a procedural run's whole path (ADR 3.11.26j D13): ids and
    /// counters, never text.
    pub struct TracedBeat {
        pub id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub atom: Option<AtomKind>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub topic: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub goal: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub ending: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub left_by_edge: Option<String>,
        pub via: crate::types::CrossedVia,
        pub entered_turn: i32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub left_turn: Option<i32>,
        pub plan_seq: i32,
    }

    /// A procedural run's counters and trace, saved beside its frontier.
    pub struct ProceduralProgress {
        #[serde(default)]
        pub visited: Vec<VisitedBeat>,
        #[serde(default)]
        pub trace: Vec<TracedBeat>,
        #[serde(default)]
        pub plans: i32,
        #[serde(default)]
        pub next_prefix: i32,
        #[serde(default)]
        pub frontier_seq: i32,
        #[serde(default)]
        pub beats: i32,
        #[serde(default)]
        pub turns: i32,
        #[serde(default)]
        pub since_crossing: i32,
    }

    /// A procedural run's half of a snapshot: `GraphSession`'s `frontier` and
    /// `progress`, to restore it.
    pub struct ProceduralSnapshot {
        pub frontier: String,
        pub progress: ProceduralProgress,
    }

    /// A graph dialogue's story: only the facts the game set.
    pub struct GraphSnapshot {
        pub id: String,
        pub npc: String,
        pub checkpoint: String,
        pub facts: HashMap<String, FactValue>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        // The ending's id, or `abandoned` (ADR 3.11.26g §11.3).
        pub outcome: Option<String>,
    }
}
