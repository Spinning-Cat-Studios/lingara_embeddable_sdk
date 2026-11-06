//! The dialogue's types (D8; ADR 28.10.26f D6, D7): the free-form config and
//! snapshot, and what a graph dialogue adds: its source, its session, the two
//! words its events carry, and its snapshot. The value types (`FactValue`,
//! `NpcLanguage`, `Pedagogy`) are `lingara-dialogue-ir`'s, re-exported here
//! and never redefined, so the IR and the commands share one type each.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[cfg(feature = "schema")]
use schemars::JsonSchema;

pub use lingara_dialogue_ir::procedural::AtomKind;
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
        /// A procedural run's frontier and counters; omitted otherwise (ADR 3.11.26j D9).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub procedural: Option<ProceduralSnapshot>,
    }

    /// Where a graph dialogue comes from.
    #[serde(tag = "type", rename_all = "snake_case")]
    #[non_exhaustive]
    pub enum DialogueSource {
        /// A `dialogue.v1` or `dialogue.v2` document, as text.
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
        /// A procedural run's saved frontier, a `PlanFragment` as JSON text
        /// (ADR 3.11.26j D9): with it `checkpoint` is required, and the run
        /// resumes on that frontier instead of asking its first plan.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub frontier: Option<String>,
        /// The saved run's counters and trace; only with `frontier`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub progress: Option<ProceduralProgress>,
    }

    /// A beat a procedural run crossed, as the planner reads it: the last 16,
    /// oldest first (ADR 3.11.26j D2).
    pub struct VisitedBeat {
        pub atom: AtomKind,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub topic: Option<String>,
    }

    /// One beat of a procedural run's whole path (ADR 3.11.26j D13): ids,
    /// one enum word and three counters, never text.
    pub struct TracedBeat {
        /// The beat id, never `‹beat›_again`.
        pub id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub atom: Option<AtomKind>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub topic: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub goal: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub ending: Option<String>,
        /// The edge that left the beat; `None` while it is current, or when
        /// the run ended by a rule with no edge.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub left_by_edge: Option<String>,
        /// How the beat was entered.
        pub via: CrossedVia,
        /// `turns` when the beat was entered.
        pub entered_turn: u16,
        /// `turns` when it was left; `None` while it is current.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub left_turn: Option<u16>,
        /// The accepted frontier it was entered under, 1 for the first.
        pub plan_seq: u16,
    }

    /// A procedural run's counters (ADR 3.11.26j D9), saved beside its
    /// frontier so a restore keeps its plan bound and its fresh prefixes.
    #[derive(Default)]
    pub struct ProceduralProgress {
        #[serde(default)]
        pub visited: Vec<VisitedBeat>,
        #[serde(default)]
        pub trace: Vec<TracedBeat>,
        #[serde(default)]
        pub plans: u16,
        #[serde(default)]
        pub next_prefix: u16,
        #[serde(default)]
        pub frontier_seq: u16,
        #[serde(default)]
        pub beats: u16,
        #[serde(default)]
        pub turns: u16,
        #[serde(default)]
        pub since_crossing: u16,
    }

    /// A procedural run's half of a snapshot: fed back as `GraphSession`'s
    /// `frontier` and `progress`, it restores the run.
    pub struct ProceduralSnapshot {
        /// The frontier the run walks, a `PlanFragment` as JSON text.
        pub frontier: String,
        pub progress: ProceduralProgress,
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
        /// The ending reached: a declared ending's id, or `abandoned`.
        pub outcome: Option<String>,
    }
}

word! {
    /// How a checkpoint was reached: the load, an edge with no `when`, or the judge.
    pub enum CrossedVia { Start, Auto, Judge }

    /// What became of a plan (ADR 3.11.26j D9): its fragment is the frontier, or the run kept its own.
    pub enum PlanOutcome { Accepted, Kept }

    /// Why a run kept its frontier (D5): the answer was stale or refused here,
    /// or the server's own reason, or the call failed or timed out.
    pub enum PlanKeptReason { Stale, Rejected, Unavailable, Timeout, Unparseable }
}

/// The ending a story reaches when the game closes it unfinished. Every other
/// outcome is a declared ending's id (ADR 3.11.26g §11.3): `outcome` is an
/// open string, no longer a closed enum.
pub const ABANDONED: &str = lingara_dialogue_ir::limits::ABANDONED;

/// Whether `outcome` is the built-in [`ABANDONED`].
pub fn is_abandoned(outcome: &str) -> bool {
    outcome == ABANDONED
}
