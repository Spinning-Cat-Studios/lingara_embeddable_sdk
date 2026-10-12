//! A value that is a credential: an embed token, or a header the Http token
//! source sends (D4, D10).
//!
//! It deserialises like a string, because a binding has to send one in. Its
//! `Debug` and its `Serialize` both write `[REDACTED]`, so a command, an
//! event or a config that holds one can be logged or echoed whole without
//! leaking it. `expose_secret` is the one way to read it.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// What every rendering of a [`Secret`] writes.
pub const REDACTED: &str = "[REDACTED]";

#[derive(Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema), schemars(transparent))]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The raw value. The one accessor that does not redact.
    pub fn expose_secret(&self) -> &str {
        &self.0
    }
}

impl From<String> for Secret {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for Secret {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(REDACTED)
    }
}

impl Serialize for Secret {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(REDACTED)
    }
}

impl<'de> Deserialize<'de> for Secret {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(Self)
    }
}
