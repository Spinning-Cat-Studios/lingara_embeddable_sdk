//! The core's one port onto Lingara (D10). Every task talks to `dyn Api`,
//! never to the library, and sees only this crate's own plain types; the
//! adapter in `library.rs` is the one file that names the library's models.
//! So a library regen changes no binding, and the unit tests drive the core
//! through a fake under paused time with no network at all.

use serde_json::Value;

use crate::error::EmbedError;
use crate::handoff::BoxFuture;
use crate::types::{LingaraEvent, PlanSummary, Reaction};

/// A plan as the practice session needs it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Plan {
    pub summary: PlanSummary,
    /// `generating`, `partial` or `complete`.
    pub status: String,
    pub sets: Vec<PlanSet>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PlanSet {
    /// The plan's own `number`, which a report names: never a position.
    pub number: u8,
    pub questions: Vec<Question>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Question {
    pub kind: String,
    pub prompt: String,
    pub options: Option<Vec<String>>,
    pub answer: String,
    pub explanation: String,
    pub hint: Option<String>,
}

/// The `202` of `POST /v1/events`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Accepted {
    pub event_id: String,
    pub reaction: Option<Reaction>,
}

/// A lesson-plan stream's frames, the ones the session acts on.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PlanFrame {
    Started(String),
    Result(Box<Plan>),
    /// `streamLessonPlan`'s `pending`: the plan is not ready to read.
    Pending,
    Other,
}

/// An NPC turn's frames. `done` ends the stream; `error` is its `Err`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TurnFrame {
    Delta(String),
    Notice { code: String, message: String },
    /// N2's judged crossing, before `done`: held, and applied only after it.
    Transition { edge: String, to: String },
    Other,
}

/// One open stream. `None` is its end; an `Err` is its last item.
pub(crate) trait Frames<T>: Send {
    fn next(&mut self) -> BoxFuture<'_, Option<Result<T, EmbedError>>>;
}

/// The events tail (CONTRACT K5a): ends only by raising.
pub(crate) trait Tail: Send {
    fn next(&mut self) -> BoxFuture<'_, Option<Result<LingaraEvent, EmbedError>>>;
    fn cursor(&self) -> Option<String>;
}

/// `exportDialogue`'s answer (ADR 28.10.26i D3, D6): the canonical body as
/// text with its revision and `ETag`, or `NotModified` on a `304`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Exported {
    Body { rev: u64, etag: String, ir_json: String },
    NotModified { rev: u64, etag: String },
}

pub(crate) type PlanStream = Box<dyn Frames<PlanFrame>>;
pub(crate) type TurnStream = Box<dyn Frames<TurnFrame>>;

/// The seven operations a player kit calls. Request bodies are JSON in the
/// published schema's shape, validated locally before they get here.
pub(crate) trait Api: Send + Sync + 'static {
    fn tail(&self, cursor: Option<String>, types: Vec<String>) -> Box<dyn Tail>;
    /// `{type, data}`, under `key` on every retry.
    fn send_event(&self, body: Value, key: String) -> BoxFuture<'_, Result<Accepted, EmbedError>>;
    fn get_plan(&self, id: String) -> BoxFuture<'_, Result<Plan, EmbedError>>;
    fn create_plan(&self, body: Value) -> BoxFuture<'_, Result<PlanStream, EmbedError>>;
    fn stream_plan(&self, id: String) -> BoxFuture<'_, Result<PlanStream, EmbedError>>;
    fn dialogue_turn(&self, body: Value) -> BoxFuture<'_, Result<TurnStream, EmbedError>>;
    /// The active revision of a stored dialogue, by its lowercased id, with
    /// `If-None-Match` when the cache holds an `etag`.
    fn export_dialogue(&self, dialogue_id: String, if_none_match: Option<String>) -> BoxFuture<'_, Result<Exported, EmbedError>>;
}
