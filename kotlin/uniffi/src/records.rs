//! The mirrors of K1's plain records (ADR 1.10.26ae D2), field for field,
//! with signed integers only. `types.rs` holds the rules.

use serde::{Deserialize, Serialize};

use crate::types::{DialoguePhase, EmbedError, GraphSnapshot, NpcLanguage, Pedagogy, PracticeOutcome, PracticePhase, Speaker, StreamState};

macro_rules! record {
    ($($item:item)*) => { $(
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
        $item
    )* };
}

record! {
    /// One event off the stream. `data_json` is the event's raw `data`.
    pub struct LingaraEvent {
        pub id: String,
        pub event_type: String,
        pub created_at: String,
        pub data_json: String,
        pub detail: crate::types::LingaraEventDetail,
    }

    /// The handoff's optional `update_available` (ADR 30.10.26p).
    pub struct UpdateAvailable {
        pub latest: Option<String>,
        pub deprecated: bool,
        pub message: Option<String>,
    }

    /// The `202`'s `reaction` to a sent world event.
    pub struct Reaction {
        pub status: String,
        #[serde(default)]
        pub plan_id: Option<String>,
        #[serde(default)]
        pub plan_status: Option<String>,
        #[serde(default)]
        pub code: Option<String>,
        #[serde(default)]
        pub error: Option<String>,
    }

    /// `world.context_changed`, bounded as Lingara's inbound door is.
    pub struct WorldContext {
        pub scene: String,
        pub source_lang: String,
        pub target_lang: String,
        pub level: i32,
        #[serde(default)]
        pub npc: Option<Npc>,
        #[serde(default)]
        pub tags: Vec<String>,
        #[serde(default)]
        pub generate: bool,
    }

    pub struct Npc {
        pub name: String,
        #[serde(default)]
        pub persona: Option<String>,
    }

    pub struct PracticeSnapshot {
        pub phase: PracticePhase,
        pub plan: Option<PlanSummary>,
        pub set_index: i64,
        pub set_count: i64,
        pub question_index: i64,
        pub question_count: i64,
        pub question: Option<PracticeQuestion>,
        pub last_result: Option<AnswerResult>,
        pub failure: Option<EmbedError>,
    }

    pub struct PlanSummary {
        pub id: String,
        pub title: Option<String>,
        pub level: i32,
        pub source_lang: String,
        pub target_lang: String,
        pub vocabulary: Vec<PlanWord>,
    }

    pub struct PlanWord {
        pub word: String,
        pub translation: String,
        #[serde(default)]
        pub pronunciation: Option<String>,
    }

    pub struct PracticeQuestion {
        /// The plan's free `type` string.
        pub kind: String,
        pub prompt: String,
        /// Present on a multiple-choice question: answer with `practiceChoose`.
        pub options: Option<Vec<String>>,
        pub hint: Option<String>,
        /// Graded by the learner after seeing the model answer.
        pub self_graded: bool,
    }

    /// The last answer's result. `outcome` is null while a self-graded
    /// question waits for the learner's grade.
    pub struct AnswerResult {
        pub outcome: Option<PracticeOutcome>,
        pub expected: String,
        pub explanation: String,
    }

    /// An NPC conversation's request, held between turns.
    pub struct DialogueConfig {
        pub npc: Npc,
        #[serde(default)]
        pub scene: Option<String>,
        pub source_lang: String,
        pub target_lang: String,
        pub level: i32,
        #[serde(default)]
        pub window: Vec<WindowEntry>,
        // Defaulted on the Kotlin side too, so an existing call site compiles.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        #[uniffi(default)]
        pub languages: Vec<NpcLanguage>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[uniffi(default)]
        pub pedagogy: Option<Pedagogy>,
    }

    pub struct WindowEntry {
        pub speaker: Speaker,
        pub text: String,
    }

    pub struct DialogueSnapshot {
        pub phase: DialoguePhase,
        pub npc: Npc,
        pub scene: Option<String>,
        pub level: i32,
        pub window: Vec<WindowEntry>,
        /// A graph dialogue's story; null for a free-form one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[uniffi(default)]
        pub graph: Option<GraphSnapshot>,
    }

    pub struct EmbedStatus {
        pub stream: StreamState,
        pub cursor: Option<String>,
        pub needs_token: bool,
        pub closed: bool,
    }
}
