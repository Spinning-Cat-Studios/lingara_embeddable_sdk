//! Tokens (D5). Two of K1's three sources need nothing from the frame:
//! `token_endpoint` runs on the core's thread, and so does a Rust
//! `token_source`. The third, the Poll source, is a Message round trip:
//! Apply writes `LingaraTokenNeeded`, and this system hands the game's
//! `ProvideToken`, `TokenUnavailable` or `SetTokenEndpoint` to the core.
//! The token is read out of its Message here, once, and goes straight into
//! a K1 command whose `Debug` and `Serialize` redact it. Nothing logs.

use bevy::ecs::message::MessageReader;
use lingara_embed::Command;

use crate::frame::Link;
use crate::messages::{ProvideToken, SetTokenEndpoint, TokenUnavailable};

pub(crate) fn send(
    mut provide: MessageReader<ProvideToken>,
    mut unavailable: MessageReader<TokenUnavailable>,
    mut endpoint: MessageReader<SetTokenEndpoint>,
    mut link: Link,
) {
    for message in provide.read() {
        let (token, expires_at) = message.parts();
        link.run(Command::ProvideToken { token, expires_at }, None);
    }
    for TokenUnavailable { reason } in unavailable.read() {
        link.run(Command::TokenUnavailable { reason: reason.clone() }, None);
    }
    for SetTokenEndpoint { url, headers } in endpoint.read() {
        link.run(Command::SetTokenEndpoint { url: url.clone(), headers: headers.clone() }, None);
    }
}
