//! Practice (D8): `StartPractice` starts a K1 session on an entity, which
//! then holds `PracticeSession`, K1's snapshot as it is. K1 grades and
//! sends the one report; the plugin forwards the learner's actions and
//! writes `PracticeFinished` once, at the first terminal phase.

use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::lifecycle::Remove;
use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::ecs::observer::On;
use bevy::ecs::system::{Commands, Query, Res, ResMut};
use bevy::ecs::world::World;
use lingara_embed::{Command, CommandOutcome, EmbedError, ErrorKind, PracticePhase, PracticeSnapshot, SessionId};

use crate::core::{Lingara, id_of};
use crate::frame::{Link, Routes};
use crate::messages::{LingaraError, PracticeFinished, PracticeInput, StartPractice};

/// One learner action: each is one K1 call, answered with the new snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PracticeAction {
    /// A typed answer, or an option's text.
    Answer(String),
    /// The 0-based option of a multiple-choice question.
    Choose(u32),
    Reveal,
    /// The learner's own grade of a self-graded question.
    SelfGrade(bool),
    Next,
    /// Ends the session now, reporting what was done. On a retryable
    /// `report_failed`, it sends the report again under the same key.
    Finish,
}

impl PracticeAction {
    fn command(&self, session: SessionId) -> Command {
        match self.clone() {
            PracticeAction::Answer(answer) => Command::PracticeAnswer { session, answer },
            PracticeAction::Choose(option) => Command::PracticeChoose { session, option },
            PracticeAction::Reveal => Command::PracticeReveal { session },
            PracticeAction::SelfGrade(correct) => Command::PracticeSelfGrade { session, correct },
            PracticeAction::Next => Command::PracticeNext { session },
            PracticeAction::Finish => Command::PracticeFinish { session },
        }
    }
}

/// A practice session. Only the plugin inserts one; the game draws it, and
/// removes it when done (a session left mid-way is finished, and reports).
#[derive(Component, Debug, Clone, PartialEq)]
pub struct PracticeSession {
    id: SessionId,
    snapshot: PracticeSnapshot,
    finished: bool,
    detached: bool,
}

impl PracticeSession {
    /// The `SessionId`, for `Lingara::core()`.
    pub fn id(&self) -> SessionId {
        self.id
    }

    pub fn snapshot(&self) -> &PracticeSnapshot {
        &self.snapshot
    }

    /// The handle closed under it: it keeps its last snapshot and goes no further.
    pub fn is_detached(&self) -> bool {
        self.detached
    }

    /// `reported`, `ended`, `failed`, or a `report_failed` that cannot be retried.
    pub fn is_terminal(&self) -> bool {
        let s = &self.snapshot;
        match s.phase {
            PracticePhase::Reported | PracticePhase::Ended | PracticePhase::Failed => true,
            PracticePhase::ReportFailed => !s.failure.as_ref().is_some_and(|f| f.retryable),
            _ => false,
        }
    }

    /// Takes `snapshot`; true the first time the session is terminal.
    fn update(&mut self, snapshot: PracticeSnapshot) -> bool {
        self.snapshot = snapshot;
        let first = self.is_terminal() && !self.finished;
        self.finished |= first;
        first
    }

    /// What a removal owes K1: `practice_finish` while a loaded session is
    /// mid-way, or its report can be retried.
    fn owes_finish(&self) -> bool {
        let mid_way = matches!(
            self.snapshot.phase,
            PracticePhase::Unanswered | PracticePhase::AwaitingSelfGrade | PracticePhase::Reviewed | PracticePhase::SetComplete
        );
        !self.detached && (mid_way || (self.snapshot.phase == PracticePhase::ReportFailed && !self.is_terminal()))
    }
}

pub(crate) fn start(mut starts: MessageReader<StartPractice>, sessions: Query<&PracticeSession>, mut commands: Commands, mut link: Link) {
    for StartPractice { entity, source } in starts.read() {
        let old = sessions.get(*entity).ok();
        if old.is_some_and(|s| !s.is_terminal() && !s.detached) {
            link.fail(EmbedError::new(ErrorKind::Busy, "this entity's practice session is not finished"), Some(*entity));
            continue;
        }
        let Some(id) = link.run(Command::PracticeStart { source: source.clone() }, Some(*entity)).and_then(|o| id_of(o).ok()) else { continue };
        let Some(CommandOutcome::Practice { snapshot: Some(snapshot) }) = link.run(Command::PracticeSnapshot { session: id }, Some(*entity)) else { continue };
        if let Some(old) = old {
            link.routes.sessions.remove(&old.id);
        }
        link.routes.sessions.insert(id, *entity);
        let session = PracticeSession { id, snapshot, finished: false, detached: false };
        if let Ok(mut target) = commands.get_entity(*entity) {
            target.insert(session);
        }
    }
}

pub(crate) fn input(mut inputs: MessageReader<PracticeInput>, mut sessions: Query<&mut PracticeSession>, mut finished: MessageWriter<PracticeFinished>, mut link: Link) {
    for PracticeInput { entity, action } in inputs.read() {
        let Ok(mut session) = sessions.get_mut(*entity) else {
            link.fail(EmbedError::new(ErrorKind::UnknownId, "this entity holds no PracticeSession"), Some(*entity));
            continue;
        };
        if session.detached {
            link.fail(crate::core::closed(), Some(*entity));
            continue;
        }
        if let Some(CommandOutcome::Practice { snapshot: Some(snapshot) }) = link.run(action.command(session.id), Some(*entity))
            && session.update(snapshot)
        {
            finished.write(PracticeFinished { entity: *entity, snapshot: session.snapshot.clone() });
        }
    }
}

/// An I/O-driven K1 `Practice` event: the plan loaded, the report sent.
pub(crate) fn on_snapshot(world: &mut World, session: SessionId, snapshot: PracticeSnapshot) {
    let Some(&entity) = world.resource::<Routes>().sessions.get(&session) else { return };
    let Some(mut held) = world.get_mut::<PracticeSession>(entity) else { return };
    if held.update(snapshot) {
        let snapshot = held.snapshot.clone();
        world.write_message(PracticeFinished { entity, snapshot });
    }
}

/// Removing the component finishes a session left mid-way (D8).
pub(crate) fn on_remove(event: On<Remove<PracticeSession>>, sessions: Query<&PracticeSession>, lingara: Option<Res<Lingara>>, mut routes: ResMut<Routes>) {
    let Ok(session) = sessions.get(event.event().entity) else { return };
    routes.sessions.remove(&session.id);
    if let Some(lingara) = lingara.filter(|_| session.owes_finish()) {
        let _ = lingara.run(Command::PracticeFinish { session: session.id });
    }
}

/// The handle closed: every session not yet terminal is detached, and the
/// game hears which.
pub(crate) fn detach_all(world: &mut World) {
    let held: Vec<(SessionId, Entity)> = world.resource_mut::<Routes>().sessions.drain().collect();
    for (_, entity) in held {
        let Some(mut session) = world.get_mut::<PracticeSession>(entity).filter(|s| !s.is_terminal()) else { continue };
        session.detached = true;
        let error = EmbedError::new(ErrorKind::Closed, "the Lingara handle closed under this practice session");
        world.write_message(LingaraError { error, entity: Some(entity) });
    }
}
