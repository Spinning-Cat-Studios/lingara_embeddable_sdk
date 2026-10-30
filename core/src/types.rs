//! Every type a binding marshals (D3, D10), in one module: the commands,
//! their outcomes, the events, the snapshots, and (re-exported) the config
//! and the error.
//!
//! Plain owned data only: no lifetimes, generics, trait objects, durations
//! or library types. Every enum whose variants carry data is internally
//! tagged by `type`, every fieldless enum is a `snake_case` string, and
//! unknown fields are ignored, so a wrapper one minor ahead still talks to
//! this core. A variant or field is only ever added within a minor version.

pub(crate) mod secret;

use serde::{Deserialize, Serialize};

pub use crate::config::{EmbedConfig, Header, TokenSourceConfig};
pub use crate::error::{EmbedError, ErrorKind};
pub use crate::secret::Secret;

/// The ids share one counter, from 1 to at most `i64::MAX`, so a Godot int
/// or a JVM `Long` holds every one, and no two kinds ever collide.
pub type RequestId = u64;
pub type SessionId = u64;
pub type DialogueId = u64;

#[cfg(feature = "schema")]
use schemars::JsonSchema;

macro_rules! data {
    ($($item:item)*) => { $(
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        #[cfg_attr(feature = "schema", derive(JsonSchema))]
        $item
    )* };
}

macro_rules! word {
    ($($item:item)*) => { $(
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        #[cfg_attr(feature = "schema", derive(JsonSchema))]
        #[serde(rename_all = "snake_case")]
        #[non_exhaustive]
        $item
    )* };
}

// After the two macros, which it uses (ADR 28.10.26f: the dialogue's types).
mod dialogue_graph;
pub use dialogue_graph::*;

data! {
    /// Every command, as one serialisable enum (`execute`). The flat methods
    /// on `Embed` are thin wrappers over it.
    #[serde(tag = "type", rename_all = "snake_case")]
    #[non_exhaustive]
    pub enum Command {
        ProvideToken { token: Secret, expires_at: String },
        ProvideTokenFor { token: Secret, expires_in_secs: u32 },
        TokenUnavailable { reason: String },
        SetTokenEndpoint { url: String, #[serde(default)] headers: Vec<Header> },
        StreamPause,
        StreamResume,
        SendContext { context: WorldContext },
        PracticeStart { source: PracticeSource },
        PracticeSnapshot { session: SessionId },
        PracticeAnswer { session: SessionId, answer: String },
        PracticeChoose { session: SessionId, option: u32 },
        PracticeReveal { session: SessionId },
        PracticeSelfGrade { session: SessionId, correct: bool },
        PracticeNext { session: SessionId },
        PracticeFinish { session: SessionId },
        DialogueOpen { config: DialogueConfig },
        DialogueSay { dialogue: DialogueId, line: String },
        DialogueSetLevel { dialogue: DialogueId, level: u8 },
        DialogueSetScene { dialogue: DialogueId, #[serde(default)] scene: Option<String> },
        DialogueCancel { dialogue: DialogueId },
        DialogueClose { dialogue: DialogueId },
        DialogueSnapshot { dialogue: DialogueId },
        DialogueLoad { source: DialogueSource, session: GraphSession },
        DialogueSetFact { dialogue: DialogueId, key: String, value: FactValue },
        DialogueValidate { ir_json: String },
    }

    /// What `execute` answers. `Id` carries a `RequestId`, `SessionId` or `DialogueId`.
    #[serde(tag = "type", rename_all = "snake_case")]
    #[non_exhaustive]
    pub enum CommandOutcome {
        Done {},
        Id { id: u64 },
        Practice { snapshot: Option<PracticeSnapshot> },
        Dialogue { snapshot: Option<DialogueSnapshot> },
    }

    /// What the engine polls. `TokenNeeded` is served ahead of the queue.
    #[serde(tag = "type", rename_all = "snake_case")]
    #[non_exhaustive]
    pub enum EmbedEvent {
        TokenNeeded { request: RequestId, reason: TokenReason },
        Stream { state: StreamState, cursor: Option<String> },
        Lingara { event: LingaraEvent },
        WorldSent { request: RequestId, event_id: String, reaction: Option<Reaction> },
        Practice { session: SessionId, snapshot: PracticeSnapshot },
        NpcTurn { dialogue: DialogueId, request: RequestId, update: NpcTurnUpdate },
        CheckpointReached { dialogue: DialogueId, checkpoint: String, edge: Option<String>, via: CrossedVia },
        DialogueEnded { dialogue: DialogueId, outcome: DialogueOutcome, checkpoint: String, edge: Option<String> },
        /// A stored dialogue's revision `rev` is loaded; `CheckpointReached { via: start }` follows.
        DialogueLoaded { dialogue: DialogueId, dialogue_id: String, rev: u64 },
        /// A stored dialogue could not load; its id is gone.
        DialogueLoadFailed { dialogue: DialogueId, error: EmbedError },
        Error { error: EmbedError },
        Log { level: LogLevel, target: String, message: String },
        Closed { reason: CloseReason },
    }

    /// The stream's state (D5). `Live` means an event has arrived since the
    /// tail was created; a quiet open stream reads `Connecting`.
    #[serde(tag = "type", rename_all = "snake_case")]
    #[non_exhaustive]
    pub enum StreamState {
        Connecting,
        Live,
        Paused,
        Down { error: EmbedError, retry_in_ms: u64 },
    }

    #[serde(tag = "type", rename_all = "snake_case")]
    #[non_exhaustive]
    pub enum NpcTurnUpdate {
        Opened,
        Fragment { text: String },
        Notice { code: String, message: String },
        /// The full reply, uncut.
        Finished { text: String },
        Cancelled,
    }

    /// Where a practice session's plan comes from (D7).
    #[serde(tag = "type", rename_all = "snake_case")]
    #[non_exhaustive]
    pub enum PracticeSource {
        Request { topic: String, source_lang: String, target_lang: String, level: u8, #[serde(default)] tags: Vec<String> },
        Create { context: String, source_lang: String, target_lang: String, level: u8 },
        /// A plan the game holds. A demo plan must be started with `report: false`.
        Plan { plan_id: String, #[serde(default = "yes")] report: bool },
    }

    /// One event off the stream (D5). `data_json` is the event's raw `data`.
    pub struct LingaraEvent {
        pub id: String,
        pub event_type: String,
        pub created_at: String,
        pub data_json: String,
        pub detail: LingaraEventDetail,
    }

    #[serde(tag = "type", rename_all = "snake_case")]
    #[non_exhaustive]
    pub enum LingaraEventDetail {
        LessonPlanReady { plan_id: String, title: Option<String>, level: u8, source_lang: String, target_lang: String, complete: bool },
        LessonPlanFailed { plan_id: String, reason: String },
        /// Every other type, newer ones included: read `data_json`.
        Other,
    }

    /// The `202`'s `reaction` (D6).
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

    /// `world.context_changed` (D6), bounded as the inbound door is.
    pub struct WorldContext {
        pub scene: String,
        pub source_lang: String,
        pub target_lang: String,
        pub level: u8,
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
        pub set_index: u32,
        pub set_count: u32,
        pub question_index: u32,
        pub question_count: u32,
        pub question: Option<PracticeQuestion>,
        pub last_result: Option<AnswerResult>,
        pub failure: Option<EmbedError>,
    }

    pub struct PlanSummary {
        pub id: String,
        pub title: Option<String>,
        pub level: u8,
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
        /// Present on a multiple-choice question: answer with `practice_choose`.
        pub options: Option<Vec<String>>,
        pub hint: Option<String>,
        /// Graded by the learner after seeing the model answer (D7).
        pub self_graded: bool,
    }

    /// The last answer's result. `outcome` is `None` while a self-graded
    /// question waits for the learner's grade.
    pub struct AnswerResult {
        pub outcome: Option<PracticeOutcome>,
        pub expected: String,
        pub explanation: String,
    }

    pub struct WindowEntry {
        pub speaker: Speaker,
        pub text: String,
    }

    pub struct EmbedStatus {
        pub stream: StreamState,
        pub cursor: Option<String>,
        pub needs_token: bool,
        pub closed: bool,
    }
}

word! {
    pub enum TokenReason { Initial, Expiring, Rejected }

    #[derive(PartialOrd, Ord)]
    pub enum LogLevel { Error, Warn, Info, Debug, Trace }

    pub enum CloseReason { Requested, Internal }

    pub enum PracticePhase {
        Requesting, Generating, Creating, Loading, Unanswered, AwaitingSelfGrade, Reviewed, SetComplete,
        Reporting, Reported, ReportFailed, Failed, Ended,
    }

    /// B2's six outcomes.
    pub enum PracticeOutcome { Correct, Incorrect, Revealed, SelfCorrect, SelfIncorrect, Skipped }

    /// `Ended` once a graph dialogue crosses an edge to an outcome (ADR
    /// 28.10.26f D4); `Loading` while a stored one is fetched (ADR 28.10.26i D2).
    pub enum DialoguePhase { Idle, Sending, Streaming, Ended, Loading }

    pub enum Speaker { Player, Npc }
}

fn yes() -> bool {
    true
}
