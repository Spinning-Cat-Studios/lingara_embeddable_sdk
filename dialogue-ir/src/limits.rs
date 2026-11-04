//! `dialogue.v1`'s frozen bounds, under the names Backend ADR 28.10.26e §10
//! gives them. Within v1 a bound never changes: a change is `dialogue.v2`.
//!
//! The vendored schema is the authority. Until Phase B vendors it, these are
//! held to N2 §10's table by review (ADR 28.10.26f AC23); from then on
//! `core/conformance`'s spec-limits test reads them against the file (AC17).

/// Every IR id: a checkpoint, an edge, an NPC, a fact key, `start`, `to`.
pub const MAX_ID_CHARS: usize = 48;
/// Every IR id, as published (`$defs/id`). [`crate::is_ir_id`] is its hand-written twin.
pub const ID_PATTERN: &str = "^[a-z][a-z0-9_]{0,47}$";
/// A dialogue's optional `title`.
pub const MAX_TITLE_CHARS: usize = 80;
/// A checkpoint's `goal`.
pub const MAX_GOAL_CHARS: usize = 240;
/// An edge's conversational condition (`when`).
pub const MAX_CONDITION_CHARS: usize = 200;
/// An edge's game condition (`predicate`), whose grammar this crate owns.
pub const MAX_PREDICATE_CHARS: usize = 200;
/// The most edges one checkpoint holds, and so the most one turn may offer.
pub const MAX_EDGES_PER_CHECKPOINT: usize = 8;
/// The most fact keys one checkpoint may disclose.
pub const MAX_DISCLOSE: usize = 16;
/// A string fact's value, and a string `default`.
pub const MAX_FACT_STRING_CHARS: usize = 120;
/// The most facts one dialogue declares.
pub const MAX_FACTS: usize = 32;
/// The most NPCs in a dialogue's cast.
pub const MAX_NPCS: usize = 8;
/// The most checkpoints in one dialogue.
pub const MAX_CHECKPOINTS: usize = 64;
/// The canonical form's UTF-8 bytes, `x-` keys included (`x-maxBytes`).
pub const MAX_IR_BYTES: usize = 131_072;
/// The reserved `to` values that end a dialogue instead of naming a checkpoint.
pub const OUTCOMES: [&str; 3] = ["success", "failure", "abandoned"];

/// An NPC's `name` (B3's bound, which the schema repeats).
pub const MAX_NPC_NAME_CHARS: usize = 32;
/// An NPC's `persona` (B3's bound).
pub const MAX_NPC_PERSONA_CHARS: usize = 120;
/// An NPC's language profile entries (N1, Backend ADR 28.10.26d).
pub const MAX_NPC_LANGUAGES: usize = 8;
/// A language profile's `code` (N1).
pub const MAX_LANGUAGE_CODE_CHARS: usize = 16;
/// A profile's `speaking` and `comprehension` levels run from 1 to 9 (N1).
pub const MIN_LEVEL: u8 = 1;
/// See [`MIN_LEVEL`].
pub const MAX_LEVEL: u8 = 9;

// dialogue.v2 (ADR 3.11.26g §1, §11.4). v1's constants above stay frozen and
// govern every v2 member they already bound; these are the whole new set.

/// The end that needs no declaration: the runtime reaches it on close.
pub const ABANDONED: &str = "abandoned";
/// The endings a v1 document is read as declaring (§2).
pub const V1_ENDINGS: [&str; 2] = ["success", "failure"];
/// `endings` holds 1 to this many.
pub const MAX_ENDINGS: usize = 8;
/// `procedural.disclosable` (= [`MAX_DISCLOSE`]).
pub const MAX_DISCLOSABLE: usize = 16;
/// `procedural.topics` (at least 1).
pub const MAX_TOPICS: usize = 16;
/// A topic's `discloses`.
pub const MAX_TOPIC_DISCLOSES: usize = 4;
/// `procedural.forbidden`.
pub const MAX_FORBIDDEN: usize = 8;
/// `procedural.goals`.
pub const MAX_GOALS: usize = 8;
/// `procedural.premise`.
pub const MAX_PREMISE_CHARS: usize = 240;
/// A topic's or a forbidden entry's `about`.
pub const MAX_ABOUT_CHARS: usize = 80;
/// A goal's `describe`.
pub const MAX_DESCRIBE_CHARS: usize = 60;
/// A beat's `angle`, the planner's only free text.
pub const MAX_ANGLE_CHARS: usize = 60;
/// A beat id.
pub const MAX_BEAT_ID_CHARS: usize = 24;
/// A beat's `exits`.
pub const MAX_EXITS_PER_BEAT: usize = 3;
/// `horizon.plan_depth` (at least 1).
pub const MAX_PLAN_DEPTH: u8 = 3;
/// `horizon.sketch_depth` (at least 0).
pub const MAX_SKETCH_DEPTH: u8 = 1;
/// `horizon.max_nodes` (at least `1 + plan_depth + sketch_depth`, and 3).
pub const MAX_FRONTIER_NODES: u8 = 16;
/// `termination.max_turns` (at least 1).
pub const MAX_TURNS: u16 = 200;
/// `termination.max_beats` (at least 1).
pub const MAX_BEATS: u16 = 64;
/// `termination.stall.turns`, lower bound.
pub const MIN_STALL_TURNS: u8 = 2;
/// `termination.stall.turns`, upper bound.
pub const MAX_STALL_TURNS: u8 = 20;

/// The deepest parenthesis nesting a predicate may hold. This crate's own
/// bound, not the schema's: no server ever evaluates a predicate.
pub const PREDICATE_MAX_DEPTH: usize = 8;
