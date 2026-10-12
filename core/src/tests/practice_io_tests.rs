use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use serde_json::{Value, json};

use crate::api::{Accepted, PlanFrame};
use crate::embed::Embed;
use crate::error::ErrorKind;
use crate::limits;
use crate::tests::fake_api::FakeApi;
use crate::tests::fake_lingara::{FakeLingara, Reply};
use crate::tests::machine_tests::{plan, question};
use crate::tests::{SECS, accepted, config, paused, refusal, started, wait_for};
use crate::types::{EmbedError, EmbedEvent, LingaraEvent, LingaraEventDetail, PracticePhase as P, PracticeSource, Reaction, SessionId};

const PLAN: &str = "8f0e2c1a-0000-4000-8000-000000000001";

/// A plan whose sets have these numbers and question counts.
fn plan_json(sets: &[(u8, usize)]) -> Value {
    let question = |i: usize| json!({ "type": "translation", "prompt": format!("q{i}"), "answer": "ni hao", "explanation": "e" });
    let sets: Vec<Value> = sets.iter().map(|(n, count)| json!({ "number": n, "questions": (0..*count).map(question).collect::<Vec<_>>() })).collect();
    json!({
        "id": PLAN, "status": "complete", "level": 2, "source_lang": "en", "target_lang": "zh", "ai_generated": true,
        "created_at": "2026-10-01T12:00:00Z", "title": "At the market",
        "content": { "learning_objectives": [], "vocabulary": [{ "word": "你好", "translation": "hello" }], "sets": sets },
    })
}

/// A session on `sets`, its report answered by `post` (the n-th report).
fn practise(sets: &'static [(u8, usize)], report: bool, post: impl Fn(usize) -> Reply + Send + Sync + 'static) -> (FakeLingara, Embed, SessionId) {
    let posts = AtomicUsize::new(0);
    let fake = FakeLingara::start(move |s, _| match s.method.as_str() {
        "GET" => Reply::Json(200, plan_json(sets)),
        _ => post(posts.fetch_add(1, Ordering::SeqCst)),
    });
    let embed = started(config(&fake.url));
    let session = embed.practice_start(PracticeSource::Plan { plan_id: PLAN.into(), report }).unwrap();
    wait_for(&embed, 5 * SECS, |e| matches!(e, EmbedEvent::Practice { snapshot, .. } if snapshot.phase == P::Unanswered));
    (fake, embed, session)
}

fn phase(phase: P) -> impl Fn(&EmbedEvent) -> bool {
    move |e| matches!(e, EmbedEvent::Practice { snapshot, .. } if snapshot.phase == phase)
}

fn reports(fake: &FakeLingara) -> Vec<(Value, String)> {
    let posts = fake.seen().into_iter().filter(|s| s.is("POST", "/v1/events"));
    posts.map(|s| (s.body.clone(), s.header("idempotency-key").unwrap_or_default().to_owned())).collect()
}

fn items(list: &[(u8, u8, &str)]) -> Value {
    list.iter().map(|(set, q, o)| json!({ "set_number": set, "question": q, "outcome": o })).collect()
}

/// Reveals `n` questions, moving on after each.
fn reveal(embed: &Embed, session: SessionId, n: usize) {
    for _ in 0..n {
        embed.practice_reveal(session).unwrap();
        if embed.practice_next(session).unwrap().phase == P::SetComplete {
            embed.practice_next(session).unwrap();
        }
    }
}

// 1.10.26y AC11
#[test]
fn one_report_per_session_under_a_stable_key() {
    let ok = |_| Reply::Json(202, accepted("lgr_evt_1"));
    let (fake, embed, s) = practise(&[(2, 2), (5, 1)], true, ok);
    reveal(&embed, s, 3);
    wait_for(&embed, 5 * SECS, phase(P::Reported));
    let all = (reports(&fake), embed.practice_snapshot(s).unwrap().phase);
    let want = json!({ "type": "world.practice_completed", "data": { "plan_id": PLAN, "completed": true,
        "items": items(&[(2, 0, "revealed"), (2, 1, "revealed"), (5, 0, "revealed")]) } });
    assert_eq!((all.0.len(), &all.0[0].0, all.1), (1, &want, P::Reported));
    let key = &all.0[0].1;
    assert!(key.starts_with("pc:") && key.len() == 35 && key[3..].bytes().all(|b| b.is_ascii_hexdigit()), "{key}");

    let (fake, embed, s) = practise(&[(2, 2), (5, 1)], true, ok);
    reveal(&embed, s, 1);
    embed.practice_finish(s).unwrap();
    wait_for(&embed, 5 * SECS, phase(P::Reported));
    let body = &reports(&fake)[0].0["data"];
    assert_eq!((&body["completed"], &body["items"]), (&json!(false), &items(&[(2, 0, "revealed"), (2, 1, "skipped"), (5, 0, "skipped")])));

    let (fake, embed, s) = practise(&[(1, 1)], true, |n| if n == 0 { Reply::Json(503, refusal("unavailable")) } else { Reply::Json(202, accepted("e")) });
    embed.practice_finish(s).unwrap();
    let (failed, _) = wait_for(&embed, 5 * SECS, phase(P::ReportFailed));
    assert!(matches!(failed, EmbedEvent::Practice { snapshot, .. } if snapshot.failure.as_ref().is_some_and(|f| f.retryable)));
    assert_eq!(embed.practice_finish(s).unwrap().phase, P::Reporting);
    wait_for(&embed, 5 * SECS, phase(P::Reported));
    let sent = reports(&fake);
    assert_eq!((sent.len(), &sent[0].1), (2, &sent[1].1), "a re-sent report carries the same key");

    let (_, embed, s) = practise(&[(1, 1)], true, |_| Reply::Json(400, refusal("invalid_request")));
    embed.practice_finish(s).unwrap();
    wait_for(&embed, 5 * SECS, phase(P::ReportFailed));
    assert_eq!(embed.practice_finish(s).unwrap_err().kind, ErrorKind::InvalidState, "a demo plan's refusal is final");

    let (fake, embed, s) = practise(&[(1, 2)], false, ok);
    assert_eq!(embed.practice_finish(s).unwrap().phase, P::Ended);
    std::thread::sleep(Duration::from_millis(200));
    assert!(reports(&fake).is_empty(), "report: false sends nothing");

    let (fake, embed, s) = practise(&[(1, 120)], true, ok);
    reveal(&embed, s, 10);
    embed.practice_finish(s).unwrap();
    wait_for(&embed, 5 * SECS, phase(P::Reported));
    let list = reports(&fake)[0].0["data"]["items"].as_array().unwrap().clone();
    assert_eq!(list.len(), limits::MAX_PRACTICE_ITEMS);
    assert!(list[..10].iter().all(|i| i["outcome"] == "revealed") && list[99]["question"] == 99, "skipped ones are dropped first, from the end");
}

fn ready(plan_id: &str) -> LingaraEvent {
    let detail = LingaraEventDetail::LessonPlanReady { plan_id: plan_id.into(), title: None, level: 2, source_lang: "en".into(), target_lang: "zh".into(), complete: true };
    LingaraEvent { id: "lgr_evt_r".into(), event_type: "lesson_plan.ready".into(), created_at: String::new(), data_json: "{}".into(), detail }
}

fn generating(plan_status: &str) -> Result<Accepted, EmbedError> {
    let reaction = Reaction { status: "started".into(), plan_id: Some(PLAN.into()), plan_status: Some(plan_status.into()), code: None, error: None };
    Ok(Accepted { event_id: "lgr_evt_1".into(), reaction: Some(reaction) })
}

fn with_status(status: &str) -> crate::api::Plan {
    let mut p = plan(vec![(1, vec![question("translation", "ni hao", None)])]);
    p.status = status.into();
    p
}

fn request() -> PracticeSource {
    PracticeSource::Request { topic: "ordering tea".into(), source_lang: "en".into(), target_lang: "zh".into(), level: 2, tags: vec![] }
}

/// A paused session from `source` over `api`, run to its first resting phase.
fn reach(api: &Arc<FakeApi>, source: PracticeSource, until: P) -> Embed {
    let embed = paused(config("http://127.0.0.1:9"), api);
    embed.practice_start(source).unwrap();
    wait_for(&embed, 5 * SECS, phase(until));
    embed
}

fn calls(api: &FakeApi) -> Vec<String> {
    api.calls.lock().unwrap().iter().map(|(_, c)| c.clone()).collect()
}

// 1.10.26y AC12
#[test]
fn each_source_reaches_a_plan() {
    let api = Arc::new(FakeApi::default());
    api.sends.lock().unwrap().push_back(generating("generating"));
    api.tails.lock().unwrap().push_back(vec![Ok(ready(PLAN))]);
    api.plans.lock().unwrap().push_back(Ok(with_status("complete")));
    let _e = reach(&api, request(), P::Unanswered);
    assert_eq!(calls(&api), ["send_event", "tail", "get_plan"], "generating waits for ready, then reads");
    assert_eq!(api.bodies.lock().unwrap()[0]["data"]["generate"], json!(true));

    let api = Arc::new(FakeApi::default());
    api.sends.lock().unwrap().push_back(generating("generating"));
    api.plans.lock().unwrap().extend([Ok(with_status("complete"))]);
    let _e = reach(&api, request(), P::Unanswered);
    assert_eq!(api.at("get_plan")[0] - api.at("send_event")[0], Duration::from_millis(limits::PLAN_POLL_MS), "a missed ready is caught by the 30 s read");

    let api = Arc::new(FakeApi::default());
    api.sends.lock().unwrap().push_back(generating("generating"));
    let mut failed = ready(PLAN);
    failed.detail = LingaraEventDetail::LessonPlanFailed { plan_id: PLAN.into(), reason: "generation_failed".into() };
    api.tails.lock().unwrap().push_back(vec![Ok(failed)]);
    let _e = reach(&api, request(), P::Failed);

    let api = Arc::new(FakeApi::default());
    api.sends.lock().unwrap().push_back(generating("generating"));
    api.plans.lock().unwrap().extend((0..4).map(|_| Ok(with_status("generating"))));
    let e = reach(&api, request(), P::Failed);
    let reads = api.at("get_plan");
    assert_eq!((reads.len(), reads[3] - api.at("send_event")[0]), (4, Duration::from_millis(limits::PLAN_WAIT_MS)), "still generating at 120 s");
    assert_eq!(e.practice_snapshot(2).unwrap().failure.unwrap().kind, ErrorKind::Unavailable);

    let api = Arc::new(FakeApi::default());
    api.sends.lock().unwrap().push_back(generating("partial"));
    api.plans.lock().unwrap().push_back(Ok(with_status("partial")));
    let _e = reach(&api, request(), P::Unanswered);
    assert_eq!(calls(&api), ["send_event", "get_plan"], "a pool-served plan is read at once");

    each_create_reaches_a_plan();
}

fn each_create_reaches_a_plan() {
    let create = || PracticeSource::Create { context: "a tea house".into(), source_lang: "en".into(), target_lang: "zh".into(), level: 2 };
    let result = || Ok(PlanFrame::Result(Box::new(with_status("complete"))));
    let api = Arc::new(FakeApi::default());
    api.creates.lock().unwrap().push_back(vec![Ok(PlanFrame::Started(PLAN.into())), result()]);
    let _e = reach(&api, create(), P::Unanswered);
    assert_eq!(calls(&api), ["create_plan"], "result carries the plan: no GET");

    let api = Arc::new(FakeApi::default());
    api.creates.lock().unwrap().push_back(vec![Ok(PlanFrame::Started(PLAN.into())), Err(EmbedError::new(ErrorKind::Network, "dropped"))]);
    api.plan_streams.lock().unwrap().push_back(vec![result()]);
    let _e = reach(&api, create(), P::Unanswered);
    assert_eq!(calls(&api), ["create_plan", "stream_plan"], "a drop after started reopens once");

    let api = Arc::new(FakeApi::default());
    api.plans.lock().unwrap().push_back(Ok(plan(vec![(1, vec![])])));
    let e = reach(&api, PracticeSource::Plan { plan_id: PLAN.into(), report: true }, P::Failed);
    assert_eq!(e.practice_snapshot(2).unwrap().failure.unwrap().kind, ErrorKind::InvalidInput, "a plan with no question");
}
