//! The update notice's latch (ADR 30.10.26p D3): the first accepted token
//! that carries `update_available` emits one `Warn` line and one
//! `UpdateAvailable` event; every later token on the handle emits nothing.
//! A handle is a session, so a new handle is told again.

use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::Diagnostics;
use crate::types::{EmbedEvent, UpdateAvailable};

/// The log line's target, pinned by `conformance/vectors/update-notice.json`.
pub(crate) const TARGET: &str = "lingara_embed::update";

/// V1's schema caps `message` at 280; the line cuts again, as a defence,
/// in Unicode scalar values (JSON Schema's `maxLength` unit).
const MESSAGE_CAP: usize = 280;

pub(crate) struct UpdateLatch {
    told: AtomicBool,
    embed_version: Option<String>,
}

impl UpdateLatch {
    pub fn new(embed_version: Option<String>) -> Self {
        Self { told: AtomicBool::new(false), embed_version }
    }

    /// Called for every accepted token, after it is stored.
    pub fn hear(&self, notice: Option<&UpdateAvailable>, diag: &Diagnostics) {
        let Some(notice) = notice else { return };
        if self.told.swap(true, Ordering::SeqCst) {
            return;
        }
        diag.emit(crate::types::LogLevel::Warn, TARGET, notice_line(notice, self.embed_version.as_deref()));
        let UpdateAvailable { latest, deprecated, message } = notice.clone();
        diag.outbox.push_later(EmbedEvent::UpdateAvailable { latest, deprecated, message });
    }
}

/// The developer-facing line, one template per case. It never compares
/// versions: after a yank `latest` can be *lower* than the declared build.
pub(crate) fn notice_line(notice: &UpdateAvailable, embed_version: Option<&str>) -> String {
    let subject = embed_version.map_or_else(|| "this build's embed version".to_owned(), |v| format!("embed version {v}"));
    let state = if notice.deprecated { "is deprecated" } else { "is not the current version" };
    let current = match &notice.latest {
        Some(latest) => format!("the current version is {latest}"),
        None => "no version is currently published".to_owned(),
    };
    let mut line = format!("lingara: {subject} {state}; {current}");
    if let Some(message) = &notice.message {
        line.push_str(": ");
        line.extend(message.chars().take(MESSAGE_CAP));
    }
    line
}
