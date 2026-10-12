//! What the host sees of a session (D7): its phase, the plan's summary, the
//! position, the current question (never its answer), and the last result.

use crate::practice::grading;
use crate::practice::machine::Session;
use crate::types::{PracticePhase as Phase, PracticeQuestion, PracticeSnapshot};

impl Session {
    pub fn snapshot(&self) -> PracticeSnapshot {
        let to_u32 = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
        let question = self.current().filter(|_| matches!(self.phase, Phase::Unanswered | Phase::AwaitingSelfGrade | Phase::Reviewed));
        let question = question.map(|q| PracticeQuestion {
            kind: q.kind.clone(),
            prompt: q.prompt.clone(),
            options: q.options.clone(),
            hint: q.hint.clone(),
            self_graded: grading::is_self_graded(&q.kind),
        });
        PracticeSnapshot {
            phase: self.phase,
            plan: self.plan.as_ref().map(|p| p.summary.clone()),
            set_index: to_u32(self.set),
            set_count: to_u32(self.outcomes.len()),
            question_index: to_u32(self.question),
            question_count: to_u32(self.outcomes.get(self.set).map_or(0, Vec::len)),
            question,
            last_result: self.last_result.clone(),
            failure: self.failure.clone(),
        }
    }
}
