//! The webhook receiver the conformance server posts to. Delivery `n` of
//! the armed case is answered through a `PlayerRouter` built from that
//! item's `router.secrets`, whose `on_event` records what it saw and raises
//! when the item's `handler` is `raise`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use lingara::events::{Event, Webhook};
use lingara_embed::server::{DirectoryError, PlayerRouter, RoutedEvent};
use serde_json::Value;

use crate::recording::{Recording, Sink};

/// What one delivery's `on_event` saw.
#[derive(Clone, Debug)]
pub struct Seen {
    pub player_ref: Option<String>,
    pub event_id: String,
    pub unknown: bool,
}

/// The armed case, as the receiver needs it.
pub struct Armed {
    pub deliver: Vec<Value>,
    pub directory: Arc<Recording>,
    pub seen: Vec<Option<Seen>>,
    pub received: usize,
    pub answered: usize,
}

#[derive(Clone)]
pub struct Receiver {
    pub armed: Arc<Mutex<Option<Armed>>>,
    /// `primary` and `secondary`, from `LINGARA_EMBED_WEBHOOK_SECRETS`.
    pub secrets: Arc<HashMap<String, String>>,
    pub sink: Sink,
}

impl Receiver {
    pub fn lock(&self) -> std::sync::MutexGuard<'_, Option<Armed>> {
        self.armed.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// The router delivery `item` is answered through.
    pub fn router(&self, item: &Value, directory: Arc<Recording>) -> Result<PlayerRouter, String> {
        let names: Vec<&str> = match item.pointer("/router/secrets").and_then(Value::as_array) {
            Some(names) => names.iter().filter_map(Value::as_str).collect(),
            None => vec!["primary"],
        };
        let secrets: Vec<&String> = names.iter().map(|n| self.secrets.get(*n).ok_or(format!("no secret named {n}"))).collect::<Result<_, _>>()?;
        let webhook = Webhook::with_secrets(secrets).map_err(|e| e.to_string())?;
        let sink = self.sink.clone();
        Ok(PlayerRouter::new(webhook, directory).with_on_error(move |failure| sink.push(format!("on_error: {failure}"))))
    }
}

pub fn app(receiver: Receiver) -> Router {
    Router::new().fallback(deliver).with_state(receiver)
}

async fn deliver(State(receiver): State<Receiver>, headers: HeaderMap, body: Bytes) -> StatusCode {
    let claimed = {
        let mut guard = receiver.lock();
        guard.as_mut().map(|armed| {
            let n = armed.received;
            armed.received += 1;
            (n, armed.deliver.get(n).cloned().unwrap_or(Value::Null), Arc::clone(&armed.directory))
        })
    };
    let Some((n, item, directory)) = claimed else { return StatusCode::CONFLICT };
    let status = match receiver.router(&item, directory) {
        Ok(router) => {
            let raise = item["handler"] == "raise";
            let on_event = |routed: RoutedEvent| record(receiver.clone(), n, routed, raise);
            router.handle(&body, &headers, on_event).await
        }
        Err(e) => {
            receiver.sink.push(format!("harness: {e}"));
            500
        }
    };
    if let Some(armed) = receiver.lock().as_mut() {
        armed.answered += 1;
    }
    StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
}

async fn record(receiver: Receiver, n: usize, routed: RoutedEvent, raise: bool) -> Result<(), DirectoryError> {
    let seen = Seen {
        player_ref: routed.player_ref,
        event_id: routed.event.id().to_string(),
        unknown: matches!(routed.event, Event::Unknown(_)),
    };
    if let Some(slot) = receiver.lock().as_mut().and_then(|a| a.seen.get_mut(n)) {
        *slot = Some(seen);
    }
    if raise { Err("the conformance handler raised, as the case asked".into()) } else { Ok(()) }
}
