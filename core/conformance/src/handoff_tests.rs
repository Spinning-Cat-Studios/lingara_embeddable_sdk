//! The device side of the token handoff (D4, D12): every body in V1's
//! `conformance/vectors/token-handoff.json` through the Http source's parser,
//! so the server kits and this crate are held to the same bytes.

use lingara_embed::IssuedToken;
use serde_json::{Value, json};

/// V1's vector: its handoff bodies, each with the token and `expires_at`
/// it must parse to. V1 writes the file; until it lands there is none.
fn vector_bodies() -> Option<Vec<Value>> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance/vectors/token-handoff.json");
    let text = std::fs::read_to_string(path).ok()?;
    let vector: Value = serde_json::from_str(&text).expect("token-handoff.json is JSON");
    let list = vector.as_array().cloned().or_else(|| ["bodies", "cases", "vectors"].iter().find_map(|k| vector[*k].as_array().cloned()));
    Some(list.expect("token-handoff.json holds a list of bodies"))
}

fn parses(body: &Value) {
    // V1's vectors hold each body as its exact text (1.10.26af D6); the
    // inline fixtures below are objects.
    let text = match body.get("body") {
        Some(Value::String(text)) => text.clone(),
        Some(other) => other.to_string(),
        None => body.to_string(),
    };
    let body: Value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{text}: {e}"));
    let issued = IssuedToken::from_handoff_json(text.as_bytes()).unwrap_or_else(|e| panic!("{body}: {e}"));
    assert_eq!(issued.token.expose_secret(), body["token"].as_str().unwrap());
    assert_eq!(issued.expires_at.as_deref(), body["expires_at"].as_str());
}

// 1.10.26y AC31
#[test]
fn the_handoff_vector_parses() {
    match vector_bodies() {
        Some(bodies) => {
            assert!(!bodies.is_empty(), "token-handoff.json holds bodies");
            bodies.iter().for_each(parses);
        }
        None => eprintln!("note: conformance/vectors/token-handoff.json is V1's (1.10.26af) and has not landed; the inline fixtures run alone"),
    }
    // B1's whole six-field answer, forwarded: the extra fields are ignored.
    parses(&json!({
        "token": "lgr_et_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", "expires_at": "2026-10-01T12:15:00Z",
        "expires_in": 900, "subject": "lgr_sub_x", "scopes": ["events:read"], "account_linked": false,
    }));
    parses(&json!({ "token": "lgr_et_BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB", "expires_at": "2026-10-01T12:15:00+02:00" }));
    for bad in [json!({ "token": "lgr_et_x" }), json!({ "expires_at": "2026-10-01T12:15:00Z" }), json!("lgr_et_x")] {
        assert!(IssuedToken::from_handoff_json(bad.to_string().as_bytes()).is_err(), "{bad}");
    }
}
