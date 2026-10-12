use serde_json::Value;

use crate::case::{Deliver, DeliverExpect, Handler, HandlerMode, SecretName, Sign};
use crate::sign::{build, secret, signature};

fn vectors() -> Vec<Value> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../vectors/webhook-signatures.json");
    let text = std::fs::read_to_string(path).expect("conformance/vectors/webhook-signatures.json");
    let parsed: Value = serde_json::from_str(&text).expect("the vectors parse");
    parsed["vectors"].as_array().expect("a vectors array").clone()
}

fn field<'a>(v: &'a Value, pointer: &str) -> &'a str {
    v.pointer(pointer).and_then(Value::as_str).unwrap_or_else(|| panic!("{pointer} in {}", v["name"]))
}

// 1.10.26x AC7
#[test]
fn signs_the_upstream_vector() {
    let all = vectors();
    // The Standard Webhooks upstream vector, byte for byte.
    let upstream = all.iter().find(|v| v["name"] == "upstream-vector").expect("the upstream vector");
    let ts: i64 = field(upstream, "/headers/webhook-timestamp").parse().expect("an integer timestamp");
    let got = signature(field(upstream, "/secrets/0"), field(upstream, "/headers/webhook-id"), ts, field(upstream, "/body").as_bytes());
    assert_eq!(got.as_deref(), Ok(field(upstream, "/headers/webhook-signature")));

    // Every single-secret vector Lingara's verifier accepts (header names in any case).
    let header = |v: &Value, name: &str| -> Option<String> {
        let headers = v["headers"].as_object()?;
        headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).and_then(|(_, s)| s.as_str()).map(String::from)
    };
    for v in all.iter().filter(|v| v["expect"].get("ok").is_some() && v["secrets"].as_array().is_some_and(|s| s.len() == 1)) {
        let (Some(id), Some(ts), Some(entries)) = (header(v, "webhook-id"), header(v, "webhook-timestamp"), header(v, "webhook-signature")) else {
            continue;
        };
        let Ok(ts) = ts.parse::<i64>() else { continue };
        let ours = signature(field(v, "/secrets/0"), &id, ts, field(v, "/body").as_bytes()).expect("signs");
        assert!(entries.split(' ').any(|e| e == ours), "{}: {ours} not in {entries}", v["name"]);
    }

    // Two secrets give two v1 entries, newest first, over the exact sent bytes.
    let item = Deliver {
        message: "practice.completed".into(),
        as_type: None,
        body: serde_json::from_str(r#"{"subject":"lgr_sub_x","data":{"player_ref":"p1"}}"#).expect("a body"),
        sign: Sign { secrets: vec![SecretName::Primary, SecretName::Secondary], timestamp_offset_s: 0, tamper: false },
        handler: HandlerMode::Ok,
        router: None,
        expect: DeliverExpect { status: 204, handler: Handler::NOT_CALLED },
    };
    let signed = build("es2.route", 0, &item, 1_790_000_000).expect("builds");
    let header = |name: &str| signed.headers.iter().find(|(n, _)| *n == name).map(|(_, v)| v.clone()).expect(name);
    let (id, ts) = (header("webhook-id"), header("webhook-timestamp").parse::<i64>().expect("ts"));
    let entries: Vec<String> = header("webhook-signature").split(' ').map(String::from).collect();
    let want: Vec<String> = [SecretName::Primary, SecretName::Secondary]
        .iter()
        .map(|s| signature(&secret(*s), &id, ts, &signed.body).expect("signs"))
        .collect();
    assert_eq!(entries, want);
    let sent: Value = serde_json::from_slice(&signed.body).expect("the body is JSON");
    assert_eq!((sent["id"].as_str(), sent["type"].as_str()), (Some(id.as_str()), Some("practice.completed")));
}

/// A receiver that answers 204 to a delivery its primary secret signed and
/// 401 to anything else, the way a server kit's routing must.
async fn receiver() -> u16 {
    use axum::http::{HeaderMap, StatusCode};
    let app = axum::Router::new().fallback(|headers: HeaderMap, body: axum::body::Bytes| async move {
        let get = |n: &str| headers.get(n).and_then(|v| v.to_str().ok()).unwrap_or_default().to_string();
        let ts: i64 = get("webhook-timestamp").parse().unwrap_or_default();
        let want = signature(&secret(SecretName::Primary), &get("webhook-id"), ts, &body).unwrap_or_default();
        if get("webhook-signature").split(' ').any(|e| e == want) { StatusCode::NO_CONTENT } else { StatusCode::UNAUTHORIZED }
    });
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.expect("binds");
    let port = listener.local_addr().expect("addr").port();
    tokio::spawn(async move { axum::serve(listener, app).await });
    port
}

#[tokio::test]
async fn deliveries_post_signed_and_handlers_are_judged() {
    use crate::sign::{deliver_all, handler_mismatches};
    let case = crate::test_support::case(
        "id: es2.route\nbehaviours: [ES2]\ntier: es\ndeliver:\n  - message: practice.completed\n    body: { data: { player_ref: p1 } }\n    sign: { secrets: [primary] }\n    expect: { status: 204, handler: { player_ref: p1 } }\n  - message: practice.completed\n    sign: { secrets: [unknown] }\n    expect: { status: 401, handler: not_called }\n  - message: practice.completed\n    sign: { secrets: [primary], tamper: true }\n    expect: { status: 401, handler: not_called }\n",
    );
    let items = case.deliver.clone().expect("deliveries");
    let url = format!("http://127.0.0.1:{}/", receiver().await);
    assert_eq!(deliver_all(case.id.clone(), items.clone(), url.clone()).await, Vec::<String>::new());

    // A status other than the case's expectation names the case and the item.
    let mut wrong = items;
    wrong[0].expect.status = 200;
    let found = deliver_all(case.id.clone(), wrong, url).await;
    assert_eq!(found, vec!["es2.route: deliver[0] answered 204, expected 200".to_string()]);

    // The handlers the harness reports at finish, in delivery order.
    assert_eq!(handler_mismatches(&case, &[Some("p1".into()), None, None]), Vec::<String>::new());
    assert_eq!(handler_mismatches(&case, &[Some("p2".into()), None, None]).len(), 1);
    assert_eq!(handler_mismatches(&case, &[Some("p1".into())]).len(), 2, "an unreported delivery is a mismatch");
}
