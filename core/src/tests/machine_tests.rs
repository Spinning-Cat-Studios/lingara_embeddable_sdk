use crate::api::{Plan, PlanSet, Question};
use crate::error::ErrorKind;
use crate::practice::machine::{After, Session};
use crate::types::{PlanSummary, PracticeOutcome as O, PracticePhase as P};

pub fn question(kind: &str, answer: &str, options: Option<&[&str]>) -> Question {
    Question {
        kind: kind.into(),
        prompt: format!("prompt for {answer}"),
        options: options.map(|o| o.iter().map(|s| (*s).to_owned()).collect()),
        answer: answer.into(),
        explanation: "because".into(),
        hint: None,
    }
}

pub fn plan(sets: Vec<(u8, Vec<Question>)>) -> Plan {
    let summary = PlanSummary { id: "8f0e2c1a-0000-4000-8000-000000000001".into(), title: Some("At the market".into()), level: 2, source_lang: "en".into(), target_lang: "zh".into(), vocabulary: vec![] };
    Plan { summary, status: "complete".into(), sets: sets.into_iter().map(|(number, questions)| PlanSet { number, questions }).collect() }
}

/// Two sets, numbered 3 and 7: a typed, a multiple-choice, a self-graded question.
fn two_sets() -> Session {
    let mut s = Session::new(P::Loading, true, "k".into());
    s.load(plan(vec![
        (3, vec![question("translation", "ni hao", None), question("multiple_choice", "xie xie", Some(&["zai jian", "xie xie"]))]),
        (7, vec![question("free_response", "wo hen hao", None)]),
    ]));
    s
}

fn refused(s: &mut Session, step: impl FnOnce(&mut Session) -> Result<(), crate::types::EmbedError>) {
    let before = s.snapshot();
    let err = step(s).expect_err("a wrong-phase call is refused");
    assert_eq!(err.kind, ErrorKind::InvalidState);
    assert_eq!(s.snapshot(), before, "and leaves the snapshot unchanged");
}

// 1.10.26y AC9
#[test]
fn a_plan_walks_every_phase_and_refuses_wrong_phase_calls() {
    let mut s = two_sets();
    assert_eq!(s.phase, P::Unanswered);
    refused(&mut s, |s| s.next().map(drop));
    refused(&mut s, |s| s.self_grade(true));
    s.answer("Ni hao!").unwrap();
    assert_eq!((s.phase, s.snapshot().last_result.unwrap().outcome), (P::Reviewed, Some(O::Correct)));
    refused(&mut s, |s| s.answer("again"));
    assert_eq!(s.next().unwrap(), After::Nothing);
    s.choose(1).unwrap();
    assert_eq!(s.next().unwrap(), After::Nothing);
    assert_eq!(s.phase, P::SetComplete);
    s.next().unwrap();
    assert_eq!((s.phase, s.snapshot().set_index, s.snapshot().question.unwrap().self_graded), (P::Unanswered, 1, true));
    s.answer("anything").unwrap();
    assert_eq!(s.phase, P::AwaitingSelfGrade);
    refused(&mut s, |s| s.reveal());
    s.self_grade(true).unwrap();
    s.next().unwrap();
    assert_eq!(s.next().unwrap(), After::Report);
    assert_eq!((s.phase, s.completed), (P::Reporting, true));
    assert_eq!(s.outcomes, vec![vec![Some(O::Correct), Some(O::Correct)], vec![Some(O::SelfCorrect)]]);
    refused(&mut s, |s| s.finish().map(drop));

    // practice_finish from each of the four phases marks the rest skipped.
    // r = reveal, n = next, a = answer.
    for (walk, phase) in [("", P::Unanswered), ("rnrnna", P::AwaitingSelfGrade), ("r", P::Reviewed), ("rnrn", P::SetComplete)] {
        let mut s = two_sets();
        for step in walk.chars() {
            match step {
                'r' => s.reveal().unwrap(),
                'n' => drop(s.next().unwrap()),
                _ => s.answer("x").unwrap(),
            }
        }
        assert_eq!(s.phase, phase);
        assert_eq!(s.finish().unwrap(), After::Report);
        assert!(!s.completed && s.phase == P::Reporting);
        assert!(s.outcomes.iter().flatten().all(Option::is_some));
        assert_eq!(s.outcomes[1][0], Some(O::Skipped), "from {phase:?}");
    }
}
