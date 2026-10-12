use crate::error::ErrorKind;
use crate::practice::grading::{is_self_graded, matches, normalise};
use crate::practice::machine::Session;
use crate::tests::machine_tests::{plan, question};
use crate::types::{PracticeOutcome as O, PracticePhase as P};

fn one(q: crate::api::Question) -> Session {
    let mut s = Session::new(P::Loading, true, "k".into());
    s.load(plan(vec![(1, vec![q])]));
    s
}

fn graded(s: &Session) -> Option<O> {
    s.last_result.as_ref().and_then(|r| r.outcome)
}

// 1.10.26y AC10
#[test]
fn grading_by_shape_and_normalisation() {
    // Normalisation folds case and whitespace, and removes punctuation anywhere.
    assert_eq!(normalise("  Ni   HAO! "), "ni hao");
    assert_eq!(normalise("¿Qué tal?"), "qué tal");
    assert_eq!(normalise("你好，世界。"), "你好世界");
    assert_eq!(normalise("don't-stop"), "dontstop");
    assert!(matches("Xie-xie.", "xiexie"));
    assert!(!matches("xie xie", "xiexie"), "whitespace is collapsed, not removed");

    // An options question grades alike by index and by text.
    let choice = || one(question("multiple_choice", "Xie xie", Some(&["zai jian", "xie  xie!"])));
    let (mut by_index, mut by_text, mut wrong) = (choice(), choice(), choice());
    by_index.choose(1).unwrap();
    by_text.answer("XIE XIE").unwrap();
    wrong.choose(0).unwrap();
    assert_eq!((graded(&by_index), graded(&by_text), graded(&wrong)), (Some(O::Correct), Some(O::Correct), Some(O::Incorrect)));
    let mut out_of_range = choice();
    assert_eq!(out_of_range.choose(2).unwrap_err().kind, ErrorKind::InvalidInput);
    assert_eq!(out_of_range.phase, P::Unanswered);

    // A typed question has no options to choose from.
    let mut typed = one(question("translation", "ni hao", None));
    assert_eq!(typed.choose(0).unwrap_err().kind, ErrorKind::InvalidInput);
    typed.answer("Ni hao.").unwrap();
    assert_eq!(graded(&typed), Some(O::Correct));

    // The three self-graded types wait for the learner's grade, showing the model answer.
    for kind in ["free_response", "phrase_translation", "conversational_response"] {
        assert!(is_self_graded(kind));
        let mut s = one(question(kind, "wo hen hao", None));
        s.answer("whatever I said").unwrap();
        assert_eq!(s.phase, P::AwaitingSelfGrade);
        let shown = s.snapshot().last_result.unwrap();
        assert_eq!((shown.outcome, shown.expected.as_str(), shown.explanation.as_str()), (None, "wo hen hao", "because"));
        s.self_grade(false).unwrap();
        assert_eq!(graded(&s), Some(O::SelfIncorrect));
    }

    // An unknown type is a typed answer, never an error.
    let mut unknown = one(question("brand_new_type", "ni hao", None));
    unknown.answer("ni hao").unwrap();
    assert_eq!((unknown.phase, graded(&unknown)), (P::Reviewed, Some(O::Correct)));
}
