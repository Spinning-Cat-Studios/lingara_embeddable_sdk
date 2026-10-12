//! An in-memory `Api` for the tests that run under paused time: no network,
//! so tokio's clock can jump a 60 s restart or a 120 s wait deterministically.
//! Each answer is scripted in order; every call is logged with the paused
//! clock's instant.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use serde_json::Value;
use tokio::time::Instant;

use crate::api::{Accepted, Api, Frames, Plan, PlanFrame, PlanStream, Tail, TurnFrame, TurnStream};
use crate::error::EmbedError;
use crate::handoff::BoxFuture;
use crate::types::LingaraEvent;

/// One scripted tail: its items, then silence (`pending` forever), or its end.
pub struct ScriptedTail {
    items: VecDeque<Result<LingaraEvent, EmbedError>>,
}

impl Tail for ScriptedTail {
    fn next(&mut self) -> BoxFuture<'_, Option<Result<LingaraEvent, EmbedError>>> {
        Box::pin(async move {
            match self.items.pop_front() {
                Some(item) => Some(item),
                None => std::future::pending().await,
            }
        })
    }

    fn cursor(&self) -> Option<String> {
        None
    }
}

/// A tail whose first read panics: a bug on the I/O thread.
struct Panicking;

impl Tail for Panicking {
    fn next(&mut self) -> BoxFuture<'_, Option<Result<LingaraEvent, EmbedError>>> {
        Box::pin(async { panic!("a scripted panic on the I/O thread") })
    }

    fn cursor(&self) -> Option<String> {
        None
    }
}

struct Scripted<T>(VecDeque<Result<T, EmbedError>>);

impl<T: Send> Frames<T> for Scripted<T> {
    fn next(&mut self) -> BoxFuture<'_, Option<Result<T, EmbedError>>> {
        Box::pin(async move { self.0.pop_front() })
    }
}

type PlanFrames = Vec<Result<PlanFrame, EmbedError>>;

#[derive(Default)]
pub struct FakeApi {
    pub calls: Mutex<Vec<(Instant, String)>>,
    pub tails: Mutex<VecDeque<Vec<Result<LingaraEvent, EmbedError>>>>,
    pub sends: Mutex<VecDeque<Result<Accepted, EmbedError>>>,
    pub plans: Mutex<VecDeque<Result<Plan, EmbedError>>>,
    pub creates: Mutex<VecDeque<PlanFrames>>,
    pub plan_streams: Mutex<VecDeque<PlanFrames>>,
    pub bodies: Mutex<Vec<Value>>,
    /// The next tail panics on its first read.
    pub panic_on_tail: std::sync::atomic::AtomicBool,
}

fn frames(script: &Mutex<VecDeque<PlanFrames>>) -> PlanStream {
    Box::new(Scripted::<PlanFrame>(script.lock().unwrap().pop_front().unwrap_or_default().into()))
}

impl FakeApi {
    fn log(&self, call: impl Into<String>) {
        self.calls.lock().unwrap().push((Instant::now(), call.into()));
    }

    /// The instants of every call named `call`.
    pub fn at(&self, call: &str) -> Vec<Instant> {
        self.calls.lock().unwrap().iter().filter(|(_, c)| c == call).map(|(t, _)| *t).collect()
    }
}

/// Lets a test keep its own `Arc<FakeApi>` beside the core's.
pub struct Handle(pub Arc<FakeApi>);

impl Api for Handle {
    fn tail(&self, _cursor: Option<String>, _types: Vec<String>) -> Box<dyn Tail> {
        self.0.log("tail");
        if self.0.panic_on_tail.load(std::sync::atomic::Ordering::SeqCst) {
            return Box::new(Panicking);
        }
        let items = self.0.tails.lock().unwrap().pop_front().unwrap_or_default();
        Box::new(ScriptedTail { items: items.into() })
    }

    fn send_event(&self, body: Value, _key: String) -> BoxFuture<'_, Result<Accepted, EmbedError>> {
        self.0.log("send_event");
        self.0.bodies.lock().unwrap().push(body);
        let answer = self.0.sends.lock().unwrap().pop_front();
        Box::pin(async move { answer.unwrap_or_else(|| Ok(Accepted { event_id: "lgr_evt_fake".into(), reaction: None })) })
    }

    fn get_plan(&self, _id: String) -> BoxFuture<'_, Result<Plan, EmbedError>> {
        self.0.log("get_plan");
        let answer = self.0.plans.lock().unwrap().pop_front();
        Box::pin(async move { answer.unwrap_or_else(|| Err(EmbedError::invalid("no scripted plan"))) })
    }

    fn create_plan(&self, _body: Value) -> BoxFuture<'_, Result<PlanStream, EmbedError>> {
        self.0.log("create_plan");
        let stream = frames(&self.0.creates);
        Box::pin(async { Ok(stream) })
    }

    fn stream_plan(&self, _id: String) -> BoxFuture<'_, Result<PlanStream, EmbedError>> {
        self.0.log("stream_plan");
        let stream = frames(&self.0.plan_streams);
        Box::pin(async { Ok(stream) })
    }

    fn dialogue_turn(&self, _body: Value) -> BoxFuture<'_, Result<TurnStream, EmbedError>> {
        self.0.log("dialogue_turn");
        Box::pin(async { Ok(Box::new(Scripted::<TurnFrame>(VecDeque::new())) as TurnStream) })
    }
}
