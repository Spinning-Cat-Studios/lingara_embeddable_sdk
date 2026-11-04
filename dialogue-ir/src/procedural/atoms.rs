//! Each atom kind's structure (ADR 3.11.26g §5): its parameter set, how many
//! checkpoints it expands to, its fragment exits, and the edge-id suffixes
//! expansion derives. The words are `wording.rs`'s.

use super::AtomKind;

/// What one kind takes and makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtomShape {
    /// Takes a `topic`.
    pub topic: bool,
    /// Takes a `goal` (Practise).
    pub goal: bool,
    /// Takes an `ending` (Close).
    pub ending: bool,
    /// Checkpoints one planned beat expands to.
    pub checkpoints: usize,
    /// Has exactly one done exit when planned (every kind but Close).
    pub done_exit: bool,
    /// The kind's own detour cap, before the stance's (§8 W10).
    pub max_detours: usize,
}

/// §5's table.
pub fn shape(kind: AtomKind) -> AtomShape {
    let base = AtomShape { topic: false, goal: false, ending: false, checkpoints: 1, done_exit: true, max_detours: 2 };
    match kind {
        AtomKind::Open => base,
        AtomKind::Elicit | AtomKind::Reveal => AtomShape { topic: true, ..base },
        AtomKind::Practise => AtomShape { topic: true, goal: true, checkpoints: 2, max_detours: 1, ..base },
        AtomKind::Close => AtomShape { ending: true, done_exit: false, max_detours: 0, ..base },
    }
}

/// Practise's second checkpoint: `‹beat›_again`. No beat id may end with it (W9).
pub(crate) const AGAIN: &str = "_again";
/// The k-th exit with a `when`: `‹beat›_x‹k›`.
pub(crate) const EXIT: &str = "_x";
/// Practise's predicate edge (no `when`) to the done exit's target.
pub(crate) const OK: &str = "_ok";
/// Practise's loop, `‹beat›` → `‹beat›_again`.
pub(crate) const TRY: &str = "_try";
/// Practise's loop back, `‹beat›_again` → `‹beat›`.
pub(crate) const RETRY: &str = "_retry";
/// Close's one edge, to its ending.
pub(crate) const END: &str = "_end";
