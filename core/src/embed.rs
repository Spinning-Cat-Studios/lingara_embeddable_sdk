//! The `Embed` handle (D3): `Send + Sync`, every method `&self`, none of
//! them blocking or awaiting. Its commands are the flat methods spread over
//! the module that owns each (`commands.rs`, `world.rs`, `practice/`,
//! `dialogue/`); this file starts it, stops it, and polls it.

use std::sync::Arc;
use std::time::Duration;

use crate::api::Api;
use crate::config::{EmbedConfig, TokenSourceConfig};
use crate::dialogue::Dialogues;
use crate::dialogue::graph::DialogueCache;
use crate::error::{Diagnostics, EmbedError};
use crate::events::StreamControl;
use crate::handoff::EmbedTokenSource;
use crate::ids::Ids;
use crate::library::LibraryApi;
use crate::practice::Practice;
use crate::queue;
use crate::runtime::IoThread;
use crate::shared::Shared;
use crate::token::{HostTokenSource, UpdateLatch};
use crate::token_fetch::{Mode, http_mode};
use crate::types::{CloseReason, EmbedEvent, EmbedStatus};

/// Builds the port a started handle talks through: the library, or a test's fake.
pub(crate) type ApiFactory = Box<dyn FnOnce(&EmbedConfig, Arc<HostTokenSource>) -> Result<Arc<dyn Api>, EmbedError>>;

fn library(config: &EmbedConfig, tokens: Arc<HostTokenSource>) -> Result<Arc<dyn Api>, EmbedError> {
    Ok(Arc::new(LibraryApi::new(config, tokens)?))
}

/// One player's connection to Lingara. Create one per game.
pub struct Embed {
    pub(crate) shared: Arc<Shared>,
}

impl Embed {
    /// Validates `config`, starts the `lingara-embed` thread, and (unless
    /// `stream_on_start` is false) opens the stream.
    pub fn start(config: EmbedConfig) -> Result<Embed, EmbedError> {
        Self::launch(config, None, Box::new(library), false)
    }

    /// `start` with a Rust token source, awaited on the core's own thread.
    /// Rust-only: not a `Command`, never on the FFI surface.
    pub fn start_with_source(config: EmbedConfig, source: Arc<dyn EmbedTokenSource>) -> Result<Embed, EmbedError> {
        Self::launch(config, Some(source), Box::new(library), false)
    }

    /// `paused` starts the I/O thread's clock paused: the unit tests only.
    pub(crate) fn launch(config: EmbedConfig, source: Option<Arc<dyn EmbedTokenSource>>, api: ApiFactory, paused: bool) -> Result<Embed, EmbedError> {
        config.validate()?;
        let mode = match (source, &config.token_source) {
            (Some(source), _) => Mode::Trait(source),
            (None, TokenSourceConfig::Http { url, headers }) => http_mode(url, headers, config.embed_version.as_deref())?,
            (None, _) => Mode::Poll,
        };
        let io = IoThread::spawn(paused)?;
        let (queue, outbox) = queue::channel(config.queue_capacity, io.handle().clone());
        let diag = Diagnostics { level: config.log_level, outbox: outbox.clone() };
        let ids = Arc::new(Ids::default());
        let wait = Duration::from_millis(config.token_wait_timeout_ms.into());
        let update = UpdateLatch::new(config.embed_version.clone());
        let tokens = HostTokenSource::new(mode, diag.clone(), Arc::clone(&ids), wait, update);
        let api = match api(&config, Arc::clone(&tokens)) {
            Ok(api) => api,
            Err(e) => {
                io.stop();
                return Err(e);
            }
        };
        let stream = StreamControl::new(config.resume_cursor.clone());
        let (practice, dialogues) = (Practice::default(), Dialogues::default());
        let dialogue_cache = DialogueCache::default();
        let shared = Arc::new(Shared { io, queue, outbox, tokens, api, ids, diag, config, stream, practice, dialogues, dialogue_cache });
        shared.tokens.ask();
        if shared.config.stream_on_start {
            shared.stream.resume(&shared);
        }
        Ok(Embed { shared })
    }

    pub fn status(&self) -> EmbedStatus {
        let (stream, cursor) = self.shared.stream.state();
        EmbedStatus { stream, cursor, needs_token: self.shared.tokens.needs_token(), closed: self.shared.queue.is_closed() }
    }

    /// Signals the I/O thread and returns at once. The queue then yields one
    /// `Closed`, and every command returns `Closed`.
    pub fn close(&self) {
        self.shared.close(CloseReason::Requested, None);
    }

    /// `close`, then joins the I/O thread for at most `timeout_ms`; true when
    /// it exited in time, so the host may unload the library.
    pub fn close_and_wait(&self, timeout_ms: u32) -> bool {
        self.close();
        self.shared.io.join(Duration::from_millis(timeout_ms.into()))
    }

    pub fn poll_event(&self) -> Option<EmbedEvent> {
        self.shared.queue.poll()
    }

    /// At most `max` events. Bindings cap a frame at 256.
    pub fn drain(&self, max: u32) -> Vec<EmbedEvent> {
        self.shared.queue.drain(max)
    }
}

/// `close()` without the join, so dropping the handle never stalls a frame.
impl Drop for Embed {
    fn drop(&mut self) {
        self.close();
    }
}

impl std::fmt::Debug for Embed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Embed").field("base_url", &self.shared.config.base_url).field("closed", &self.shared.queue.is_closed()).finish()
    }
}
