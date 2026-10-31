use crate::case::{shape_problems, Case, Handler, HostStep, Tier};
use crate::test_support::case;

const EK: &str = r#"
id: ek3.resume-after-drop
behaviours: [EK3]
tier: ek
serve:
  - route: { operation: streamEvents }
    respond:
      headers: { x-trace: "1" }
      sse:
        - { id: "e1", event: "lesson_plan.ready", data: { plan_id: "p" } }
        - { comment: "heartbeat" }
      fault: { drop_after_events: 1, heartbeat_gap_s: 0.5, delay_fragment_ms: 5 }
  - route: { operation: streamEvents }
    expect_request: { headers: { last-event-id: "e1" } }
    respond: { status: 401, json: { error: "unauthorized" } }
host:
  - start: { initial_token: none }
  - provide_token: { token: "lgr_et_conformance0001", expires_in_secs: 900 }
  - command: { type: stream_pause }
  - poll_until: { event: stream, within_ms: 2000 }
  - close: {}
expect:
  events: [ { type: live } ]
  requests: { count: 2 }
"#;

const ES: &str = r#"
id: es2.route-by-player
behaviours: [ES2]
tier: es
serve:
  - route: { token_grant: {} }
    respond: { json: { access_token: "lgr_at_conformance0001", token_type: "Bearer", expires_in: 3600 } }
  - route: { operation: createEmbedToken }
    respond: { json: { token: "lgr_et_x" } }
deliver:
  - message: practice.completed
    body: { subject: "lgr_sub_x", data: { player_ref: "p1" } }
    sign: { secrets: [primary, secondary], timestamp_offset_s: -10, tamper: false }
    expect: { status: 204, handler: { player_ref: p1 } }
  - message: practice.completed
    sign: { secrets: [unknown] }
    expect: { status: 401, handler: not_called }
"#;

fn refused(yaml: &str) -> String {
    serde_yaml::from_str::<Case>(yaml).expect_err("this case must be refused").to_string()
}

// 1.10.26x AC4
#[test]
fn the_case_format_is_closed_per_tier() {
    // Every key of both tiers deserialises, both route forms included.
    let ek = case(EK);
    assert_eq!(ek.tier, Tier::Ek);
    assert!(matches!(ek.host.as_deref(), Some([HostStep::Start(_), HostStep::ProvideToken(_), HostStep::Command(_), HostStep::PollUntil(_), HostStep::Close(_)])));
    assert_eq!(shape_problems(&ek), Vec::<String>::new());
    let es = case(ES);
    assert!(es.serve[0].route.token_grant.is_some() && es.serve[1].route.operation.is_some());
    let deliver = es.deliver.as_ref().expect("deliver");
    assert_eq!(deliver[0].expect.handler, Handler::player_ref("p1"));
    assert_eq!(deliver[1].expect.handler, Handler::NOT_CALLED);
    assert_eq!(shape_problems(&es), Vec::<String>::new());

    // An unknown key anywhere is refused.
    assert!(refused(&EK.replace("requests: { count: 2 }", "requests: { count: 2, slow: true }")).contains("unknown field"));
    assert!(refused(&EK.replace("tier: ek", "tier: ek\nnotes: x")).contains("unknown field"));
    assert!(refused(&EK.replace("close: {}", "skip: {}")).contains("unknown variant"));

    // A secret name outside primary · secondary · unknown is refused.
    assert!(refused(&ES.replace("[unknown]", "[tertiary]")).contains("unknown variant"));

    // A route naming both forms, and an ES deliver: in an EK case, are refused.
    let both = case(&EK.replace("route: { operation: streamEvents }\n    respond:", "route: { operation: streamEvents, token_grant: {} }\n    respond:"));
    assert!(shape_problems(&both).iter().any(|p| p.contains("exactly one of operation and token_grant")), "{:?}", shape_problems(&both));
    let neither = case(&EK.replacen("route: { operation: streamEvents }", "route: {}", 1));
    assert!(shape_problems(&neither).iter().any(|p| p.contains("exactly one")));
    let mut mixed = case(ES);
    mixed.tier = Tier::Ek;
    assert!(shape_problems(&mixed).iter().any(|p| p.contains("an ek case has no deliver:")));
    let mut mixed = case(EK);
    mixed.tier = Tier::Es;
    assert!(shape_problems(&mixed).iter().any(|p| p.contains("an es case has no host:")));
}

const HOST: &str = r#"
id: ek9.full-queue-loses-nothing
behaviours: [EK9]
tier: ek
host:
  - start:
      initial_token: lgr_et_conformance0001
      config: { queue_capacity: 8, api_version: "2026-10-golden-remora", user_agent_suffix: "conformance/1", event_types: [lesson_plan.ready], resume_cursor: c0, stream_on_start: true, stream_idle_timeout_ms: 1000, log_level: warn }
  - provide_token: { token: "lgr_et_conformance0002", expires_at: "2026-10-01T12:15:00Z" }
  - provide_token: { token: "lgr_et_conformance0003", expires_in_secs: 900 }
  - command: { type: send_context, context: { scene: market, source_lang: en, target_lang: zh, level: 2 } }
  - poll_until: { event: stream, match: { state: { type: live } }, within_ms: 2000 }
  - expect_no_event: { event: error, for_ms: 200 }
  - stop_polling: { for_ms: 500 }
  - close: {}
"#;

// 1.10.26y AC21
#[test]
fn the_ek_host_script_is_closed() {
    use super::host::{HostStep as H, PollUntil, ProvideToken, Start};
    let ek = case(HOST);
    let steps = ek.host.as_deref().expect("host");
    assert_eq!(steps.len(), 8);
    let Some(H::Start(Start { initial_token, config })) = steps.first() else { panic!("start first") };
    assert_eq!((initial_token.as_str(), config.queue_capacity, config.stream_idle_timeout_ms), ("lgr_et_conformance0001", Some(8), Some(1000)));
    assert!(matches!(&steps[1], H::ProvideToken(ProvideToken { expires_at: Some(_), expires_in_secs: None, .. })));
    assert!(matches!(&steps[2], H::ProvideToken(ProvideToken { expires_at: None, expires_in_secs: Some(900), .. })));
    assert!(matches!(&steps[4], H::PollUntil(PollUntil { matches: Some(_), within_ms: 2000, .. })));
    assert!(matches!(&steps[5..], [H::ExpectNoEvent(_), H::StopPolling(_), H::Close(_)]));
    assert_eq!(shape_problems(&ek), Vec::<String>::new());

    // Served to a harness as JSON, each step is still a one-key object.
    let json = serde_json::to_value(&ek).expect("json");
    assert_eq!(json["host"][4]["poll_until"]["match"]["state"]["type"], "live");

    // An unknown step, or an unknown field on any step, is refused.
    for (from, to) in [
        ("  - close: {}", "  - skip: {}"),
        ("initial_token: lgr_et_conformance0001", "initial_token: lgr_et_conformance0001\n      token_source: http"),
        ("log_level: warn", "log_level: warn, base_url: x"),
        ("within_ms: 2000", "within_ms: 2000, retries: 2"),
        ("for_ms: 200", "for_ms: 200, quiet: true"),
        ("stop_polling: { for_ms: 500 }", "stop_polling: { for_ms: 500, then: drain }"),
        ("expires_in_secs: 900 }", "expires_in_secs: 900, scopes: [] }"),
    ] {
        let err = refused(&HOST.replace(from, to));
        assert!(err.contains("unknown"), "{to}: {err}");
    }

    // The rules serde cannot state: one lifetime per provide_token, a typed command.
    let both = case(&HOST.replace("expires_in_secs: 900 }", "expires_in_secs: 900, expires_at: \"2026-10-01T12:15:00Z\" }"));
    let neither = case(&HOST.replace(", expires_in_secs: 900 }", " }"));
    let untyped = case(&HOST.replace("type: send_context, ", ""));
    for (bad, want) in [(both, "exactly one of expires_at and expires_in_secs"), (neither, "exactly one"), (untyped, "a Command with a `type`")] {
        assert!(shape_problems(&bad).iter().any(|p| p.contains(want)), "{:?}", shape_problems(&bad));
    }
}

const KIT: &str = r#"
id: es5.no-token-in-renderings
behaviours: [ES5]
tier: es
directory: { seed: { lgr_sub_seeded: p0 } }
serve:
  - route: { operation: deleteEmbedPlayer }
    expect_request: { path: "/v1/embed/players/p1", body_equals: {} }
    respond: { status: 204 }
kit:
  - mint: { player_ref: p1, scopes: [events:read], origin: "https://game.example" }
  - delete: { player_ref: p1 }
  - handoff: {}
  - render: {}
deliver:
  - message: practice.completed
    as_type: world.weather_changed
    body: { subject: "lgr_sub_x" }
    sign: { secrets: [primary] }
    handler: raise
    router: { secrets: [secondary, primary] }
    expect: { status: 500, handler: { player_ref: null, event_id: "e1", unknown: false } }
expect:
  directory: { lgr_sub_seeded: p0 }
  directory_lookups: 1
  minted: { token: lgr_et_x, expires_at: "2026-10-01T09:27:44Z", expires_in: 900, subject: lgr_sub_x, scopes: [], account_linked: false }
  handoff: '{"token":"lgr_et_x","expires_at":"2026-10-01T09:27:44Z"}'
  error: { status: 403, code: embed_needs_metered }
"#;

// 1.10.26af AC2
/// Every ES key D11 adds deserialises, and an unknown `kit:` job or field,
/// or a `router` secret name outside R1's three, is refused.
#[test]
fn the_es_kit_script_is_closed() {
    use super::{Called, DeleteJob, HandlerMode, KitStep as K, MintJob, SecretName};
    let es = case(KIT);
    let steps = es.kit.as_deref().expect("kit");
    let Some(K::Mint(MintJob { player_ref, scopes: Some(scopes), origin: Some(_) })) = steps.first() else { panic!("mint first") };
    assert_eq!((player_ref.as_str(), scopes.as_slice()), ("p1", ["events:read".to_string()].as_slice()));
    assert!(matches!(&steps[1..], [K::Delete(DeleteJob { .. }), K::Handoff(_), K::Render(_)]));
    assert_eq!(es.directory.as_ref().map(|d| d.seed.len()), Some(1));
    let deliver = &es.deliver.as_deref().expect("deliver")[0];
    assert_eq!((deliver.handler, deliver.as_type.as_deref()), (HandlerMode::Raise, Some("world.weather_changed")));
    assert_eq!(deliver.router.as_ref().map(|r| r.secrets.clone()), Some(vec![SecretName::Secondary, SecretName::Primary]));
    let called = Called { player_ref: None, event_id: Some("e1".into()), unknown: Some(false) };
    assert_eq!(deliver.expect.handler, Handler::Called(called));
    let expect = &es.expect;
    assert_eq!((expect.directory_lookups, expect.minted.as_ref().map(|m| m.expires_in)), (Some(1), Some(900)));
    assert_eq!(expect.error.as_ref().map(|e| e.code.as_str()), Some("embed_needs_metered"));
    assert!(expect.handoff.as_deref().is_some_and(|h| h.starts_with("{\"token\"")));
    assert_eq!(es.serve[0].expect_request.as_ref().and_then(|e| e.path.as_deref()), Some("/v1/embed/players/p1"));
    assert_eq!(shape_problems(&es), Vec::<String>::new());

    // Served to a harness as JSON, each job is still a one-key object.
    let json = serde_json::to_value(&es).expect("json");
    assert_eq!(json["kit"][0]["mint"]["player_ref"], "p1");
    assert_eq!(json["deliver"][0]["expect"]["handler"]["player_ref"], serde_json::Value::Null);

    // An unknown job, an unknown field anywhere, or a fourth secret name is refused.
    for (from, to) in [
        ("  - render: {}", "  - refresh: {}"),
        ("mint: { player_ref: p1,", "mint: { player_ref: p1, cache: true,"),
        ("delete: { player_ref: p1 }", "delete: { player_ref: p1, soft: true }"),
        ("directory: { seed:", "directory: { wipe: true, seed:"),
        ("router: { secrets: [secondary, primary] }", "router: { secrets: [tertiary] }"),
        ("handler: raise", "handler: hang"),
        ("unknown: false }", "unknown: false, typed: true }"),
        ("account_linked: false }", "account_linked: false, scope: x }"),
        ("code: embed_needs_metered }", "code: embed_needs_metered, retry: 1 }"),
        ("directory_lookups: 1", "directory_lookups: 1\n  directory_writes: 1"),
    ] {
        let err = refused(&KIT.replace(from, to));
        assert!(err.contains("unknown") || err.contains("did not match"), "{to}: {err}");
    }

    // An EK case carries no kit: or directory:.
    let mut mixed = case(KIT);
    mixed.tier = Tier::Ek;
    assert!(shape_problems(&mixed).iter().any(|p| p.contains("an ek case has no kit: or directory:")));
}

// 30.10.26p D5: provide_token's update_available is passed through as written,
// a malformed one included (EK13's malformed case relies on it).
#[test]
fn provide_token_carries_update_available_unchecked() {
    use super::host::{HostStep as H, ProvideToken};
    for (notice, want) in [
        ("{ latest: \"1.2.0\", deprecated: true, message: null }", serde_json::json!({ "latest": "1.2.0", "deprecated": true, "message": null })),
        ("{ latest: \"1.2.0\", message: null }", serde_json::json!({ "latest": "1.2.0", "message": null })),
        ("5", serde_json::json!(5)),
    ] {
        let from = "expires_at: \"2026-10-01T12:15:00Z\" }";
        let ek = case(&HOST.replace(from, &format!("expires_at: \"2026-10-01T12:15:00Z\", update_available: {notice} }}")));
        let H::ProvideToken(ProvideToken { update_available, .. }) = &ek.host.as_deref().unwrap()[1] else { panic!("provide_token") };
        assert_eq!(update_available.as_ref(), Some(&want));
        assert_eq!(shape_problems(&ek), Vec::<String>::new());
        assert_eq!(serde_json::to_value(&ek).unwrap()["host"][1]["provide_token"]["update_available"], want, "served as written");
    }
    let H::ProvideToken(plain) = &case(HOST).host.unwrap()[1] else { panic!("provide_token") };
    assert!(plain.update_available.is_none(), "absent stays absent");
}
