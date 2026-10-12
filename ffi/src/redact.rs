//! The second guard on the FFI's own messages (D4): any `lgr_<kind>_` run
//! that still reaches one is cut to its prefix plus `[REDACTED]`. The first
//! guard is that no message quotes input at all.

use core_embed::REDACTED;

/// `lgr_et_abc123 x` → `lgr_et_[REDACTED] x`.
pub(crate) fn redact(message: &str) -> String {
    let mut out = String::with_capacity(message.len());
    let mut rest = message;
    while let Some(at) = rest.find("lgr_") {
        out.push_str(&rest[..at]);
        let tail = &rest[at + 4..];
        let kind = tail.bytes().take_while(u8::is_ascii_lowercase).count();
        if kind > 0 && tail.as_bytes().get(kind) == Some(&b'_') {
            out.push_str(&rest[at..at + 4 + kind + 1]);
            out.push_str(REDACTED);
            let body = &tail[kind + 1..];
            let secret = body.bytes().take_while(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.')).count();
            rest = &body[secret..];
        } else {
            out.push_str("lgr_");
            rest = tail;
        }
    }
    out.push_str(rest);
    out
}
