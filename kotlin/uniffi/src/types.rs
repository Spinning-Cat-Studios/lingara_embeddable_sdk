//! The mirrors of K1's `types.rs` that cross to Kotlin (ADR 1.10.26ae D2):
//! field for field, with K1's own serde attributes, so `convert.rs` moves a
//! value between the two through `serde_json::Value` and K1's JSON shape is
//! the one contract. Three rules shape them:
//!
//! - No unsigned integer crosses: Kotlin mangles the JVM name of anything
//!   typed `UInt` or `ULong`, and Java cannot call it. K1's `u8` and `u16`
//!   are `i32` here, its `u32` and `u64` are `i64`, and a value out of K1's
//!   range is refused by K1's own deserialiser as `invalid_input`.
//! - Secrets are `SecretString`, which Kotlin lifts into `Secret`.
//! - The fields K1 defaults are `Option`s here, absent when `None`, so K1's
//!   `Default` stays the one source of every default.

use serde::{Deserialize, Serialize};

pub use crate::config::{EmbedConfig, Header, SecretString, TokenSourceConfig};
pub use crate::graph::{FactValue, GraphSession, GraphSnapshot, NpcLanguage};
pub use crate::records::*;

macro_rules! tagged {
    ($($item:item)*) => { $(
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Enum)]
        #[serde(tag = "type", rename_all = "snake_case")]
        $item
    )* };
}

macro_rules! word {
    ($($item:item)*) => { $(
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
        #[serde(rename_all = "snake_case")]
        $item
    )* };
}

tagged! {
    /// Every command, as one value: `LingaraEmbed.execute`.
    pub enum Command {
        ProvideToken { token: SecretString, expires_at: String },
        ProvideTokenFor { token: SecretString, expires_in_secs: i64 },
        TokenUnavailable { reason: String },
        SetTokenEndpoint { url: String, #[serde(default)] headers: Vec<Header> },
        StreamPause,
        StreamResume,
        SendContext { context: WorldContext },
        PracticeStart { source: PracticeSource },
        PracticeSnapshot { session: i64 },
        PracticeAnswer { session: i64, answer: String },
        PracticeChoose { session: i64, option: i64 },
        PracticeReveal { session: i64 },
        PracticeSelfGrade { session: i64, correct: bool },
        PracticeNext { session: i64 },
        PracticeFinish { session: i64 },
        DialogueOpen { config: DialogueConfig },
        DialogueSay { dialogue: i64, line: String },
        DialogueSetLevel { dialogue: i64, level: i32 },
        DialogueSetScene { dialogue: i64, #[serde(default)] scene: Option<String> },
        DialogueCancel { dialogue: i64 },
        DialogueClose { dialogue: i64 },
        DialogueSnapshot { dialogue: i64 },
        DialogueLoad { source: DialogueSource, session: GraphSession },
        DialogueSetFact { dialogue: i64, key: String, value: FactValue },
        DialogueValidate { ir_json: String },
    }

    /// What `execute` answers. `Id` carries a request, session or dialogue id.
    pub enum CommandOutcome {
        Done,
        Id { id: i64 },
        Practice { snapshot: Option<PracticeSnapshot> },
        Dialogue { snapshot: Option<DialogueSnapshot> },
    }

    /// What the game drains. `TokenNeeded` is served ahead of the queue.
    /// `Unknown` is a variant of a newer core: its `json` is K1's whole event.
    pub enum EmbedEvent {
        TokenNeeded { request: i64, reason: TokenReason },
        Stream { state: StreamState, cursor: Option<String> },
        Lingara { event: LingaraEvent },
        WorldSent { request: i64, event_id: String, reaction: Option<Reaction> },
        Practice { session: i64, snapshot: PracticeSnapshot },
        NpcTurn { dialogue: i64, request: i64, update: NpcTurnUpdate },
        CheckpointReached { dialogue: i64, checkpoint: String, edge: Option<String>, via: CrossedVia },
        DialogueEnded { dialogue: i64, outcome: DialogueOutcome, checkpoint: String, edge: Option<String> },
        /// A stored dialogue's revision `rev` is loaded (ADR 28.10.26i); `CheckpointReached` follows.
        DialogueLoaded { dialogue: i64, dialogue_id: String, rev: i64 },
        /// A stored dialogue could not load; its id is gone.
        DialogueLoadFailed { dialogue: i64, error: EmbedError },
        Error { error: EmbedError },
        Log { level: LogLevel, target: String, message: String },
        Closed { reason: CloseReason },
        #[serde(skip)]
        Unknown { type_tag: String, json: String },
    }

    /// The stream's state. A quiet open stream reads `Connecting`.
    pub enum StreamState {
        Connecting,
        Live,
        Paused,
        Down { error: EmbedError, retry_in_ms: i64 },
    }

    pub enum NpcTurnUpdate {
        Opened,
        Fragment { text: String },
        Notice { code: String, message: String },
        /// The full reply, uncut.
        Finished { text: String },
        Cancelled,
    }

    /// Where a practice session's plan comes from.
    pub enum PracticeSource {
        Request { topic: String, source_lang: String, target_lang: String, level: i32, #[serde(default)] tags: Vec<String> },
        Create { context: String, source_lang: String, target_lang: String, level: i32 },
        /// A plan the game holds. A demo plan must be started with `report: false`.
        Plan { plan_id: String, #[serde(default = "yes")] report: bool },
    }

    /// Where a graph dialogue comes from: a `dialogue.v1` document, as
    /// text, or a dialogue saved on Lingara by its store id (ADR 28.10.26i).
    pub enum DialogueSource {
        Ir { ir_json: String },
        Stored { dialogue_id: String },
    }

    pub enum LingaraEventDetail {
        LessonPlanReady { plan_id: String, title: Option<String>, level: i32, source_lang: String, target_lang: String, complete: bool },
        LessonPlanFailed { plan_id: String, reason: String },
        /// Every other type, newer ones included: read `data_json`.
        Other,
    }
}

word! {
    pub enum TokenReason { Initial, Expiring, Rejected }

    pub enum LogLevel { Error, Warn, Info, Debug, Trace }

    pub enum CloseReason { Requested, Internal }

    pub enum PracticePhase {
        Requesting, Generating, Creating, Loading, Unanswered, AwaitingSelfGrade, Reviewed, SetComplete,
        Reporting, Reported, ReportFailed, Failed, Ended,
    }

    pub enum PracticeOutcome { Correct, Incorrect, Revealed, SelfCorrect, SelfIncorrect, Skipped }

    pub enum DialoguePhase { Idle, Sending, Streaming, Ended, Loading }

    /// How a checkpoint was reached.
    pub enum CrossedVia { Start, Auto, Judge }

    /// The reserved names a story ends on.
    pub enum DialogueOutcome { Success, Failure, Abandoned }

    /// How much the NPC adapts its speech to the learner.
    pub enum Pedagogy { Subtle, None }

    pub enum Speaker { Player, Npc }

    /// The closed code every kit surfaces.
    pub enum ErrorKind {
        Config, InvalidInput, TokenUnavailable, TokenRejected, Forbidden, Billing, NotFound, RateLimited,
        Unavailable, Network, Busy, InvalidState, UnknownId, Closed, Internal,
    }
}

/// Every failure the core reports. `message` is English, for logs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct EmbedError {
    pub kind: ErrorKind,
    pub message: String,
    #[serde(default)]
    pub status: Option<i32>,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub retry_after_ms: Option<i64>,
    pub retryable: bool,
    /// The command this failure answers, when it answers one.
    #[serde(default)]
    pub request: Option<i64>,
}

/// What every fallible call throws: one variant, carrying the error record.
#[derive(Debug, Clone, PartialEq, uniffi::Error)]
pub enum EmbedException {
    Failed { error: EmbedError },
}

impl std::fmt::Display for EmbedException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let EmbedException::Failed { error } = self;
        write!(f, "{:?}: {}", error.kind, error.message)
    }
}

impl std::error::Error for EmbedException {}

fn yes() -> bool {
    true
}
