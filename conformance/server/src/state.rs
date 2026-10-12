//! The server's shared state: the loaded cases, the one armed case and what
//! it has seen, and the arm/finish log `run` cross-checks against a results
//! file (ADR 1.10.26x D5).

use std::collections::BTreeMap;
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Mutex, MutexGuard};

use tokio::task::JoinHandle;

use crate::case::Case;
use crate::judge::Seen;
use crate::spec::Frozen;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogEntry {
    Arm(String),
    Finish(String),
}

/// The armed case: the next scripted answer, and every mismatch so far.
#[derive(Debug, Default)]
pub struct Armed {
    pub id: String,
    pub cursor: usize,
    pub requests: usize,
    pub mismatches: Vec<String>,
    /// The last request this case answered, for `expect_request.repeat`.
    pub previous: Option<Seen>,
    /// ES deliveries run after the arm answers; finish awaits them.
    pub deliveries: Option<JoinHandle<Vec<String>>>,
}

#[derive(Debug, Default)]
pub struct State {
    pub cases: BTreeMap<String, Case>,
    pub armed: Option<Armed>,
    pub log: Vec<LogEntry>,
    /// Requests that arrived while no case was armed.
    pub stray: Vec<String>,
    /// ES: the harness's webhook receiver, `http://127.0.0.1:<port>/`.
    pub receiver: Option<String>,
}

/// The state, the frozen spec, and the count of streams still connected
/// (EK9 judges that a closed kit drops every one): cheap to clone.
#[derive(Clone)]
pub struct Shared {
    state: Arc<Mutex<State>>,
    pub frozen: Arc<Frozen>,
    pub open_streams: Arc<AtomicUsize>,
}

impl Shared {
    pub fn new(cases: Vec<Case>, frozen: Frozen) -> Self {
        let cases = cases.into_iter().map(|c| (c.id.clone(), c)).collect();
        let state = Arc::new(Mutex::new(State { cases, ..State::default() }));
        Self { state, frozen: Arc::new(frozen), open_streams: Arc::new(AtomicUsize::new(0)) }
    }

    /// A poisoned lock is recovered: a panicking handler must not hide the log.
    pub fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|p| p.into_inner())
    }
}
