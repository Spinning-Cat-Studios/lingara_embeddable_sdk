//! The practice session's I/O (D7): reaching a plan from each source, and
//! sending the report. Every transition here is reported to the host as a
//! `Practice` event; the pure ones (answering) return from their call.

use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::sync::oneshot;
use tokio::time::Instant;

use crate::api::{Plan, PlanFrame, PlanStream};
use crate::error::{EmbedError, ErrorKind};
use crate::ids::random_key;
use crate::limits;
use crate::practice::PlanSignal;
use crate::practice::report;
use crate::shared::Shared;
use crate::types::{EmbedEvent, PracticePhase, PracticeSource, SessionId};
use crate::world;

pub(crate) fn check(source: &PracticeSource) -> Result<(), EmbedError> {
    let (source_lang, target_lang, level) = match source {
        PracticeSource::Request { topic, source_lang, target_lang, level, tags } => {
            world::text("topic", topic, limits::TOPIC_MAX_CHARS)?;
            world::tags(tags)?;
            (source_lang, target_lang, *level)
        }
        PracticeSource::Create { context, source_lang, target_lang, level } => {
            world::lang("context", context)?;
            (source_lang, target_lang, *level)
        }
        PracticeSource::Plan { plan_id, .. } => return world::lang("plan_id", plan_id),
    };
    world::lang("source_lang", source_lang)?;
    world::lang("target_lang", target_lang)?;
    world::level("level", level)
}

/// Reports the session's snapshot as an I/O-driven transition.
async fn emit(shared: &Shared, session: SessionId) {
    if let Ok(snapshot) = shared.practice.with(session, |s| s.snapshot()) {
        shared.outbox.send(EmbedEvent::Practice { session, snapshot }).await;
    }
}

async fn transition(shared: &Shared, session: SessionId, step: impl FnOnce(&mut crate::practice::machine::Session)) {
    let _ = shared.practice.with(session, step);
    emit(shared, session).await;
}

/// Reaches a plan from `source`, then waits for the host.
pub(crate) async fn run(shared: Arc<Shared>, session: SessionId, source: PracticeSource) {
    let plan = match source {
        PracticeSource::Request { topic, source_lang, target_lang, level, tags } => {
            let mut data = json!({ "topic": topic, "source_lang": source_lang, "target_lang": target_lang, "level": level, "generate": true });
            if !tags.is_empty() {
                data["tags"] = json!(tags);
            }
            request(&shared, session, json!({ "type": "world.practice_requested", "data": data })).await
        }
        PracticeSource::Create { context, source_lang, target_lang, level } => {
            create(&shared, json!({ "context": context, "source_lang": source_lang, "target_lang": target_lang, "level": level })).await
        }
        PracticeSource::Plan { plan_id, .. } => shared.api.get_plan(plan_id).await,
    };
    match plan {
        Ok(plan) => transition(&shared, session, |s| s.load(plan)).await,
        Err(error) => transition(&shared, session, |s| s.fail(error)).await,
    }
}

/// `world.practice_requested`, then the plan: read at once when the pool
/// served one, waited for when it is being made.
async fn request(shared: &Arc<Shared>, session: SessionId, body: Value) -> Result<Plan, EmbedError> {
    let accepted = shared.api.send_event(body, random_key()).await?;
    let reaction = accepted.reaction.ok_or_else(|| EmbedError::new(ErrorKind::Internal, "the request was accepted with no reaction"))?;
    if reaction.status != "started" {
        let kind = if reaction.status == "refused" { ErrorKind::InvalidInput } else { ErrorKind::Unavailable };
        let mut error = EmbedError::new(kind, reaction.error.unwrap_or_else(|| format!("the practice request was {}", reaction.status)));
        error.code = reaction.code;
        return Err(error);
    }
    let plan_id = reaction.plan_id.ok_or_else(|| EmbedError::new(ErrorKind::Internal, "the reaction names no plan"))?;
    if reaction.plan_status.as_deref() == Some("generating") {
        // Watch first, then turn the stream on and leave it on.
        let signal = shared.practice.watch(&plan_id);
        shared.stream.resume(shared);
        transition(shared, session, |s| s.phase = PracticePhase::Generating).await;
        if let Some(plan) = wait_for_plan(shared, &plan_id, signal).await? {
            return Ok(plan);
        }
    }
    transition(shared, session, |s| s.phase = PracticePhase::Loading).await;
    shared.api.get_plan(plan_id).await
}

/// Waits on the stream for the plan's `ready` or `failed`, reading the plan
/// every 30 s besides (a `ready` sent before the tail opened is never seen),
/// and failing at 120 s. A plan read that is no longer `generating` ends the
/// wait as `ready` would, and is returned so it is not read twice.
async fn wait_for_plan(shared: &Shared, plan_id: &str, signal: oneshot::Receiver<PlanSignal>) -> Result<Option<Plan>, EmbedError> {
    let mut signal = Some(signal);
    let deadline = Instant::now() + Duration::from_millis(limits::PLAN_WAIT_MS);
    loop {
        let poll_at = (Instant::now() + Duration::from_millis(limits::PLAN_POLL_MS)).min(deadline);
        if let Some(rx) = signal.as_mut() {
            tokio::select! {
                got = rx => match got {
                    Ok(PlanSignal::Ready) => return Ok(None),
                    Ok(PlanSignal::Failed(reason)) => return Err(EmbedError::new(ErrorKind::Unavailable, format!("the plan failed: {reason}"))),
                    Err(_) => signal = None,
                },
                () = tokio::time::sleep_until(poll_at) => {}
            }
        } else {
            tokio::time::sleep_until(poll_at).await;
        }
        let plan = shared.api.get_plan(plan_id.to_owned()).await?;
        if plan.status != "generating" {
            return Ok(Some(plan));
        }
        if Instant::now() >= deadline {
            return Err(EmbedError::new(ErrorKind::Unavailable, "the plan was still generating after 120 s"));
        }
    }
}

/// `POST /v1/lesson-plans` read to its `result`. A drop after `started`
/// reopens the plan's own stream once.
async fn create(shared: &Shared, body: Value) -> Result<Plan, EmbedError> {
    let mut plan_id = None;
    let error = match shared.api.create_plan(body).await {
        Ok(frames) => match read_plan(frames, &mut plan_id).await {
            Ok(plan) => return Ok(plan),
            Err(error) => error,
        },
        Err(error) => return Err(error),
    };
    match plan_id {
        Some(id) if error.kind == ErrorKind::Network => read_plan(shared.api.stream_plan(id).await?, &mut None).await,
        _ => Err(error),
    }
}

async fn read_plan(mut frames: PlanStream, plan_id: &mut Option<String>) -> Result<Plan, EmbedError> {
    loop {
        match frames.next().await {
            Some(Ok(PlanFrame::Started(id))) => *plan_id = Some(id),
            Some(Ok(PlanFrame::Result(plan))) => return Ok(*plan),
            Some(Ok(PlanFrame::Pending)) => return Err(EmbedError::new(ErrorKind::Unavailable, "the plan is not ready to read")),
            Some(Ok(PlanFrame::Other)) => {}
            Some(Err(error)) => return Err(error),
            None => return Err(EmbedError::new(ErrorKind::Network, "the plan stream ended before its result")),
        }
    }
}

/// Sends the session's one report, then reports how it went.
pub(crate) async fn send_report(shared: Arc<Shared>, session: SessionId) {
    let Ok((body, key, cut)) = shared.practice.with(session, |s| {
        let (items, cut) = report::items(s);
        (report::body(s, &items), report::key(s), cut)
    }) else {
        return;
    };
    if cut {
        shared.diag.warn("lingara_embed::practice", format!("the report holds at most {} items: skipped ones were dropped first", limits::MAX_PRACTICE_ITEMS));
    }
    let result = shared.api.send_event(body, key).await.map(drop);
    transition(&shared, session, |s| s.reported(result)).await;
}
