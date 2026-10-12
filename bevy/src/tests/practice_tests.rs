//! Practice (D8): the component follows K1's session.

use bevy::app::App;
use bevy::ecs::entity::Entity;
use lingara_embed::{Command, EmbedError, EmbedEvent, ErrorKind, PracticePhase, PracticeSnapshot, PracticeSource};

use super::fake_core::{Fake, app, read, snapshot, write};
use crate::config::LingaraConfig;
use crate::messages::*;
use crate::{PracticeAction, PracticeSession};

fn request() -> PracticeSource {
    PracticeSource::Request { topic: "ordering food".into(), source_lang: "en".into(), target_lang: "zh".into(), level: 2, tags: vec![] }
}

fn phase(app: &App, entity: Entity) -> PracticePhase {
    app.world().get::<PracticeSession>(entity).expect("a session").snapshot().phase
}

fn act(app: &mut App, entity: Entity, action: PracticeAction) {
    write(app, PracticeInput { entity, action });
    app.update();
}

fn report_failed(retryable: bool) -> PracticeSnapshot {
    let mut failure = EmbedError::new(ErrorKind::Unavailable, "down");
    failure.retryable = retryable;
    PracticeSnapshot { failure: Some(failure), ..snapshot(PracticePhase::ReportFailed) }
}

/// 1.10.26z AC8: the source reaches `practice_start` unchanged; a second
/// start on an unfinished session is `Busy`; each action is its K1 call and
/// its snapshot lands the same frame; an I/O snapshot replaces it; one
/// `PracticeFinished`.
#[test]
fn a_requested_practice_runs_to_finished() {
    let fake = Fake::default();
    let mut app = app(LingaraConfig::new(), &fake);
    let learner = app.world_mut().spawn_empty().id();
    write(&mut app, StartPractice { entity: learner, source: request() });
    app.update();
    assert_eq!(fake.sent("practice_start"), [Command::PracticeStart { source: request() }]);
    assert_eq!(phase(&app, learner), PracticePhase::Requesting);
    let session = app.world().get::<PracticeSession>(learner).unwrap().id();

    write(&mut app, StartPractice { entity: learner, source: request() });
    app.update();
    let errors = read::<LingaraError>(&mut app);
    assert_eq!(errors.iter().map(|e| (e.error.kind, e.entity)).collect::<Vec<_>>(), [(ErrorKind::Busy, Some(learner))]);
    assert_eq!(fake.sent("practice_start").len(), 1);

    fake.push(EmbedEvent::Practice { session, snapshot: snapshot(PracticePhase::Unanswered) });
    app.update();
    assert_eq!(phase(&app, learner), PracticePhase::Unanswered);
    fake.state().snapshots.extend([snapshot(PracticePhase::Reviewed), snapshot(PracticePhase::AwaitingSelfGrade)]);
    act(&mut app, learner, PracticeAction::Choose(1));
    assert_eq!(phase(&app, learner), PracticePhase::Reviewed);
    act(&mut app, learner, PracticeAction::Answer("饺子".into()));
    assert_eq!(phase(&app, learner), PracticePhase::AwaitingSelfGrade);
    fake.state().snapshots.extend([snapshot(PracticePhase::Reviewed), snapshot(PracticePhase::Reporting)]);
    act(&mut app, learner, PracticeAction::SelfGrade(true));
    act(&mut app, learner, PracticeAction::Next);
    act(&mut app, learner, PracticeAction::Reveal);
    let kinds: Vec<String> = fake.state().commands.iter().map(super::fake_core::kind).filter(|k| k.starts_with("practice_") && k != "practice_snapshot").collect();
    assert_eq!(kinds, ["practice_start", "practice_choose", "practice_answer", "practice_self_grade", "practice_next", "practice_reveal"]);
    assert!(read::<PracticeFinished>(&mut app).is_empty());

    fake.push(EmbedEvent::Practice { session, snapshot: snapshot(PracticePhase::Reported) });
    fake.push(EmbedEvent::Practice { session, snapshot: snapshot(PracticePhase::Reported) });
    app.update();
    let finished = read::<PracticeFinished>(&mut app);
    assert_eq!(finished, [PracticeFinished { entity: learner, snapshot: snapshot(PracticePhase::Reported) }]);

    write(&mut app, StartPractice { entity: learner, source: request() });
    app.update();
    assert_eq!(fake.sent("practice_start").len(), 2);
    assert_ne!(app.world().get::<PracticeSession>(learner).unwrap().id(), session);
}

/// 1.10.26z AC9: a retryable `report_failed` is not terminal until a
/// `Finish` reaches `reported`; removal finishes a session left mid-way and
/// nothing else.
#[test]
fn terminal_phases_and_removal_follow_k1() {
    let fake = Fake::default();
    let mut app = app(LingaraConfig::new(), &fake);
    let learner = app.world_mut().spawn_empty().id();
    write(&mut app, StartPractice { entity: learner, source: request() });
    app.update();
    let session = app.world().get::<PracticeSession>(learner).unwrap().id();
    fake.push(EmbedEvent::Practice { session, snapshot: report_failed(true) });
    app.update();
    assert!(read::<PracticeFinished>(&mut app).is_empty());
    fake.state().snapshots.push_back(snapshot(PracticePhase::Reported));
    act(&mut app, learner, PracticeAction::Finish);
    assert_eq!(read::<PracticeFinished>(&mut app).len(), 1);

    let removed_in = |phase: PracticeSnapshot| {
        let fake = Fake::default();
        let mut app = super::fake_core::app(LingaraConfig::new(), &fake);
        let learner = app.world_mut().spawn_empty().id();
        write(&mut app, StartPractice { entity: learner, source: request() });
        app.update();
        let session = app.world().get::<PracticeSession>(learner).unwrap().id();
        fake.push(EmbedEvent::Practice { session, snapshot: phase });
        app.update();
        app.world_mut().entity_mut(learner).remove::<PracticeSession>();
        app.update();
        fake.sent("practice_finish").len()
    };
    for mid_way in [PracticePhase::Unanswered, PracticePhase::AwaitingSelfGrade, PracticePhase::Reviewed, PracticePhase::SetComplete] {
        assert_eq!(removed_in(snapshot(mid_way)), 1, "{mid_way:?}");
    }
    assert_eq!(removed_in(report_failed(true)), 1);
    for quiet in [PracticePhase::Reporting, PracticePhase::Requesting, PracticePhase::Loading, PracticePhase::Reported, PracticePhase::Ended] {
        assert_eq!(removed_in(snapshot(quiet)), 0, "{quiet:?}");
    }
    assert_eq!(removed_in(report_failed(false)), 0);
}
