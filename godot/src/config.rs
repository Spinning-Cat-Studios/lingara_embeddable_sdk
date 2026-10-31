//! The node's exported properties into K1's `EmbedConfig` (ADR 1.10.26ab
//! D2). Each property maps to one field; `extra_config` fills only the
//! fields no property owns, so two places never disagree about one value,
//! and never `token_source`, so a header value never sits in a property.

use lingara_embed::{EmbedConfig, EmbedError, ErrorKind};
use serde_json::{Map, Value};

use crate::convert::arg;

/// The kit's own User-Agent token (K1 EK10), ahead of the game's suffix.
pub const KIT_TOKEN: &str = concat!("lingara-godot/", env!("CARGO_PKG_VERSION"));

/// The `EmbedConfig` fields `extra_config` may set.
const EXTRA_FIELDS: [&str; 4] = ["event_types", "resume_cursor", "token_wait_timeout_ms", "stream_idle_timeout_ms"];

/// The property values `start()` reads.
#[derive(Debug, Clone, PartialEq)]
pub struct Props {
    pub base_url: String,
    pub api_version: String,
    pub stream_on_start: bool,
    pub queue_capacity: i64,
    pub log_level: String,
    pub user_agent_suffix: String,
    /// ADR 30.10.26p: empty is none.
    pub embed_version: String,
    pub extra_config: Value,
}

impl Default for Props {
    /// K1's defaults (D2's table, "Default without one").
    fn default() -> Self {
        let k1 = EmbedConfig::default();
        Self {
            base_url: k1.base_url,
            api_version: String::new(),
            stream_on_start: k1.stream_on_start,
            queue_capacity: k1.queue_capacity.into(),
            log_level: k1.log_level.to_string(),
            user_agent_suffix: String::new(),
            embed_version: String::new(),
            extra_config: Value::Object(Map::new()),
        }
    }
}

/// `lingara-godot/<version>`, then the game's suffix when it sets one.
pub fn user_agent(game: &str) -> String {
    match game.trim() {
        "" => KIT_TOKEN.to_owned(),
        game => format!("{KIT_TOKEN} {game}"),
    }
}

fn refuse(message: String) -> EmbedError {
    EmbedError::new(ErrorKind::Config, message)
}

/// `extra_config`'s fields, refusing any a property owns (and `token_source`).
fn extra(extra: &Value) -> Result<Map<String, Value>, EmbedError> {
    let map = match extra {
        Value::Null => return Ok(Map::new()),
        Value::Object(map) => map,
        _ => return Err(refuse("extra_config must be a Dictionary".into())),
    };
    match map.keys().find(|k| !EXTRA_FIELDS.contains(&k.as_str())) {
        Some(key) => Err(refuse(format!("extra_config may not set {key}: a property owns it, or it is not an EmbedConfig field"))),
        None => Ok(map.clone()),
    }
}

impl Props {
    /// The config `start()` hands K1. `token_source` is `token.rs`'s answer.
    pub fn config(&self, token_source: Value) -> Result<EmbedConfig, EmbedError> {
        let mut object = extra(&self.extra_config)?;
        let api_version = Some(self.api_version.trim()).filter(|v| !v.is_empty());
        object.insert("base_url".into(), self.base_url.clone().into());
        object.insert("token_source".into(), token_source);
        object.insert("api_version".into(), api_version.into());
        object.insert("stream_on_start".into(), self.stream_on_start.into());
        object.insert("queue_capacity".into(), arg::<u32>("queue_capacity", self.queue_capacity.into()).map_err(|e| refuse(e.message))?.into());
        object.insert("log_level".into(), self.log_level.clone().into());
        object.insert("user_agent_suffix".into(), user_agent(&self.user_agent_suffix).into());
        let embed_version = Some(self.embed_version.trim()).filter(|v| !v.is_empty());
        object.insert("embed_version".into(), embed_version.into());
        serde_json::from_value(Value::Object(object)).map_err(|e| refuse(format!("not a valid configuration: {e}")))
    }
}
