use crate::judge::{judge, Seen, GRANT_TOKEN};
use crate::test_support::{case, frozen};

const EK: &str = r#"
id: ek1.mint-and-stream
behaviours: [EK1]
tier: ek
serve:
  - route: { operation: sendEvent }
    respond: { status: 202 }
  - route: { operation: streamEvents }
    expect_request: { headers: { last-event-id: "e1" } }
    respond: { sse: [] }
"#;

const ES: &str = r#"
id: es1.mint
behaviours: [ES1]
tier: es
serve:
  - route: { token_grant: {} }
    respond: { json: { access_token: "lgr_at_conformance0001" } }
  - route: { operation: createEmbedToken }
    respond: { json: {} }
"#;

fn seen(method: &str, path: &str, auth: Option<&str>, body: &str) -> Seen {
    let headers = auth.map(|a| vec![("authorization".to_string(), a.to_string())]).unwrap_or_default();
    Seen { method: method.into(), path: path.into(), headers, body: body.as_bytes().to_vec() }
}

fn found(yaml: &str, step: usize, request: &Seen) -> Vec<String> {
    let c = case(yaml);
    judge(&c, &c.serve[step], request, None, &frozen())
}

fn one(yaml: &str, step: usize, request: &Seen, needle: &str) {
    let got = found(yaml, step, request);
    assert!(got.iter().any(|m| m.contains(needle)), "no `{needle}` in {got:?}");
    let id = &case(yaml).id;
    assert!(got.iter().all(|m| m.starts_with(&format!("{id}: "))), "a mismatch does not name {id}: {got:?}");
}

// 1.10.26x AC5
#[test]
fn every_mismatch_names_its_case() {
    let et = Some("Bearer lgr_et_conformance0001");
    let event = r#"{"type":"world.practice_completed"}"#;
    // A clean EK request is no mismatch.
    assert_eq!(found(EK, 0, &seen("POST", "/v1/events", et, event)), Vec::<String>::new());

    // A body that fails its operation's schema in the frozen current.
    one(EK, 0, &seen("POST", "/v1/events", et, r#"{"data":{}}"#), "sendEvent: body");
    let es_auth = format!("Bearer {GRANT_TOKEN}");
    one(ES, 1, &seen("POST", "/v1/embed/tokens", Some(&es_auth), r#"{"player_ref":""}"#), "createEmbedToken: body");
    one(ES, 1, &seen("POST", "/v1/embed/tokens", Some(&es_auth), r#"{"player_ref":"p","extra":1}"#), "createEmbedToken: body");

    // EK: no Bearer lgr_et_, an lgr_at_ token, a client_secret in the body.
    one(EK, 0, &seen("POST", "/v1/events", None, event), "no Bearer lgr_et_");
    one(EK, 0, &seen("POST", "/v1/events", Some("Bearer lgr_at_abc"), event), "carries `lgr_at_`");
    one(EK, 0, &seen("POST", "/v1/events", et, r#"{"type":"x","client_secret":"s"}"#), "carries `client_secret`");

    // ES: a mint without the grant's Bearer lgr_at_.
    one(ES, 1, &seen("POST", "/v1/embed/tokens", Some("Bearer lgr_et_x"), r#"{"player_ref":"p"}"#), "no Bearer lgr_at_");
    let clean = found(ES, 1, &seen("POST", "/v1/embed/tokens", Some(&es_auth), r#"{"player_ref":"p"}"#));
    assert_eq!(clean, Vec::<String>::new());

    // A token grant whose grant_type is not client_credentials.
    one(ES, 0, &seen("POST", "/oauth/token", None, "grant_type=authorization_code"), "not client_credentials");
    assert_eq!(found(ES, 0, &seen("POST", "/oauth/token", None, "grant_type=client_credentials&scope=embed%3Amint")), Vec::<String>::new());

    // A reconnect without Last-Event-ID.
    one(EK, 1, &seen("GET", "/v1/events/stream", et, ""), "missing header last-event-id");

    // The wrong route.
    one(EK, 1, &seen("GET", "/v1/events", et, ""), "is not streamEvents");
}

const EXPECT: &str = r#"
id: ek4.key-survives-a-503
behaviours: [EK4]
tier: ek
serve:
  - route: { operation: sendEvent }
    respond: { status: 503 }
  - route: { operation: sendEvent }
    expect_request:
      repeat: [idempotency-key]
      body: { type: world.context_changed, data: { level: 2 } }
      user_agent_tail: "conformance/1.0"
    respond: { status: 202 }
"#;

fn with(headers: &[(&str, &str)], body: &str) -> Seen {
    let mut s = seen("POST", "/v1/events", Some("Bearer lgr_et_conformance0001"), body);
    s.headers.extend(headers.iter().map(|(n, v)| ((*n).to_owned(), (*v).to_owned())));
    s
}

// 1.10.26y AC21 (K1's expect_request fields)
#[test]
fn k1_expectations_judge_repeat_body_and_user_agent() {
    let c = case(EXPECT);
    let body = r#"{"type":"world.context_changed","data":{"scene":"m","source_lang":"en","target_lang":"zh","level":2}}"#;
    let ua = "lingara-rust/0.2.0 (rust/1.91.1; aarch64-apple-darwin) lingara-embed/0.1.0 conformance/1.0";
    let first = with(&[("idempotency-key", "k1")], body);
    let good = with(&[("idempotency-key", "k1"), ("user-agent", ua)], body);
    assert_eq!(judge(&c, &c.serve[1], &good, Some(&first), &frozen()), Vec::<String>::new());

    let kit = ua.replace("lingara-embed/0.1.0 ", "lingara-embed/0.1.0 lingara-godot/0.1.0 ");
    assert_eq!(judge(&c, &c.serve[1], &with(&[("idempotency-key", "k1"), ("user-agent", &kit)], body), Some(&first), &frozen()), Vec::<String>::new());

    let other_key = with(&[("idempotency-key", "k2"), ("user-agent", ua)], body);
    let level_3 = with(&[("idempotency-key", "k1"), ("user-agent", ua)], &body.replace("\"level\":2", "\"level\":3"));
    let two_kits = ua.replace("lingara-embed/0.1.0 ", "lingara-embed/0.1.0 a/1 b/1 ");
    let bad_ua = with(&[("idempotency-key", "k1"), ("user-agent", &two_kits)], body);
    for (request, needle) in [(other_key, "not the previous request's"), (level_3, "does not contain"), (bad_ua, "is not <library>")] {
        let got = judge(&c, &c.serve[1], &request, Some(&first), &frozen());
        assert!(got.iter().any(|m| m.contains(needle)), "no `{needle}` in {got:?}");
    }
}
