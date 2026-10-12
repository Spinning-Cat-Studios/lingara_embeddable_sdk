//! The token sources the core fetches from itself (D4): the integrator's
//! Http endpoint, and a Rust game's own `EmbedTokenSource`.

use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use crate::config::{Header, check_url};
use crate::error::{EmbedError, ErrorKind};
use crate::handoff::{EmbedTokenSource, IssuedToken};
use crate::types::TokenReason;

#[derive(Clone)]
pub(crate) enum Mode {
    Poll,
    Http { url: String, headers: HeaderMap, client: reqwest::Client },
    Trait(Arc<dyn EmbedTokenSource>),
}

/// The Http source for `url`, its header values marked sensitive so not
/// even reqwest's own `Debug` renders them.
pub(crate) fn http_mode(url: &str, headers: &[Header]) -> Result<Mode, EmbedError> {
    check_url("token endpoint", url)?;
    let mut map = HeaderMap::new();
    for header in headers {
        let name = HeaderName::from_bytes(header.name.as_bytes())
            .map_err(|_| EmbedError::new(ErrorKind::Config, format!("`{}` is not a header name", header.name)))?;
        let mut value = HeaderValue::from_str(header.value.expose_secret())
            .map_err(|_| EmbedError::new(ErrorKind::Config, format!("the value of header `{}` is not a header value", header.name)))?;
        value.set_sensitive(true);
        map.append(name, value);
    }
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| EmbedError::new(ErrorKind::Config, format!("cannot build the token endpoint's HTTP client: {e}")))?;
    Ok(Mode::Http { url: url.to_owned(), headers: map, client })
}

/// One token from a fetching source. The poll source never fetches.
pub(crate) async fn fetch(mode: &Mode, reason: TokenReason) -> Result<IssuedToken, EmbedError> {
    match mode {
        Mode::Poll => Err(EmbedError::new(ErrorKind::Internal, "the poll source does not fetch")),
        Mode::Http { url, headers, client } => http(client, url, headers).await,
        Mode::Trait(source) => source.fetch(reason).await,
    }
}

/// `POST {}` with the host's headers; the answer is V1's handoff body.
async fn http(client: &reqwest::Client, url: &str, headers: &HeaderMap) -> Result<IssuedToken, EmbedError> {
    let unavailable = |message: String| EmbedError::new(ErrorKind::TokenUnavailable, message);
    let res = client
        .post(url)
        .headers(headers.clone())
        .header("content-type", "application/json")
        .header("accept", "application/json")
        .body("{}")
        .send()
        .await
        .map_err(|e| unavailable(format!("the token endpoint did not answer: {}", e.without_url())))?;
    let status = res.status();
    if !status.is_success() {
        let mut err = unavailable(format!("the token endpoint answered HTTP {}", status.as_u16()));
        err.status = Some(status.as_u16());
        return Err(err);
    }
    let body = res.bytes().await.map_err(|e| unavailable(format!("the token endpoint's answer was cut off: {}", e.without_url())))?;
    IssuedToken::from_handoff_json(&body)
}
