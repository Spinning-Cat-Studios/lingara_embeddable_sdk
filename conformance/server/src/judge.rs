//! What the server judges on every request (ADR 1.10.26x D5): the method and
//! path against the case's route, a JSON body against the frozen schema for
//! its `operationId`, the case's `expect_request` headers, and the two tier
//! rules that hold whatever a case says. Every mismatch names its case.

use serde_json::Value;

use crate::case::{Case, ExpectRequest, ServeStep, Tier};
use crate::spec::{path_matches, Frozen};

/// The token a case's `token_grant` route issues, and an ES mint must carry.
pub const GRANT_TOKEN: &str = "lgr_at_conformance0001";
const TOKEN_PATH: &str = "/oauth/token";
/// Credentials that never appear on a player's device (F1, roadmap D2).
const DEVICE_FORBIDDEN: [&str; 3] = ["lgr_at_", "lgr_cs_", "client_secret"];
/// The operations a server kit calls with its client-credentials token.
const ES_AUTHENTICATED: [&str; 2] = ["createEmbedToken", "deleteEmbedPlayer"];

/// A request as the server received it. Header names are lowercase.
#[derive(Debug, Clone, Default)]
pub struct Seen {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Seen {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
    }
}

/// Every mismatch between `seen` and the step it answers. `previous` is the
/// request before it in the same case, which `repeat` compares against.
pub fn judge(case: &Case, step: &ServeStep, seen: &Seen, previous: Option<&Seen>, frozen: &Frozen) -> Vec<String> {
    let id = &case.id;
    let mut found = match &step.route.operation {
        Some(op) => operation_mismatches(op, seen, frozen),
        None => grant_mismatches(seen),
    };
    if let Some(expect) = &step.expect_request {
        found.extend(expect_mismatches(expect, seen, previous));
    }
    found.extend(match case.tier {
        Tier::Ek => device_mismatches(seen),
        Tier::Es => server_mismatches(step, seen),
    });
    found.into_iter().map(|m| format!("{id}: {m}")).collect()
}

/// A step's `expect_request`: exact headers, repeated headers, a body it
/// contains, and the `User-Agent` tail.
fn expect_mismatches(expect: &ExpectRequest, seen: &Seen, previous: Option<&Seen>) -> Vec<String> {
    let mut found = Vec::new();
    for (name, want) in &expect.headers {
        match seen.header(name) {
            None => found.push(format!("missing header {name}")),
            Some(got) if got != want => found.push(format!("header {name} is `{got}`, not `{want}`")),
            Some(_) => {}
        }
    }
    for name in &expect.repeat {
        let before = previous.and_then(|p| p.header(name));
        if before.is_none() || seen.header(name) != before {
            found.push(format!("header {name} is `{:?}`, not the previous request's `{before:?}`", seen.header(name)));
        }
    }
    found.extend(path_and_body_mismatches(expect, seen));
    if let Some(tail) = &expect.user_agent_tail {
        found.extend(user_agent_mismatch(seen.header("user-agent").unwrap_or_default(), tail));
    }
    found
}

/// `path` exactly, a body that contains `body`, and one equal to `body_equals`.
fn path_and_body_mismatches(expect: &ExpectRequest, seen: &Seen) -> Vec<String> {
    let mut found = Vec::new();
    if let Some(want) = expect.path.as_deref().filter(|p| *p != seen.path) {
        found.push(format!("the path is `{}`, not `{want}`", seen.path));
    }
    let got: Value = serde_json::from_slice(&seen.body).unwrap_or(Value::Null);
    if let Some(want) = expect.body.as_ref().filter(|w| !contains(&got, w)) {
        found.push(format!("the body {got} does not contain {want}"));
    }
    let equal = |want: &Value| contains(&got, want) && contains(want, &without_nulls(&got));
    if let Some(want) = expect.body_equals.as_ref().filter(|w| !equal(w)) {
        found.push(format!("the body {got} is not {want}"));
    }
    found
}

/// True when `got` holds every non-null part of `want`; arrays match
/// element by element, at equal length.
pub fn contains(got: &Value, want: &Value) -> bool {
    match (got, want) {
        (_, Value::Null) => true,
        (Value::Object(g), Value::Object(w)) => w.iter().all(|(k, v)| v.is_null() || g.get(k).is_some_and(|x| contains(x, v))),
        (Value::Array(g), Value::Array(w)) => g.len() == w.len() && g.iter().zip(w).all(|(x, v)| contains(x, v)),
        (g, w) => g == w,
    }
}

/// `value` with every null object member dropped, at any depth.
fn without_nulls(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(map.iter().filter(|(_, v)| !v.is_null()).map(|(k, v)| (k.clone(), without_nulls(v))).collect()),
        Value::Array(items) => Value::Array(items.iter().map(without_nulls).collect()),
        other => other.clone(),
    }
}

/// `lingara-<lang>/<v> (…) lingara-embed/<v> [<kit>/<v>] <tail>`: the
/// library's token first, then the core's, then at most one kit token.
fn user_agent_mismatch(agent: &str, tail: &str) -> Option<String> {
    let Some((head, rest)) = agent.split_once(" lingara-embed/") else {
        return Some(format!("user-agent `{agent}` carries no lingara-embed/<version> token"));
    };
    let between: Vec<&str> = rest.split(' ').skip(1).collect();
    let ends = rest.ends_with(&format!(" {tail}")) && between.len() <= 1 + tail.split(' ').count();
    if !head.starts_with("lingara-") || !ends {
        return Some(format!("user-agent `{agent}` is not <library> lingara-embed/<version> [<kit>/<version>] {tail}"));
    }
    None
}

fn operation_mismatches(op: &str, seen: &Seen, frozen: &Frozen) -> Vec<String> {
    let Some(operation) = frozen.operation(op) else {
        return vec![format!("operation {op} is not in the frozen {}", frozen.id)];
    };
    let mut found = Vec::new();
    if seen.method != operation.method || !path_matches(&operation.path, &seen.path) {
        found.push(format!("{} {} is not {op} ({} {})", seen.method, seen.path, operation.method, operation.path));
    }
    if let Some(schema) = operation.body_schema.filter(|_| !seen.body.is_empty()) {
        found.extend(body_mismatches(op, &frozen.schema_root(&schema), &seen.body));
    }
    found
}

fn body_mismatches(op: &str, schema: &serde_json::Value, body: &[u8]) -> Vec<String> {
    let instance: serde_json::Value = match serde_json::from_slice(body) {
        Ok(v) => v,
        Err(e) => return vec![format!("{op}: the body is not JSON: {e}")],
    };
    match jsonschema::validator_for(schema) {
        Ok(validator) => validator.iter_errors(&instance).map(|e| format!("{op}: body {e} at {}", e.instance_path)).collect(),
        Err(e) => vec![format!("{op}: the frozen schema does not compile: {e}")],
    }
}

/// The client-credentials grant: `POST /oauth/token`, `grant_type=client_credentials`.
fn grant_mismatches(seen: &Seen) -> Vec<String> {
    let mut found = Vec::new();
    if seen.method != "POST" || seen.path != TOKEN_PATH {
        found.push(format!("{} {} is not the token grant (POST {TOKEN_PATH})", seen.method, seen.path));
    }
    let body = String::from_utf8_lossy(&seen.body);
    if !body.split('&').any(|pair| pair == "grant_type=client_credentials") {
        found.push("the token grant's grant_type is not client_credentials".into());
    }
    found
}

/// EK: `Bearer lgr_et_…` on every request, and no server credential anywhere.
fn device_mismatches(seen: &Seen) -> Vec<String> {
    let mut found = Vec::new();
    if !seen.header("authorization").is_some_and(|a| a.starts_with("Bearer lgr_et_")) {
        found.push("an ek request carries no Bearer lgr_et_ token".into());
    }
    let body = String::from_utf8_lossy(&seen.body);
    for needle in DEVICE_FORBIDDEN {
        let in_headers = seen.headers.iter().any(|(n, v)| n.contains(needle) || v.contains(needle));
        if in_headers || body.contains(needle) {
            found.push(format!("an ek request carries `{needle}`, which never belongs on a player's device"));
        }
    }
    found
}

/// ES: a mint or delete carries the token the case's grant issued.
fn server_mismatches(step: &ServeStep, seen: &Seen) -> Vec<String> {
    let op = step.route.operation.as_deref().unwrap_or_default();
    let want = format!("Bearer {GRANT_TOKEN}");
    if ES_AUTHENTICATED.contains(&op) && seen.header("authorization") != Some(want.as_str()) {
        return vec![format!("{op} carries no Bearer lgr_at_ token from the case's grant")];
    }
    Vec::new()
}
