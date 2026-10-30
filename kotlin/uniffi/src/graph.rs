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

use crate::types::{DialogueOutcome, WindowEntry};

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
    }

    /// A graph dialogue's story: only the facts the game set.
    pub struct GraphSnapshot {
        pub id: String,
        pub npc: String,
        pub checkpoint: String,
        pub facts: HashMap<String, FactValue>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub outcome: Option<DialogueOutcome>,
    }
}
