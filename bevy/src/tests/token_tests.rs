//! Tokens (D5): the Poll source's Message round trip, and redaction.

use lingara_embed::{Command, EmbedEvent, Header, Secret, TokenReason, UpdateAvailable};

use super::fake_core::{Fake, app, read, write};
use crate::config::LingaraConfig;
use crate::messages::*;

const TOKEN: &str = "lgr_et_abcdefgh12345678";

/// 1.10.26z AC4: `TokenNeeded` → `LingaraTokenNeeded`; `ProvideToken`,
/// `TokenUnavailable` and `SetTokenEndpoint` reach their K1 calls; the
/// token shows nowhere a `Debug` or a resource could print it.
#[test]
fn the_message_round_trip_provides_the_token() {
    let fake = Fake::default();
    fake.push(EmbedEvent::TokenNeeded { request: 4, reason: TokenReason::Expiring });
    let mut app = app(LingaraConfig::new(), &fake);
    app.update();
    assert_eq!(read::<LingaraTokenNeeded>(&mut app), [LingaraTokenNeeded { request: 4, reason: TokenReason::Expiring }]);

    let provide = ProvideToken::new(TOKEN, "2026-10-01T12:15:00Z");
    let shown = format!("{provide:?}");
    assert!(shown.contains("[REDACTED]") && !shown.contains(TOKEN), "{shown}");
    write(&mut app, provide);
    write(&mut app, TokenUnavailable { reason: "signed out".into() });
    let header = Header { name: "x-session".into(), value: Secret::new("session-secret") };
    let endpoint = SetTokenEndpoint { url: "https://game.example/token".into(), headers: vec![header.clone()] };
    let shown = format!("{endpoint:?}");
    assert!(shown.contains("[REDACTED]") && !shown.contains("session-secret"), "{shown}");
    write(&mut app, endpoint);
    app.update();

    let provided = fake.sent("provide_token");
    let Some(Command::ProvideToken { token, expires_at, update_available: None }) = provided.first() else { panic!("no provide_token: {provided:?}") };
    assert_eq!((token.expose_secret(), expires_at.as_str()), (TOKEN, "2026-10-01T12:15:00Z"));
    assert_eq!(fake.sent("token_unavailable"), [Command::TokenUnavailable { reason: "signed out".into() }]);
    assert_eq!(fake.sent("set_token_endpoint"), [Command::SetTokenEndpoint { url: "https://game.example/token".into(), headers: vec![header] }]);
    let held = format!("{:?} {:?}", app.world().resource::<LingaraStatus>(), fake.state().commands);
    assert!(!held.contains(TOKEN) && !held.contains("session-secret"), "{held}");
}

/// 30.10.26p D4: `ProvideToken::with_update` reaches the command, and the
/// core's `UpdateAvailable` becomes `LingaraUpdateAvailable`.
#[test]
fn the_update_notice_rides_the_token_and_comes_back_as_a_message() {
    let fake = Fake::default();
    let notice = UpdateAvailable { latest: Some("1.2.0".into()), deprecated: true, message: None };
    fake.push(EmbedEvent::UpdateAvailable { latest: Some("1.2.0".into()), deprecated: true, message: None });
    let mut app = app(LingaraConfig::new().embed_version("1.1.0"), &fake);
    app.update();
    assert_eq!(read::<LingaraUpdateAvailable>(&mut app), [LingaraUpdateAvailable { latest: Some("1.2.0".into()), deprecated: true, message: None }]);
    write(&mut app, ProvideToken::with_update(TOKEN, "2026-10-01T12:15:00Z", Some(notice.clone())));
    app.update();
    let provided = fake.sent("provide_token");
    assert!(matches!(provided.first(), Some(Command::ProvideToken { update_available: Some(n), .. }) if *n == notice), "{provided:?}");
}
