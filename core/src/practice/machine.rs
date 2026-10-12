//! The practice session as a pure state machine (D7): a plan and the host's
//! actions in, a snapshot out, no I/O. The registry in `mod.rs` does the
//! plan read and the report around it.
//!
//! `unanswered → reviewed → set_complete → reporting` is the shape of
//! Lingara's own apps' exercise session, restated here, not shared.

use crate::api::{Plan, Question};
use crate::error::{EmbedError, ErrorKind};
use crate::practice::grading;
use crate::types::{AnswerResult, PracticeOutcome, PracticePhase as Phase};

/// What the caller must do after a transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum After {
    Nothing,
    /// Send the one report: the session has entered `Reporting`.
    Report,
}

#[derive(Debug, Clone)]
pub(crate) struct Session {
    pub phase: Phase,
    pub report: bool,
    /// 16 random bytes in hex: the report's key is `pc:<key>`.
    pub key: String,
    pub plan: Option<Plan>,
    pub set: usize,
    pub question: usize,
    /// One slot per question, in plan order.
    pub outcomes: Vec<Vec<Option<PracticeOutcome>>>,
    pub last_result: Option<AnswerResult>,
    pub failure: Option<EmbedError>,
    pub completed: bool,
}

fn wrong_phase(phase: Phase, action: &str) -> EmbedError {
    EmbedError::new(ErrorKind::InvalidState, format!("cannot {action} in phase {phase:?}"))
}

impl Session {
    pub fn new(phase: Phase, report: bool, key: String) -> Self {
        Self { phase, report, key, plan: None, set: 0, question: 0, outcomes: Vec::new(), last_result: None, failure: None, completed: false }
    }

    /// The plan has arrived. Sets with no question are dropped; a plan with
    /// none at all cannot be reported, so it fails here.
    pub fn load(&mut self, mut plan: Plan) {
        plan.sets.retain(|s| !s.questions.is_empty());
        if plan.sets.is_empty() {
            return self.fail(EmbedError::invalid("the plan has no question to practise"));
        }
        self.outcomes = plan.sets.iter().map(|s| vec![None; s.questions.len()]).collect();
        self.plan = Some(plan);
        self.phase = Phase::Unanswered;
    }

    pub fn fail(&mut self, error: EmbedError) {
        self.failure = Some(error);
        self.phase = Phase::Failed;
    }

    pub(crate) fn current(&self) -> Option<&Question> {
        self.plan.as_ref()?.sets.get(self.set)?.questions.get(self.question)
    }

    fn require(&self, phase: Phase, action: &str) -> Result<Question, EmbedError> {
        match self.current() {
            Some(q) if self.phase == phase => Ok(q.clone()),
            _ => Err(wrong_phase(self.phase, action)),
        }
    }

    fn review(&mut self, q: &Question, outcome: PracticeOutcome) {
        self.outcomes[self.set][self.question] = Some(outcome);
        self.last_result = Some(AnswerResult { outcome: Some(outcome), expected: q.answer.clone(), explanation: q.explanation.clone() });
        self.phase = Phase::Reviewed;
    }

    /// A typed answer, or an option's text. A self-graded type waits for
    /// the learner's own grade, showing the model answer.
    pub fn answer(&mut self, text: &str) -> Result<(), EmbedError> {
        let q = self.require(Phase::Unanswered, "answer")?;
        if grading::is_self_graded(&q.kind) {
            self.last_result = Some(AnswerResult { outcome: None, expected: q.answer.clone(), explanation: q.explanation.clone() });
            self.phase = Phase::AwaitingSelfGrade;
            return Ok(());
        }
        let outcome = if grading::matches(text, &q.answer) { PracticeOutcome::Correct } else { PracticeOutcome::Incorrect };
        self.review(&q, outcome);
        Ok(())
    }

    /// The 0-based index into a multiple-choice question's options.
    pub fn choose(&mut self, option: u32) -> Result<(), EmbedError> {
        let q = self.require(Phase::Unanswered, "choose")?;
        let options = q.options.as_deref().ok_or_else(|| EmbedError::invalid("this question has no options: answer it with text"))?;
        let chosen = usize::try_from(option).ok().and_then(|i| options.get(i));
        let chosen = chosen.ok_or_else(|| EmbedError::invalid(format!("option {option} is out of range (0–{})", options.len().saturating_sub(1))))?;
        let outcome = if grading::matches(chosen, &q.answer) { PracticeOutcome::Correct } else { PracticeOutcome::Incorrect };
        self.review(&q, outcome);
        Ok(())
    }

    pub fn reveal(&mut self) -> Result<(), EmbedError> {
        let q = self.require(Phase::Unanswered, "reveal")?;
        self.review(&q, PracticeOutcome::Revealed);
        Ok(())
    }

    pub fn self_grade(&mut self, correct: bool) -> Result<(), EmbedError> {
        let q = self.require(Phase::AwaitingSelfGrade, "self-grade")?;
        self.review(&q, if correct { PracticeOutcome::SelfCorrect } else { PracticeOutcome::SelfIncorrect });
        Ok(())
    }

    pub fn next(&mut self) -> Result<After, EmbedError> {
        let sets = self.outcomes.len();
        match self.phase {
            Phase::Reviewed if self.question + 1 < self.outcomes[self.set].len() => self.question += 1,
            Phase::Reviewed => {
                self.phase = Phase::SetComplete;
                return Ok(After::Nothing);
            }
            Phase::SetComplete if self.set + 1 < sets => (self.set, self.question) = (self.set + 1, 0),
            Phase::SetComplete => return Ok(self.end(true)),
            phase => return Err(wrong_phase(phase, "move on")),
        }
        self.phase = Phase::Unanswered;
        self.last_result = None;
        Ok(After::Nothing)
    }

    /// Ends early: every question not yet graded is `skipped`. After a
    /// retryable report failure, sends the same report again.
    pub fn finish(&mut self) -> Result<After, EmbedError> {
        match self.phase {
            Phase::Unanswered | Phase::AwaitingSelfGrade | Phase::Reviewed | Phase::SetComplete => {
                let complete = self.outcomes.iter().flatten().all(Option::is_some);
                self.outcomes.iter_mut().flatten().filter(|o| o.is_none()).for_each(|o| *o = Some(PracticeOutcome::Skipped));
                Ok(self.end(complete))
            }
            Phase::ReportFailed if self.failure.as_ref().is_some_and(|f| f.retryable) => {
                self.phase = Phase::Reporting;
                Ok(After::Report)
            }
            phase => Err(wrong_phase(phase, "finish")),
        }
    }

    fn end(&mut self, completed: bool) -> After {
        self.completed = completed;
        if self.report {
            self.phase = Phase::Reporting;
            After::Report
        } else {
            self.phase = Phase::Ended;
            After::Nothing
        }
    }

    pub fn reported(&mut self, result: Result<(), EmbedError>) {
        match result {
            Ok(()) => (self.phase, self.failure) = (Phase::Reported, None),
            Err(e) => (self.phase, self.failure) = (Phase::ReportFailed, Some(e)),
        }
    }
}
