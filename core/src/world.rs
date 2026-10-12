//! The world door (D6): `send_context`, and the inbound bounds every local
//! check shares (the practice request and the dialogue reuse them). A bound
//! is checked before anything is sent; a violation names its field.

use serde_json::{Map, Value, json};

use crate::embed::Embed;
use crate::error::EmbedError;
use crate::ids::random_key;
use crate::limits;
use crate::runtime;
use crate::types::{EmbedEvent, Npc, RequestId, WorldContext};

/// 1 to `max` Unicode scalar values, not blank.
pub(crate) fn text(field: &str, value: &str, max: usize) -> Result<(), EmbedError> {
    let n = value.chars().count();
    if value.trim().is_empty() || n > max {
        return Err(EmbedError::invalid(format!("{field} must be 1–{max} characters, not blank (got {n})")));
    }
    Ok(())
}

pub(crate) fn level(field: &str, level: u8) -> Result<(), EmbedError> {
    if !(limits::LEVEL_MIN..=limits::LEVEL_MAX).contains(&level) {
        return Err(EmbedError::invalid(format!("{field} must be {}–{} (got {level})", limits::LEVEL_MIN, limits::LEVEL_MAX)));
    }
    Ok(())
}

pub(crate) fn lang(field: &str, value: &str) -> Result<(), EmbedError> {
    if value.trim().is_empty() {
        return Err(EmbedError::invalid(format!("{field} must not be empty")));
    }
    Ok(())
}

/// `^[a-z0-9][a-z0-9._-]*$`, 1–24 characters, at most 8.
pub(crate) fn tags(tags: &[String]) -> Result<(), EmbedError> {
    if tags.len() > limits::TAGS_MAX_ITEMS {
        return Err(EmbedError::invalid(format!("tags holds at most {} (got {})", limits::TAGS_MAX_ITEMS, tags.len())));
    }
    let lead = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit();
    let ok = |t: &str| t.len() <= limits::TAG_MAX_CHARS && t.starts_with(lead) && t.chars().all(|c| lead(c) || matches!(c, '.' | '_' | '-'));
    match tags.iter().find(|t| !ok(t)) {
        Some(bad) => Err(EmbedError::invalid(format!("tag `{bad}` does not match {} (1–{} characters)", limits::TAG_PATTERN, limits::TAG_MAX_CHARS))),
        None => Ok(()),
    }
}

pub(crate) fn npc(npc: &Npc) -> Result<(), EmbedError> {
    text("npc.name", &npc.name, limits::NPC_NAME_MAX_CHARS)?;
    match &npc.persona {
        Some(persona) => text("npc.persona", persona, limits::NPC_PERSONA_MAX_CHARS),
        None => Ok(()),
    }
}

/// The inbound `npc` object, its persona left out when absent.
pub(crate) fn npc_json(npc: &Npc) -> Value {
    let mut out = Map::new();
    out.insert("name".into(), npc.name.clone().into());
    if let Some(persona) = &npc.persona {
        out.insert("persona".into(), persona.clone().into());
    }
    Value::Object(out)
}

fn check(context: &WorldContext) -> Result<(), EmbedError> {
    text("scene", &context.scene, limits::SCENE_MAX_CHARS)?;
    lang("source_lang", &context.source_lang)?;
    lang("target_lang", &context.target_lang)?;
    level("level", context.level)?;
    tags(&context.tags)?;
    context.npc.as_ref().map_or(Ok(()), npc)
}

/// `world.context_changed`'s body: optional fields only when set.
fn body(context: &WorldContext) -> Value {
    let mut data = json!({
        "scene": context.scene,
        "source_lang": context.source_lang,
        "target_lang": context.target_lang,
        "level": context.level,
    });
    if let Some(npc) = &context.npc {
        data["npc"] = npc_json(npc);
    }
    if !context.tags.is_empty() {
        data["tags"] = json!(context.tags);
    }
    if context.generate {
        data["generate"] = json!(true);
    }
    json!({ "type": "world.context_changed", "data": data })
}

impl Embed {
    /// Sends `world.context_changed`. Each send spends one of the player's
    /// inbound cells (60 an hour), so send on a scene change, never per frame.
    /// The answer arrives as `WorldSent` with the returned id.
    pub fn send_context(&self, context: WorldContext) -> Result<RequestId, EmbedError> {
        self.shared.live()?;
        check(&context)?;
        let request = self.shared.ids.next();
        // One key, drawn now, on every retry: the Backend sees one event.
        let key = random_key();
        let shared = std::sync::Arc::clone(&self.shared);
        runtime::spawn(&self.shared, async move {
            let event = match shared.api.send_event(body(&context), key).await {
                Ok(accepted) => EmbedEvent::WorldSent { request, event_id: accepted.event_id, reaction: accepted.reaction },
                Err(error) => EmbedEvent::Error { error: error.with_request(request) },
            };
            shared.outbox.send(event).await;
        });
        Ok(request)
    }
}
