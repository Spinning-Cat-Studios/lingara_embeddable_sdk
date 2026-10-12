//! The library's streams as the port's `Frames` and `Tail`: each item read
//! under `tracked`, and mapped to this crate's frames.

use lingara::EventStream;
use lingara::events::EventTail;
use lingara::models::{CreateLessonPlanEvent, SendDialogueTurnEvent, StreamLessonPlanEvent};
use serde::de::DeserializeOwned;

use crate::api::{Frames, PlanFrame, Tail, TurnFrame};
use crate::error::EmbedError;
use crate::handoff::BoxFuture;
use crate::library::map::{lingara_event, plan};
use crate::token_bridge::tracked;
use crate::types::LingaraEvent;

/// One stream item through `tracked`, mapped by `map`.
async fn read<E: DeserializeOwned, T>(stream: &mut EventStream<E>, map: fn(E) -> T) -> Option<Result<T, EmbedError>> {
    let item = tracked(async { stream.next().await.transpose() }).await;
    item.transpose().map(|r| r.map(map))
}

impl Frames<PlanFrame> for EventStream<CreateLessonPlanEvent> {
    fn next(&mut self) -> BoxFuture<'_, Option<Result<PlanFrame, EmbedError>>> {
        Box::pin(read(self, |e| match e {
            CreateLessonPlanEvent::Started(s) => PlanFrame::Started(s.plan_id),
            CreateLessonPlanEvent::Result(r) => PlanFrame::Result(Box::new(plan(r.plan))),
            _ => PlanFrame::Other,
        }))
    }
}

impl Frames<PlanFrame> for EventStream<StreamLessonPlanEvent> {
    fn next(&mut self) -> BoxFuture<'_, Option<Result<PlanFrame, EmbedError>>> {
        Box::pin(read(self, |e| match e {
            StreamLessonPlanEvent::Started(s) => PlanFrame::Started(s.plan_id),
            StreamLessonPlanEvent::Result(r) => PlanFrame::Result(Box::new(plan(r.plan))),
            StreamLessonPlanEvent::Pending(_) => PlanFrame::Pending,
            _ => PlanFrame::Other,
        }))
    }
}

impl Frames<TurnFrame> for EventStream<SendDialogueTurnEvent> {
    fn next(&mut self) -> BoxFuture<'_, Option<Result<TurnFrame, EmbedError>>> {
        Box::pin(read(self, |e| match e {
            SendDialogueTurnEvent::Delta(d) => TurnFrame::Delta(d.text),
            SendDialogueTurnEvent::Notice(n) => TurnFrame::Notice { code: n.code, message: n.message },
            // `done` and `error` end the stream before they reach here.
            _ => TurnFrame::Other,
        }))
    }
}

impl Tail for EventTail {
    fn next(&mut self) -> BoxFuture<'_, Option<Result<LingaraEvent, EmbedError>>> {
        Box::pin(async move {
            let item = tracked(async { EventTail::next(self).await.transpose() }).await;
            item.transpose().map(|r| r.map(|e| lingara_event(&e)))
        })
    }

    fn cursor(&self) -> Option<String> {
        EventTail::cursor(self).map(str::to_owned)
    }
}
