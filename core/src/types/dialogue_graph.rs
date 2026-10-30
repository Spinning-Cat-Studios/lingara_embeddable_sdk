//! The dialogue's types (D8; ADR 28.10.26f D6, D7): the free-form config and
//! snapshot, and what a graph dialogue adds: its source, its session, the two
//! words its events carry, and its snapshot. The value types (`FactValue`,
//! `NpcLanguage`, `Pedagogy`) are `lingara-dialogue-ir`'s, re-exported here
//! and never redefined, so the IR and the commands share one type each.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[cfg(feature = "schema")]
use schemars::JsonSchema;

pub use lingara_dialogue_ir::{FactValue, NpcLanguage, Pedagogy};

use super::{DialoguePhase, Npc, WindowEntry};

data! {
    /// B3's request, held between turns (D8), with N1's profile (ADR
    /// 28.10.26f D7): both omitted from JSON when unset, so an existing
    /// `dialogue_open` reads and writes back byte-equal.
    pub struct DialogueConfig {
        pub npc: Npc,
        #[serde(default)]
        pub scene: Option<String>,
        pub source_lang: String,
        pub target_lang: String,
        pub level: u8,
        #[serde(default)]
        pub window: Vec<WindowEntry>,
        /// The NPC's languages: 1–8 entries with unique codes, or none.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub languages: Vec<NpcLanguage>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub pedagogy: Option<Pedagogy>,
    }

    pub struct DialogueSnapshot {
        pub phase: DialoguePhase,
        pub npc: Npc,
        pub scene: Option<String>,
        pub level: u8,
        pub window: Vec<WindowEntry>,
        /// Where a graph dialogue's story is; omitted for a free-form one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub graph: Option<GraphSnapshot>,
    }

    /// Where a graph dialogue comes from.
    #[serde(tag = "type", rename_all = "snake_case")]
    #[non_exhaustive]
    pub enum DialogueSource {
        /// A `dialogue.v1` document, as text.
        Ir { ir_json: String },
        /// The active revision of a dialogue saved on Lingara, by its store
        /// id (a UUID, never the IR's own `id`), fetched with the player's
        /// token (ADR 28.10.26i D2).
        Stored { dialogue_id: String },
    }

    /// One run of a graph: which cast member speaks, the learner's settings,
    /// and, to resume a saved story, its checkpoint, facts and window.
    pub struct GraphSession {
        /// A cast member's id; may be omitted only when the cast has one.
        #[serde(default)]
        pub npc: Option<String>,
        pub source_lang: String,
        pub target_lang: String,
        pub level: u8,
        #[serde(default)]
        pub scene: Option<String>,
        /// Values set before the first auto-crossing, each checked as `DialogueSetFact` checks it.
        #[serde(default)]
        pub facts: BTreeMap<String, FactValue>,
        /// Resume here instead of at the IR's `start`.
        #[serde(default)]
        pub checkpoint: Option<String>,
        #[serde(default)]
        pub window: Vec<WindowEntry>,
    }

    /// A graph dialogue's story, for the game to save. `facts` holds only
    /// the values the game set, never defaults. Fed back as a
    /// [`GraphSession`] with the same IR, it restores the story.
    pub struct GraphSnapshot {
        /// The IR's own `id`.
        pub id: String,
        pub npc: String,
        pub checkpoint: String,
        pub facts: BTreeMap<String, FactValue>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub outcome: Option<DialogueOutcome>,
    }
}

word! {
    /// How a checkpoint was reached: the load, an edge with no `when`, or the judge.
    pub enum CrossedVia { Start, Auto, Judge }

    /// The reserved `to` names a story ends on.
    pub enum DialogueOutcome { Success, Failure, Abandoned }
}

impl DialogueOutcome {
    /// The outcome a reserved `to` name stands for.
    pub(crate) fn from_name(name: &str) -> Option<Self> {
        match name {
            "success" => Some(DialogueOutcome::Success),
            "failure" => Some(DialogueOutcome::Failure),
            "abandoned" => Some(DialogueOutcome::Abandoned),
            _ => None,
        }
    }
}
