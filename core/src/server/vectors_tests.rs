//! ES4 and ES6's vectors through this kit, read from the repository root as
//! every server kit reads them.

use serde_json::Value;

use super::handoff::handoff_body;
use super::{SnippetOptions, widget_snippet};

fn vectors(name: &str) -> Value {
    let path = format!("{}/../conformance/vectors/{name}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn options(o: &Value) -> SnippetOptions {
    let get = |k: &str| o.get(k).and_then(Value::as_str).map(String::from);
    SnippetOptions {
        plan_id: get("planId").unwrap_or_default(),
        token_endpoint: get("tokenEndpoint").unwrap_or_default(),
        surface: get("surface"),
        theme: get("theme"),
        locale: get("locale"),
        lingara_origin: get("lingaraOrigin"),
        title: get("title"),
        id: get("id"),
        nonce: get("nonce"),
        element_src: get("elementSrc"),
    }
}

// 1.10.26af AC11
/// Every `snippet.json` vector through `widget_snippet`, and every
/// `token-handoff.json` vector through the internal `handoff_body`, gives its
/// expected answer.
#[test]
fn every_snippet_and_handoff_vector() {
    let snippet = vectors("snippet.json");
    let cases = snippet["cases"].as_array().expect("cases");
    assert!(cases.len() >= 40, "the snippet vectors are all here");
    for case in cases {
        let (name, got) = (&case["name"], widget_snippet(&options(&case["options"])));
        match (&case["expect"]["html"], &case["expect"]["refused"]) {
            (Value::String(html), _) => {
                let want = html.replace("{{version}}", env!("CARGO_PKG_VERSION"));
                assert_eq!(got.as_deref(), Ok(want.as_str()), "{name}");
            }
            (_, Value::String(field)) => assert_eq!(got.map_err(|e| e.field), Err(field.as_str()), "{name}"),
            _ => panic!("{name}: expect is html or refused"),
        }
    }
    let handoff = vectors("token-handoff.json");
    for case in handoff.as_array().expect("a list") {
        let input = &case["input"];
        let got = handoff_body(input["token"].as_str().expect("token"), input["expires_at"].as_str().expect("expires_at"));
        assert_eq!(Some(got.as_str()), case["body"].as_str(), "{}", case["name"]);
    }
}
