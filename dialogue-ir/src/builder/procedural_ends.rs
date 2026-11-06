//! When a procedural run stops (ADR 3.11.26j D8), over the crate's own
//! `Termination`. Both limits are required: the spec has no endless run.

use crate::limits::ABANDONED;
use crate::procedural::{StallRule, SuccessRule, Termination as SpecTermination};

/// When a run stops: the two limits, the success goal, the stall rule and
/// what a limit ends on.
pub struct Termination(SpecTermination);

impl Termination {
    pub fn new(max_turns: u16, max_beats: u16) -> Self {
        Termination(SpecTermination { max_turns, max_beats, success: None, stall: StallRule::default(), on_limit: ABANDONED.into() })
    }

    /// When `goal`'s predicate holds, the run ends on `ending`.
    pub fn success(mut self, goal: &str, ending: &str) -> Self {
        self.0.success = Some(SuccessRule { goal: goal.into(), ending: ending.into() });
        self
    }

    /// `turns` turns with no crossing end the run on `ending`.
    pub fn stall(mut self, turns: u8, ending: &str) -> Self {
        self.0.stall = StallRule { turns, ending: ending.into() };
        self
    }

    /// What either limit ends on; never called, `abandoned`.
    pub fn on_limit(mut self, ending: &str) -> Self {
        self.0.on_limit = ending.into();
        self
    }

    pub(super) fn into_spec(self) -> SpecTermination {
        self.0
    }
}
