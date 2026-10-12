//! ES delivery (ADR 1.10.26x D5): the body is serialised once, then signed
//! and posted with Standard Webhooks headers, one `v1,` entry per signing
//! secret, newest first. The signer is held to the upstream vector in
//! `conformance/vectors/webhook-signatures.json` (E7's), as every server
//! kit's verifier is.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use hmac::{Hmac, Mac};
use serde_json::{Map, Value};
use sha2::Sha256;

use crate::case::{Case, Deliver, Handler, SecretName};

pub const SECRET_PREFIX: &str = "lgr_whsec_";
pub const USER_AGENT: &str = "Lingara-Webhooks/1 (conformance)";

/// The ASCII each conformance secret encodes: 32 visibly fake bytes.
fn secret_bytes(name: SecretName) -> &'static str {
    match name {
        SecretName::Primary => "conformance-embed-secret-0001!!!",
        SecretName::Secondary => "conformance-embed-secret-0002!!!",
        SecretName::Unknown => "conformance-embed-secret-0003!!!",
    }
}

/// `lgr_whsec_` + base64 of the name's fake bytes.
pub fn secret(name: SecretName) -> String {
    format!("{SECRET_PREFIX}{}", STANDARD.encode(secret_bytes(name)))
}

/// What a server kit's receiver is started with: the two secrets it accepts,
/// newest first. `unknown` signs a delivery the receiver must refuse.
pub fn receiver_secrets() -> String {
    format!("{},{}", secret(SecretName::Primary), secret(SecretName::Secondary))
}

/// One `v1,<base64 HMAC-SHA256("id.timestamp.body")>` entry; the key is the
/// base64 after `lgr_whsec_`.
pub fn signature(secret: &str, id: &str, timestamp: i64, body: &[u8]) -> Result<String, String> {
    let encoded = secret.strip_prefix(SECRET_PREFIX).ok_or("a signing secret starts with lgr_whsec_")?;
    let key = STANDARD.decode(encoded).map_err(|e| format!("a signing secret's key is not base64: {e}"))?;
    let mut mac = Hmac::<Sha256>::new_from_slice(&key).map_err(|e| e.to_string())?;
    mac.update(format!("{id}.{timestamp}.").as_bytes());
    mac.update(body);
    Ok(format!("v1,{}", STANDARD.encode(mac.finalize().into_bytes())))
}

/// A delivery ready to post: its headers (lowercase names) and exact bytes.
#[derive(Debug)]
pub struct Signed {
    pub headers: Vec<(&'static str, String)>,
    pub body: Vec<u8>,
}

/// The `n`th delivery of `case_id` at `now` (Unix seconds).
pub fn build(case_id: &str, n: usize, item: &Deliver, now: i64) -> Result<Signed, String> {
    let id = format!("lgr_evt_{}_{n}", case_id.replace(['.', '-'], "_"));
    let mut envelope = Map::new();
    envelope.insert("id".into(), Value::from(id.clone()));
    envelope.insert("type".into(), Value::from(item.as_type.clone().unwrap_or_else(|| item.message.clone())));
    envelope.extend(item.body.iter().filter(|(k, _)| *k != "id" && *k != "type").map(|(k, v)| (k.clone(), v.clone())));
    let mut body = serde_json::to_vec(&envelope).map_err(|e| e.to_string())?;
    let timestamp = now + item.sign.timestamp_offset_s;
    let entries: Vec<String> =
        item.sign.secrets.iter().map(|s| signature(&secret(*s), &id, timestamp, &body)).collect::<Result<_, _>>()?;
    if item.sign.tamper {
        let last = body.last_mut().ok_or("tamper needs a body")?;
        *last ^= 0x01;
    }
    let headers = vec![
        ("webhook-id", id),
        ("webhook-timestamp", timestamp.to_string()),
        ("webhook-signature", entries.join(" ")),
        ("content-type", "application/json".to_string()),
        ("user-agent", USER_AGENT.to_string()),
    ];
    Ok(Signed { headers, body })
}

/// Posts each delivery to `receiver` in order; a status other than the
/// case's expectation is a mismatch naming the case.
pub async fn deliver_all(case_id: String, items: Vec<Deliver>, receiver: String) -> Vec<String> {
    let client = reqwest::Client::new();
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    let mut found = Vec::new();
    for (n, item) in items.iter().enumerate() {
        let signed = match build(&case_id, n, item, now) {
            Ok(s) => s,
            Err(e) => {
                found.push(format!("{case_id}: deliver[{n}]: {e}"));
                continue;
            }
        };
        let mut request = client.post(&receiver).body(signed.body);
        for (name, value) in signed.headers {
            request = request.header(name, value);
        }
        match request.send().await {
            Ok(r) if r.status().as_u16() == item.expect.status => {}
            Ok(r) => found.push(format!("{case_id}: deliver[{n}] answered {}, expected {}", r.status().as_u16(), item.expect.status)),
            Err(e) => found.push(format!("{case_id}: deliver[{n}] did not reach the receiver: {e}")),
        }
    }
    found
}

/// Each delivery's `expect.handler` against what the harness reported at finish.
pub fn handler_mismatches(case: &Case, reported: &[Option<String>]) -> Vec<String> {
    let items = case.deliver.as_deref().unwrap_or_default();
    let mut found = Vec::new();
    for (n, item) in items.iter().enumerate() {
        let got = reported.get(n);
        let ok = match (&item.expect.handler, got) {
            // The server sees only the reported player; a handler called with
            // none reports `null` too, and the harness judges the rest (D11).
            (Handler::Word(_), Some(None)) => true,
            (Handler::Called(call), Some(seen)) => call.player_ref == *seen,
            _ => false,
        };
        if !ok {
            found.push(format!("{}: deliver[{n}] handler {:?}, expected {:?}", case.id, got, item.expect.handler));
        }
    }
    found
}
