//! Every bound the core checks locally, and its timeouts (D4–D8).
//!
//! The schema bounds are copied from the frozen API version's published
//! schemas, and `core/conformance`'s spec-limits test fails when one
//! differs, so a server-side change to a bound cannot ship silently. Every
//! constant is `pub`: a binding or a game may bound a text field before it
//! sends. Lengths are counted in Unicode scalar values, as the server counts.

// ── The inbound door (`world.context_changed`, `world.practice_requested`) ──

pub const SCENE_MAX_CHARS: usize = 160;
pub const TOPIC_MAX_CHARS: usize = 160;
pub const NPC_NAME_MAX_CHARS: usize = 32;
pub const NPC_PERSONA_MAX_CHARS: usize = 120;
pub const LEVEL_MIN: u8 = 1;
pub const LEVEL_MAX: u8 = 9;
pub const TAGS_MAX_ITEMS: usize = 8;
pub const TAG_MAX_CHARS: usize = 24;
/// A tag is `^[a-z0-9][a-z0-9._-]*$`.
pub const TAG_PATTERN: &str = "^[a-z0-9][a-z0-9._-]*$";

// ── The practice report (`world.practice_completed`) ──

/// B2's `items` `maxItems`. A longer plan drops `skipped` items first.
pub const MAX_PRACTICE_ITEMS: usize = 100;

/// The question types graded by the learner, after seeing the model answer:
/// the three Lingara's own apps send to a model, with no `/v1/` grading route.
pub const SELF_GRADED_TYPES: [&str; 3] = ["free_response", "phrase_translation", "conversational_response"];

// ── The NPC turn (`DialogueTurnRequest`) ──

pub const DIALOGUE_LINE_MAX_CHARS: usize = 500;
pub const DIALOGUE_HISTORY_MAX_ITEMS: usize = 12;
/// Each `history` entry, and the cut an NPC reply is stored at.
pub const DIALOGUE_TEXT_MAX_CHARS: usize = 500;
/// N1's `npc.languages`: at most this many, with unique codes.
pub const NPC_LANGUAGES_MAX_ITEMS: usize = 8;
/// Each `npc.languages[].code`.
pub const NPC_LANGUAGE_CODE_MAX_CHARS: usize = 16;

// ── The dialogue graph (`dialogue.v1`, ADR 28.10.26f D1) ──

/// The IR's frozen bounds, defined once in `lingara-dialogue-ir`.
pub use lingara_dialogue_ir::limits::*;

// ── Timeouts and defaults ──

pub const DEFAULT_QUEUE_CAPACITY: u32 = 1024;
pub const DEFAULT_TOKEN_WAIT_TIMEOUT_MS: u32 = 30_000;
/// Three missed 15 s heartbeats.
pub const DEFAULT_STREAM_IDLE_TIMEOUT_MS: u32 = 45_000;
pub const MIN_STREAM_IDLE_TIMEOUT_MS: u32 = 1_000;
/// A raised tail is restarted from its cursor after this long.
pub const STREAM_RESTART_MS: u64 = 60_000;
/// Event ids remembered to drop a resumed tail's repeats.
pub const DEDUP_WINDOW: usize = 256;
/// A plan being made is read this often while the stream is awaited.
pub const PLAN_POLL_MS: u64 = 30_000;
/// A plan still `generating` this long after the request is `Failed`.
pub const PLAN_WAIT_MS: u64 = 120_000;
/// A token lifetime on the device clock outside `(0, 1 h]` is taken as this.
pub const CLAMPED_LIFETIME_SECS: u64 = 300;
pub const MAX_TRUSTED_LIFETIME_SECS: u64 = 3_600;
/// The library's refresh margin: stale at `lifetime − min(60 s, lifetime / 2)`.
pub const STALE_MARGIN_SECS: u64 = 60;
/// A stored dialogue's cached revision is reused this long with no request,
/// and is the bound on how soon an activation reaches a new conversation
/// (ADR 28.10.26i D3).
pub const DIALOGUE_ACTIVE_TTL_MS: u64 = 300_000;
/// Stored dialogues a handle keeps parsed, the least recently used dropped first.
pub const DIALOGUE_CACHE_MAX_ENTRIES: usize = 32;
/// The only token a game binary may hold.
pub const EMBED_TOKEN_PREFIX: &str = "lgr_et_";
