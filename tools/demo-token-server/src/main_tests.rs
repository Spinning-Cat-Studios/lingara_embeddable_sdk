use axum::body::{Body, to_bytes};
use axum::http::{Request, header};
use tower::ServiceExt as _;

use super::*;

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_owned()).collect()
}

fn with_secret(key: &str) -> Option<String> {
    match key {
        "LINGARA_CLIENT_ID" => Some("lgr_ci_demo".into()),
        "LINGARA_CLIENT_SECRET" => Some("demo-secret".into()),
        _ => None,
    }
}

/// 1.10.26z AC14: no secret, no start, unless `--fake`; loopback only;
/// `listening <port>` first; `--fake` answers exactly `{token, expires_at}`,
/// an `lgr_et_` token, and `no-store`.
#[tokio::test]
async fn the_server_needs_a_secret_unless_fake() {
    assert!(options(&[], |_| None).is_err());
    let id_only = |k: &str| (k == "LINGARA_CLIENT_ID").then(|| "lgr_ci_demo".to_owned());
    assert!(options(&[], id_only).is_err());
    let fake = options(&args(&["--fake", "--port", "0"]), |_| None).expect("--fake needs no secret");
    assert_eq!((fake.port, fake.player_ref.as_str(), fake.fake), (0, "demo-player-1", true));
    let real = options(&args(&["--player-ref", "p7"]), with_secret).expect("both set");
    assert_eq!((real.port, real.player_ref.as_str(), real.credentials.is_some()), (8787, "p7", true));

    let listener = bind(0).await.expect("a loopback port");
    let local = listener.local_addr().unwrap();
    assert!(local.ip().is_loopback() && local.port() != 0);
    assert_eq!(listening(local.port()), format!("listening {}", local.port()));

    let request = Request::post("/token").body(Body::empty()).unwrap();
    let response = app(server(&fake).unwrap()).oneshot(request).await.unwrap();
    assert!(response.status().is_success());
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let body = String::from_utf8(to_bytes(response.into_body(), 4096).await.unwrap().to_vec()).unwrap();
    let keys: Vec<&str> = body.trim_matches(['{', '}']).split(',').map(|kv| kv.split(':').next().unwrap()).collect();
    assert_eq!(keys, ["\"token\"", "\"expires_at\""], "{body}");
    assert!(body.starts_with("{\"token\":\"lgr_et_"), "{body}");
    let expires_at = body.split("\"expires_at\":\"").nth(1).unwrap().trim_end_matches("\"}");
    assert!(expires_at.len() == 20 && expires_at.ends_with('Z') && expires_at.as_bytes()[10] == b'T', "{expires_at}");
}
