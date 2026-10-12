//! The world and practice commands on `EmbedHandle` (ADR 1.10.26ae D2).
//! Each returns K1's snapshot in the same call, so a game shows the graded
//! answer in the frame it was given.

use lingara_embed as core;

use crate::convert::{fail, id_out, known, narrow, to_core};
use crate::handle::{EmbedHandle, Outcome};
use crate::types::{PracticeSnapshot, PracticeSource, WorldContext};

fn snapshot(result: Result<core::PracticeSnapshot, core::EmbedError>) -> Outcome<PracticeSnapshot> {
    known(&result.map_err(fail)?)
}

#[uniffi::export]
impl EmbedHandle {
    /// Sends `world.context_changed`; the request id its `WorldSent` echoes.
    pub fn send_context(&self, context: WorldContext) -> Outcome<i64> {
        let context: core::WorldContext = to_core("context", &context)?;
        id_out(self.embed.send_context(context).map_err(fail)?)
    }

    /// Starts a practice session; its id.
    pub fn practice_start(&self, source: PracticeSource) -> Outcome<i64> {
        let source: core::PracticeSource = to_core("source", &source)?;
        id_out(self.embed.practice_start(source).map_err(fail)?)
    }

    pub fn practice_snapshot(&self, session: i64) -> Outcome<Option<PracticeSnapshot>> {
        self.embed.practice_snapshot(narrow("session", session)?).as_ref().map(known).transpose()
    }

    pub fn practice_answer(&self, session: i64, answer: String) -> Outcome<PracticeSnapshot> {
        snapshot(self.embed.practice_answer(narrow("session", session)?, &answer))
    }

    pub fn practice_choose(&self, session: i64, option: i64) -> Outcome<PracticeSnapshot> {
        let option = narrow("option", option)?;
        snapshot(self.embed.practice_choose(narrow("session", session)?, option))
    }

    pub fn practice_reveal(&self, session: i64) -> Outcome<PracticeSnapshot> {
        snapshot(self.embed.practice_reveal(narrow("session", session)?))
    }

    pub fn practice_self_grade(&self, session: i64, correct: bool) -> Outcome<PracticeSnapshot> {
        snapshot(self.embed.practice_self_grade(narrow("session", session)?, correct))
    }

    pub fn practice_next(&self, session: i64) -> Outcome<PracticeSnapshot> {
        snapshot(self.embed.practice_next(narrow("session", session)?))
    }

    pub fn practice_finish(&self, session: i64) -> Outcome<PracticeSnapshot> {
        snapshot(self.embed.practice_finish(narrow("session", session)?))
    }
}
