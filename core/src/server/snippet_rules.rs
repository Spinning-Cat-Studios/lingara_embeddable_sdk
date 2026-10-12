//! ES4's option rules (ADR 1.10.26af D7), written out by hand: the crate
//! takes no regex dependency, and every pattern is ASCII-only. Each function
//! is one row of the contract's table.

/// Unicode category Cc.
pub(crate) fn is_control(c: char) -> bool {
    matches!(c, '\u{0}'..='\u{1f}' | '\u{7f}'..='\u{9f}')
}

/// The contract's whitespace set (Unicode `White_Space` beyond Cc).
fn is_space(c: char) -> bool {
    matches!(c, ' ' | '\u{85}' | '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}')
}

fn all(s: &str, ok: impl Fn(u8) -> bool) -> bool {
    s.bytes().all(ok)
}

/// `^[0-9A-Fa-f]{8}(-[0-9A-Fa-f]{4}){3}-[0-9A-Fa-f]{12}$`
pub(crate) fn plan_id(s: &str) -> bool {
    let groups: Vec<&str> = s.split('-').collect();
    groups.len() == 5 && groups.iter().zip([8, 4, 4, 4, 12]).all(|(g, n)| g.len() == n && all(g, |b| b.is_ascii_hexdigit()))
}

/// `^[a-z][a-z0-9-]{0,31}$`
pub(crate) fn surface(s: &str) -> bool {
    (1..=32).contains(&s.len()) && s.as_bytes()[0].is_ascii_lowercase() && all(s, |b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// `^[A-Za-z]{2,3}(-[A-Za-z0-9]{2,8}){0,3}$`
pub(crate) fn locale(s: &str) -> bool {
    let mut parts = s.split('-');
    let lang = parts.next().unwrap_or_default();
    let rest: Vec<&str> = parts.collect();
    (2..=3).contains(&lang.len())
        && all(lang, |b| b.is_ascii_alphabetic())
        && rest.len() <= 3
        && rest.iter().all(|p| (2..=8).contains(&p.len()) && all(p, |b| b.is_ascii_alphanumeric()))
}

/// `https://` plus a host, or `http://localhost` / `http://127.0.0.1`, each
/// with an optional `:port` and an optional trailing `/`.
pub(crate) fn lingara_origin(s: &str) -> bool {
    let s = s.strip_suffix('/').unwrap_or(s);
    let (host, port) = match s.rsplit_once(':') {
        Some((h, p)) if !h.ends_with("//") && !h.ends_with(':') && h.contains("//") => (h, Some(p)),
        _ => (s, None),
    };
    let port_ok = port.is_none_or(|p| (1..=5).contains(&p.len()) && all(p, |b| b.is_ascii_digit()));
    let https = host.strip_prefix("https://").is_some_and(|h| h.split('.').all(|l| !l.is_empty() && all(l, |b| b.is_ascii_alphanumeric() || b == b'-')));
    let local = matches!(host, "http://localhost" | "http://127.0.0.1");
    port_ok && (https || local)
}

/// 1–80 scalar values, none of them a control character.
pub(crate) fn title(s: &str) -> bool {
    (1..=80).contains(&s.chars().count()) && !s.chars().any(is_control)
}

/// `^[A-Za-z][A-Za-z0-9_-]{0,63}$`
pub(crate) fn id(s: &str) -> bool {
    (1..=64).contains(&s.len()) && s.as_bytes()[0].is_ascii_alphabetic() && all(s, |b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// At most 2048 bytes, with no control character or whitespace.
fn url_text(s: &str) -> bool {
    s.len() <= 2048 && !s.chars().any(|c| is_control(c) || is_space(c))
}

/// A path that starts `/` but not `//`, or an absolute `https://` URL.
pub(crate) fn token_endpoint(s: &str) -> bool {
    let shape = (s.starts_with('/') && !s.starts_with("//")) || s.starts_with("https://");
    shape && url_text(s)
}

/// `^[A-Za-z0-9+/=_-]{8,128}$`
pub(crate) fn nonce(s: &str) -> bool {
    (8..=128).contains(&s.len()) && all(s, |b| b.is_ascii_alphanumeric() || b"+/=_-".contains(&b))
}

/// An `https://` URL.
pub(crate) fn element_src(s: &str) -> bool {
    s.starts_with("https://") && url_text(s)
}
