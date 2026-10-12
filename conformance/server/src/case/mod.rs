//! The closed case format for both tiers (ADR 1.10.26x D4).
//!
//! One file is one case, `conformance/<tier>/cases/<group>/<slug>.yaml`, with
//! id `<group>.<slug>`. Every struct denies unknown fields, so a misspelt key
//! is a refusal rather than a silently ignored line. A step, fault or `expect`
//! field a child needs is added here, in that child's own change.

mod es;
mod host;
mod load;
#[cfg(test)]
mod mod_tests;

pub use es::{Directory, Handler, HandlerMode, KitStep, Minted, RaisedError, Router};
#[cfg(test)]
pub use es::{Called, DeleteJob, MintJob};
pub use host::HostStep;
pub use load::load_dirs;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    Ek,
    Es,
}

impl Tier {
    pub fn name(self) -> &'static str {
        match self {
            Tier::Ek => "ek",
            Tier::Es => "es",
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub id: String,
    pub behaviours: Vec<String>,
    pub tier: Tier,
    /// The server's scripted answers, consumed in order.
    #[serde(default)]
    pub serve: Vec<ServeStep>,
    /// EK only: what the harness does through its kit's surface, in order.
    /// Each step is a one-key map (`- start: {…}`), not a YAML tag.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "serde_yaml::with::singleton_map_recursive")]
    pub host: Option<Vec<HostStep>>,
    /// ES only: the signed posts the server makes to the kit's receiver.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deliver: Option<Vec<Deliver>>,
    /// ES only: the jobs the harness performs through its kit, in order.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "serde_yaml::with::singleton_map_recursive")]
    pub kit: Option<Vec<KitStep>>,
    /// ES only: the harness's player directory before the case.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub directory: Option<Directory>,
    #[serde(default)]
    pub expect: Expect,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ServeStep {
    pub route: Route,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expect_request: Option<ExpectRequest>,
    pub respond: Respond,
}

/// `{ operation: <operations.toml entry> }` or `{ token_grant: {} }`, never both.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Route {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_grant: Option<Empty>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Empty {}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectRequest {
    /// Header name (any case) → exact value.
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    /// Headers whose value equals the previous request's (K1: EK4's one
    /// `Idempotency-Key` across a retry).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub repeat: Vec<String>,
    /// A JSON object the body must contain, compared after dropping nulls
    /// (K1: EK5's report, EK6's window).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<Value>,
    /// The exact raw (percent-encoded) path (1.10.26af: ES2's one segment).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// A JSON value the body equals, after dropping nulls: `body` cannot
    /// say a key is absent (1.10.26af: ES1's omitted options).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_equals: Option<Value>,
    /// The `User-Agent` ends `lingara-embed/<version> [<kit>/<version>] <tail>`
    /// after the library's own token (K1: EK10).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_agent_tail: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Respond {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub json: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sse: Option<Vec<Frame>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fault: Option<Fault>,
}

/// One SSE frame: an event (`data`, with an optional `id` and `event`), or a
/// comment, which is how B2 writes a heartbeat.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Fault {
    /// Abort the connection after this many event frames (comments do not count).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drop_after_events: Option<usize>,
    /// After the frames, hold the stream open and silent this long, then close.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heartbeat_gap_s: Option<f64>,
    /// Write each frame in two writes this far apart.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delay_fragment_ms: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Deliver {
    /// An `operations.toml` message entry, e.g. `practice.completed`.
    pub message: String,
    /// ES: the envelope's `type` when it is not `message`'s own: a type
    /// newer than the libraries, which must still be routed (ES3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub as_type: Option<String>,
    /// The envelope's fields beside `id` and `type`, which the server sets.
    #[serde(default)]
    pub body: serde_json::Map<String, Value>,
    pub sign: Sign,
    /// ES: what the harness's `onEvent` does for this delivery.
    #[serde(default)]
    pub handler: HandlerMode,
    /// ES: the secrets this delivery's `PlayerRouter` holds; absent, `[primary]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub router: Option<Router>,
    pub expect: DeliverExpect,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Sign {
    pub secrets: Vec<SecretName>,
    #[serde(default)]
    pub timestamp_offset_s: i64,
    /// Flip the body's last byte after signing.
    #[serde(default)]
    pub tamper: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SecretName {
    Primary,
    Secondary,
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeliverExpect {
    pub status: u16,
    pub handler: Handler,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Expect {
    /// What the host saw, JSON-equal after dropping nulls; the harness judges it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub events: Option<Vec<Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requests: Option<RequestCount>,
    /// ES: the harness's directory after the case, subject → player_ref.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub directory: Option<BTreeMap<String, String>>,
    /// ES: how many `playerRefFor` calls the harness's directory saw.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub directory_lookups: Option<usize>,
    /// ES: the last minted token, in wire form.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minted: Option<Minted>,
    /// ES6: `tokenHandoff`'s exact text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handoff: Option<String>,
    /// ES: the error a kit job raised.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<RaisedError>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RequestCount {
    pub count: usize,
}

/// The rules serde cannot state: one route form, one frame form, and each
/// tier's own steps only.
pub fn shape_problems(case: &Case) -> Vec<String> {
    let id = &case.id;
    let mut problems = Vec::new();
    problems.extend(tier_problems(case));
    for (n, step) in case.serve.iter().enumerate() {
        if step.route.operation.is_some() == step.route.token_grant.is_some() {
            problems.push(format!("{id}: serve[{n}].route names exactly one of operation and token_grant"));
        }
        let frames = step.respond.sse.iter().flatten();
        for frame in frames.filter(|f| f.comment.is_some() == f.data.is_some()) {
            problems.push(format!("{id}: serve[{n}]: an sse frame is a comment or has data, not {frame:?}"));
        }
        if step.respond.sse.is_some() && step.respond.json.is_some() {
            problems.push(format!("{id}: serve[{n}].respond is json or sse, not both"));
        }
    }
    let steps = case.host.iter().flatten().enumerate();
    problems.extend(steps.filter_map(|(n, step)| host::step_problems(id, n, step)));
    problems
}

/// Each tier's own keys only: EK's `host:`, ES's `deliver:`, `kit:` and `directory:`.
fn tier_problems(case: &Case) -> Vec<String> {
    let id = &case.id;
    let es_only = [(case.deliver.is_some(), "deliver:"), (case.kit.is_some() || case.directory.is_some(), "kit: or directory:")];
    match case.tier {
        Tier::Ek => es_only.iter().filter(|(present, _)| *present).map(|(_, keys)| format!("{id}: an ek case has no {keys}")).collect(),
        Tier::Es if case.host.is_some() => vec![format!("{id}: an es case has no host:")],
        Tier::Es => Vec::new(),
    }
}
