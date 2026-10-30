//! What the engine thread and the I/O thread share (D2). Session state sits
//! behind plain mutexes that no task holds across an `.await`, so a pure
//! transition (answering a question) is synchronous and returns its new
//! snapshot in the same frame.

use std::sync::Arc;

use crate::api::Api;
use crate::config::EmbedConfig;
use crate::dialogue::Dialogues;
use crate::dialogue::graph::DialogueCache;
use crate::error::{Diagnostics, EmbedError};
use crate::events::StreamControl;
use crate::ids::Ids;
use crate::practice::Practice;
use crate::queue::{Outbox, Queue};
use crate::runtime::IoThread;
use crate::token::HostTokenSource;
use crate::types::{CloseReason, EmbedEvent, LogLevel};

pub(crate) struct Shared {
    pub io: IoThread,
    pub queue: Queue,
    pub outbox: Outbox,
    pub tokens: Arc<HostTokenSource>,
    pub api: Arc<dyn Api>,
    pub ids: Arc<Ids>,
    pub diag: Diagnostics,
    pub config: EmbedConfig,
    pub stream: StreamControl,
    pub practice: Practice,
    pub dialogues: Dialogues,
    /// Stored dialogues by store id (ADR 28.10.26i D3).
    pub dialogue_cache: DialogueCache,
}

impl Shared {
    /// `Closed` once the handle is closed: every command checks it first.
    pub fn live(&self) -> Result<(), EmbedError> {
        if self.queue.is_closed() { Err(EmbedError::closed()) } else { Ok(()) }
    }

    /// Ends the handle: the queue yields `Closed`, the I/O thread drops every
    /// task (and so every connection), and the thread exits. Returns at once.
    pub fn close(&self, reason: CloseReason, preface: Option<EmbedEvent>) {
        if self.queue.close(reason, preface) {
            self.tokens.shutdown();
            self.io.stop();
        }
    }

    /// A panic on the I/O thread: one `Log`, then `Closed { Internal }`.
    pub fn close_internal(&self, message: &str) {
        log::error!(target: "lingara_embed", "{message}");
        let preface = (LogLevel::Error <= self.diag.level)
            .then(|| EmbedEvent::Log { level: LogLevel::Error, target: "lingara_embed".into(), message: message.to_owned() });
        self.close(CloseReason::Internal, preface);
    }
}
