//! The adapter from `Api` to the `lingara` library: the one module that
//! names its client, its models and its streams (D10). Bodies go in as JSON
//! and are decoded into the library's validated types here; answers come
//! back as this crate's own types (`map.rs`, `streams.rs`). Every call runs
//! under `tracked`, so a token the host could not give surfaces as
//! `TokenUnavailable`.

mod map;
mod streams;

use std::sync::Arc;
use std::time::Duration;

use lingara::Client;
use lingara::events::{EventStart, EventsOptions, InboundEvent, SendEventOptions};
use lingara::models::{DialogueTurnRequest, LessonPlanCreateRequest};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::api::{Accepted, Api, Plan, PlanStream, Tail, TurnStream};
use crate::config::EmbedConfig;
use crate::error::{EmbedError, ErrorKind};
use crate::handoff::BoxFuture;
use crate::token::HostTokenSource;
use crate::token_bridge::{LibrarySource, tracked};

/// This crate's own `User-Agent` token, after the library's.
pub(crate) fn user_agent_suffix(config: &EmbedConfig) -> String {
    let own = format!("lingara-embed/{}", env!("CARGO_PKG_VERSION"));
    match config.user_agent_suffix.as_deref().map(str::trim) {
        Some(suffix) if !suffix.is_empty() => format!("{own} {suffix}"),
        _ => own,
    }
}

pub(crate) struct LibraryApi {
    client: Client,
}

impl LibraryApi {
    pub fn new(config: &EmbedConfig, tokens: Arc<HostTokenSource>) -> Result<Self, EmbedError> {
        let mut builder = Client::builder()
            .base_url(config.base_url.clone())
            .token_source(Arc::new(LibrarySource(tokens)))
            .user_agent_suffix(user_agent_suffix(config))
            .stream_idle_timeout(Duration::from_millis(config.stream_idle_timeout_ms.into()));
        if let Some(version) = &config.api_version {
            builder = builder.version(version.clone());
        }
        let client = builder.build().map_err(|e| EmbedError::new(ErrorKind::Config, e.to_string()))?;
        Ok(Self { client })
    }
}

/// A body the library's validated types refuse: local validation missed a bound.
fn decode<T: DeserializeOwned>(body: Value) -> Result<T, EmbedError> {
    serde_json::from_value(body).map_err(|e| EmbedError::invalid(format!("the request does not fit the published schema: {e}")))
}

impl Api for LibraryApi {
    fn tail(&self, cursor: Option<String>, types: Vec<String>) -> Box<dyn Tail> {
        let start = cursor.is_none().then_some(EventStart::Latest);
        Box::new(self.client.tail_events(EventsOptions { cursor, start, types }))
    }

    fn send_event(&self, body: Value, key: String) -> BoxFuture<'_, Result<Accepted, EmbedError>> {
        Box::pin(async move {
            let event: InboundEvent = decode(body)?;
            let options = SendEventOptions { idempotency_key: Some(key) };
            let accepted = tracked(self.client.send_event(&event, options)).await?.into_inner();
            Ok(Accepted { event_id: accepted.id, reaction: accepted.reaction.as_ref().map(map::reaction) })
        })
    }

    fn get_plan(&self, id: String) -> BoxFuture<'_, Result<Plan, EmbedError>> {
        Box::pin(async move { Ok(map::plan(tracked(self.client.get_lesson_plan(&id)).await?.into_inner())) })
    }

    fn create_plan(&self, body: Value) -> BoxFuture<'_, Result<PlanStream, EmbedError>> {
        Box::pin(async move {
            let request: LessonPlanCreateRequest = decode(body)?;
            let stream = tracked(self.client.create_lesson_plan(&request)).await?;
            Ok(Box::new(stream) as PlanStream)
        })
    }

    fn stream_plan(&self, id: String) -> BoxFuture<'_, Result<PlanStream, EmbedError>> {
        Box::pin(async move { Ok(Box::new(tracked(self.client.stream_lesson_plan(&id)).await?) as PlanStream) })
    }

    fn dialogue_turn(&self, body: Value) -> BoxFuture<'_, Result<TurnStream, EmbedError>> {
        Box::pin(async move {
            let request: DialogueTurnRequest = decode(body)?;
            Ok(Box::new(tracked(self.client.send_dialogue_turn(&request)).await?) as TurnStream)
        })
    }
}
