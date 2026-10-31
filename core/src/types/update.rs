//! The update notice (ADR 30.10.26p D3): V1's `update_available`, as the
//! handoff and `provide_token` carry it. Its shape is V1's (Backend 30.10.26o
//! §6): all three keys present, `latest` and `message` possibly `null`.
//!
//! A notice is a warning for the developer, so a malformed one is dropped,
//! never fatal: `from_value` is the one door every path takes in.

#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

data! {
    /// The platform's answer to a declared `embed_version`: a newer
    /// version serves, or the declared one is deprecated or withdrawn.
    pub struct UpdateAvailable {
        /// The version now serving, or `None` when no published one serves.
        pub latest: Option<String>,
        pub deprecated: bool,
        /// The developer's message, verbatim, or `None`.
        pub message: Option<String>,
    }
}

impl UpdateAvailable {
    /// The lenient conversion: a malformed value is `None`, with a `Debug`
    /// line through the `log` facade. The token it rode is never refused.
    pub fn from_value(value: Value) -> Option<Self> {
        match Self::parse(&value) {
            Ok(notice) => Some(notice),
            Err(why) => {
                log::debug!(target: "lingara_embed::update", "update_available dropped: {why}");
                None
            }
        }
    }

    /// The strict reading `from_value` forgives.
    pub(crate) fn parse(value: &Value) -> Result<Self, &'static str> {
        let object = value.as_object().ok_or("not an object")?;
        let deprecated = object.get("deprecated").and_then(Value::as_bool).ok_or("deprecated is missing or not a boolean")?;
        let text = |key: &str| match object.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(s)) => Ok(Some(s.clone())),
            Some(_) => Err("latest or message is neither a string nor null"),
        };
        Ok(Self { latest: text("latest")?, deprecated, message: text("message")? })
    }
}

/// `deserialize_with` for the `Command` fields: absent, `null` and malformed
/// all decode to `None`, so the command itself always decodes.
pub(crate) fn lenient<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<UpdateAvailable>, D::Error> {
    Ok(Option::<Value>::deserialize(deserializer)?.filter(|v| !v.is_null()).and_then(UpdateAvailable::from_value))
}
