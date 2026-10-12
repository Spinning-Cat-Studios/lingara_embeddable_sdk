//! The practice session (D7): the registry of sessions, the plan events
//! they wait on, and the host's commands. `machine.rs` is the pure state
//! machine, `grading.rs` its grading, `source.rs` its I/O and `report.rs`
//! its one report.

pub(crate) mod grading;
pub(crate) mod machine;
pub(crate) mod report;
mod snapshot;
mod source;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::sync::oneshot;

use crate::commands::Dispatch;
use crate::embed::Embed;
use crate::error::{EmbedError, ErrorKind};
use crate::ids::random_key;
use crate::practice::machine::{After, Session};
use crate::runtime;
use crate::sync::lock;
use crate::types::{Command, CommandOutcome, LingaraEvent, LingaraEventDetail, PracticePhase, PracticeSnapshot, PracticeSource, SessionId};

/// What the stream says about a plan being made.
pub(crate) enum PlanSignal {
    Ready,
    Failed(String),
}

#[derive(Default)]
pub(crate) struct Practice {
    sessions: Mutex<HashMap<SessionId, Session>>,
    waits: Mutex<HashMap<String, Vec<oneshot::Sender<PlanSignal>>>>,
}

impl Practice {
    /// Runs `f` on one session under its lock; never held across an await.
    pub fn with<R>(&self, id: SessionId, f: impl FnOnce(&mut Session) -> R) -> Result<R, EmbedError> {
        let mut sessions = lock(&self.sessions);
        let session = sessions.get_mut(&id).ok_or_else(|| EmbedError::new(ErrorKind::UnknownId, format!("no practice session {id}")))?;
        Ok(f(session))
    }

    /// A signal for `plan_id`'s `ready` or `failed`, registered before the
    /// stream is turned on.
    pub fn watch(&self, plan_id: &str) -> oneshot::Receiver<PlanSignal> {
        let (tx, rx) = oneshot::channel();
        lock(&self.waits).entry(plan_id.to_owned()).or_default().push(tx);
        rx
    }

    /// The stream's plan events, to every session waiting on that plan.
    pub fn on_event(&self, event: &LingaraEvent) {
        let (plan_id, failed) = match &event.detail {
            LingaraEventDetail::LessonPlanReady { plan_id, .. } => (plan_id, None),
            LingaraEventDetail::LessonPlanFailed { plan_id, reason } => (plan_id, Some(reason)),
            _ => return,
        };
        for tx in lock(&self.waits).remove(plan_id).unwrap_or_default() {
            let _ = tx.send(failed.map_or(PlanSignal::Ready, |reason| PlanSignal::Failed(reason.clone())));
        }
    }
}

impl Embed {
    /// Starts a session; its plan arrives as a `Practice` event. Sessions
    /// from `Request` and `Create` always report; a `Plan` session reports
    /// unless started with `report: false`, which a demo plan needs.
    pub fn practice_start(&self, source: PracticeSource) -> Result<SessionId, EmbedError> {
        self.shared.live()?;
        source::check(&source)?;
        let (phase, report) = match &source {
            PracticeSource::Request { .. } => (PracticePhase::Requesting, true),
            PracticeSource::Create { .. } => (PracticePhase::Creating, true),
            PracticeSource::Plan { report, .. } => (PracticePhase::Loading, *report),
        };
        let id = self.shared.ids.next();
        lock(&self.shared.practice.sessions).insert(id, Session::new(phase, report, random_key()));
        runtime::spawn(&self.shared, source::run(Arc::clone(&self.shared), id, source));
        Ok(id)
    }

    pub fn practice_snapshot(&self, session: SessionId) -> Option<PracticeSnapshot> {
        self.shared.practice.with(session, |s| s.snapshot()).ok()
    }

    /// A typed answer, or an option's text.
    pub fn practice_answer(&self, session: SessionId, answer: &str) -> Result<PracticeSnapshot, EmbedError> {
        self.act(session, |s| s.answer(answer).map(|()| After::Nothing))
    }

    /// The 0-based option of a multiple-choice question.
    pub fn practice_choose(&self, session: SessionId, option: u32) -> Result<PracticeSnapshot, EmbedError> {
        self.act(session, |s| s.choose(option).map(|()| After::Nothing))
    }

    pub fn practice_reveal(&self, session: SessionId) -> Result<PracticeSnapshot, EmbedError> {
        self.act(session, |s| s.reveal().map(|()| After::Nothing))
    }

    /// The learner's own grade of a self-graded question.
    pub fn practice_self_grade(&self, session: SessionId, correct: bool) -> Result<PracticeSnapshot, EmbedError> {
        self.act(session, |s| s.self_grade(correct).map(|()| After::Nothing))
    }

    pub fn practice_next(&self, session: SessionId) -> Result<PracticeSnapshot, EmbedError> {
        self.act(session, Session::next)
    }

    /// Ends the session now, reporting what was done. Call it when the game
    /// quits mid-session: a killed game loses the session's report.
    pub fn practice_finish(&self, session: SessionId) -> Result<PracticeSnapshot, EmbedError> {
        self.act(session, Session::finish)
    }

    /// One synchronous transition; a session that enters `Reporting` sends
    /// its report on the I/O thread.
    fn act(&self, session: SessionId, step: impl FnOnce(&mut Session) -> Result<After, EmbedError>) -> Result<PracticeSnapshot, EmbedError> {
        self.shared.live()?;
        let (after, snapshot) = self.shared.practice.with(session, |s| step(s).map(|after| (after, s.snapshot())))??;
        if after == After::Report {
            runtime::spawn(&self.shared, source::send_report(Arc::clone(&self.shared), session));
        }
        Ok(snapshot)
    }

    pub(crate) fn exec_practice(&self, command: Command) -> Dispatch {
        let practice = |r: Result<PracticeSnapshot, EmbedError>| r.map(|s| CommandOutcome::Practice { snapshot: Some(s) });
        Ok(match command {
            Command::PracticeStart { source } => self.practice_start(source).map(|id| CommandOutcome::Id { id }),
            Command::PracticeSnapshot { session } => self.shared.live().map(|()| CommandOutcome::Practice { snapshot: self.practice_snapshot(session) }),
            Command::PracticeAnswer { session, answer } => practice(self.practice_answer(session, &answer)),
            Command::PracticeChoose { session, option } => practice(self.practice_choose(session, option)),
            Command::PracticeReveal { session } => practice(self.practice_reveal(session)),
            Command::PracticeSelfGrade { session, correct } => practice(self.practice_self_grade(session, correct)),
            Command::PracticeNext { session } => practice(self.practice_next(session)),
            Command::PracticeFinish { session } => practice(self.practice_finish(session)),
            other => return Err(other),
        })
    }
}
