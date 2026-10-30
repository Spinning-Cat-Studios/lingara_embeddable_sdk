//! A dialogue loaded by its store id (ADR 28.10.26i D2, D3): the load task,
//! the one-fetch-per-id map and the cache, all decided on the I/O thread so
//! the freshness window reads one clock (tokio's, paused in the tests).
//!
//! A load is planned under the cache's lock: a fresh entry starts the story
//! at once, a fetch already in flight for the id takes the dialogue as one
//! more waiter, and anything else fetches, sending the cached `etag` back as
//! `If-None-Match`. The fetch's outcome reaches every waiter, each started or
//! failed on its own. A running story keeps its own `Arc`, so eviction and a
//! newer revision never touch it.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use lingara_dialogue_ir::DialogueIr;
use tokio::time::Instant;

use crate::api::Exported;
use crate::dialogue::Dialogue;
use crate::dialogue::graph::{TARGET, emit, session::open_run};
use crate::error::{EmbedError, ErrorKind};
use crate::limits::{DIALOGUE_ACTIVE_TTL_MS, DIALOGUE_CACHE_MAX_ENTRIES};
use crate::shared::Shared;
use crate::sync::lock;
use crate::types::{DialogueId, EmbedEvent, FactValue, GraphSession};

/// What a `loading` dialogue holds until its revision arrives: the session
/// it was loaded with, and the facts set since, in order.
pub(crate) struct Pending {
    pub session: GraphSession,
    pub held: Vec<(String, FactValue)>,
}

/// The store id as the core keys and sends it: UUID-shaped, lowercased.
pub(crate) fn store_id(raw: &str) -> Result<String, EmbedError> {
    let hyphen_at = |i: usize| matches!(i, 8 | 13 | 18 | 23);
    let shaped = raw.len() == 36 && raw.chars().enumerate().all(|(i, c)| if hyphen_at(i) { c == '-' } else { c.is_ascii_hexdigit() });
    if !shaped {
        return Err(EmbedError::invalid("a stored dialogue id is a UUID: 8-4-4-4-12 hex digits"));
    }
    Ok(raw.to_ascii_lowercase())
}

struct Entry {
    rev: u64,
    etag: String,
    ir: Arc<DialogueIr>,
    fetched_at: Instant,
    /// The cache's tick when last used: the smallest is evicted first.
    used: u64,
}

/// A revision as each waiting dialogue takes it.
#[derive(Clone)]
struct Loaded {
    rev: u64,
    ir: Arc<DialogueIr>,
}

enum Plan {
    Ready(Loaded),
    Wait,
    Fetch(Option<String>),
}

#[derive(Default)]
struct Inner {
    entries: HashMap<String, Entry>,
    in_flight: HashMap<String, Vec<DialogueId>>,
    tick: u64,
}

/// One per handle, in memory only (D3).
#[derive(Default)]
pub(crate) struct DialogueCache {
    inner: Mutex<Inner>,
}

#[cfg(test)]
impl DialogueCache {
    /// The cached IR for a store id, without touching its recency.
    pub(crate) fn peek(&self, id: &str) -> Option<Arc<DialogueIr>> {
        lock(&self.inner).entries.get(id).map(|e| Arc::clone(&e.ir))
    }
}

impl Inner {
    fn touch(&mut self, id: &str) -> Option<&mut Entry> {
        self.tick += 1;
        let tick = self.tick;
        let entry = self.entries.get_mut(id)?;
        entry.used = tick;
        Some(entry)
    }

    fn plan(&mut self, id: &str, dialogue: DialogueId, now: Instant) -> Plan {
        let ttl = Duration::from_millis(DIALOGUE_ACTIVE_TTL_MS);
        if let Some(entry) = self.touch(id).filter(|e| now.duration_since(e.fetched_at) < ttl) {
            return Plan::Ready(Loaded { rev: entry.rev, ir: Arc::clone(&entry.ir) });
        }
        if let Some(waiters) = self.in_flight.get_mut(id) {
            waiters.push(dialogue);
            return Plan::Wait;
        }
        self.in_flight.insert(id.to_owned(), vec![dialogue]);
        Plan::Fetch(self.entries.get(id).map(|e| e.etag.clone()))
    }

    /// The fetch's answer, applied: the waiters it reaches, what each gets,
    /// and the IR's load warnings.
    fn settle(&mut self, id: &str, answer: Result<Exported, EmbedError>, now: Instant) -> (Vec<DialogueId>, Result<Loaded, EmbedError>, Vec<String>) {
        let waiters = self.in_flight.remove(id).unwrap_or_default();
        let (loaded, warnings) = match answer {
            Ok(Exported::Body { rev, etag, ir_json }) => match self.body(id, rev, etag, &ir_json, now) {
                Ok((loaded, warnings)) => (Ok(loaded), warnings),
                Err(error) => (Err(error), Vec::new()),
            },
            Ok(Exported::NotModified { rev, etag }) => (self.not_modified(id, rev, etag, now), Vec::new()),
            Err(error) => (Err(error), Vec::new()),
        };
        (waiters, loaded, warnings)
    }

    /// A `200`: an unchanged revision keeps its parsed `Arc`; a new one is
    /// parsed and validated, and cached only if K1 accepts it. Answers the
    /// IR's load warnings beside it.
    fn body(&mut self, id: &str, rev: u64, etag: String, ir_json: &str, now: Instant) -> Result<(Loaded, Vec<String>), EmbedError> {
        if let Some(entry) = self.touch(id).filter(|e| e.rev == rev && e.etag == etag) {
            entry.fetched_at = now;
            return Ok((Loaded { rev, ir: Arc::clone(&entry.ir) }, Vec::new()));
        }
        let (ir, found) = lingara_dialogue_ir::load(ir_json).map_err(|e| EmbedError::invalid(format!("stored dialogue {id} revision {rev} is refused: {e}")))?;
        let ir = Arc::new(ir);
        self.insert(id, Entry { rev, etag, ir: Arc::clone(&ir), fetched_at: now, used: self.tick });
        Ok((Loaded { rev, ir }, found.iter().map(ToString::to_string).collect()))
    }

    /// A `304`: the cached revision is fresh again.
    fn not_modified(&mut self, id: &str, rev: u64, etag: String, now: Instant) -> Result<Loaded, EmbedError> {
        let Some(entry) = self.touch(id) else {
            return Err(EmbedError::new(ErrorKind::Network, format!("stored dialogue {id} answered 304 with nothing cached")));
        };
        (entry.rev, entry.etag, entry.fetched_at) = (rev, etag, now);
        Ok(Loaded { rev, ir: Arc::clone(&entry.ir) })
    }

    fn insert(&mut self, id: &str, entry: Entry) {
        if !self.entries.contains_key(id) && self.entries.len() >= DIALOGUE_CACHE_MAX_ENTRIES {
            let oldest = self.entries.iter().min_by_key(|(_, e)| e.used).map(|(k, _)| k.clone());
            oldest.iter().for_each(|k| drop(self.entries.remove(k)));
        }
        self.entries.insert(id.to_owned(), entry);
    }
}

/// The load task for `dialogue`, on the I/O thread. It never retries.
pub(crate) async fn load(shared: Arc<Shared>, dialogue: DialogueId, id: String) {
    let plan = lock(&shared.dialogue_cache.inner).plan(&id, dialogue, Instant::now());
    let (waiters, loaded) = match plan {
        Plan::Ready(loaded) => (vec![dialogue], Ok(loaded)),
        Plan::Wait => return,
        Plan::Fetch(etag) => {
            // Still completes when every waiter has closed: the cache is filled for the next load.
            let answer = shared.api.export_dialogue(id.clone(), etag).await;
            let (waiters, loaded, warnings) = lock(&shared.dialogue_cache.inner).settle(&id, answer, Instant::now());
            warnings.into_iter().for_each(|warning| shared.diag.warn(TARGET, warning));
            (waiters, loaded)
        }
    };
    for waiter in waiters {
        finish(&shared, waiter, &id, loaded.clone()).await;
    }
}

/// Starts `dialogue` on its revision, or removes it, unless it was closed
/// while loading. Level and scene set while loading are kept.
async fn finish(shared: &Shared, dialogue: DialogueId, id: &str, loaded: Result<Loaded, EmbedError>) {
    let started = {
        let mut map = lock(&shared.dialogues.map);
        let Some(d) = map.get_mut(&dialogue) else { return };
        let Some(pending) = d.pending.take() else { return };
        let mut session = pending.session;
        (session.level, session.scene) = (d.config.level, d.config.scene.clone());
        let started = loaded.and_then(|l| open_run(l.ir, session, &pending.held).map(|(state, config)| (l.rev, state, config)));
        match started {
            Ok((rev, mut state, config)) => {
                let steps = state.start();
                *d = Dialogue::new(config, Some(state));
                Ok((rev, steps))
            }
            Err(error) => {
                map.remove(&dialogue);
                Err(error)
            }
        }
    };
    match started {
        Ok((rev, steps)) => {
            shared.outbox.send(EmbedEvent::DialogueLoaded { dialogue, dialogue_id: id.to_owned(), rev }).await;
            emit(shared, dialogue, steps);
        }
        Err(error) => shared.outbox.send(EmbedEvent::DialogueLoadFailed { dialogue, error }).await,
    }
}
