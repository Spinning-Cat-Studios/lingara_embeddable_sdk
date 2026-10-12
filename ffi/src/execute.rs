//! `execute` (D3): bytes to a K1 `Command` in two steps, then K1's own
//! `execute`. The FFI keeps no tag list of its own: serde's *unknown
//! variant* is `ERR_UNKNOWN_COMMAND`, any other decode failure is
//! `ERR_INVALID_ARGUMENT`, and neither message quotes the input, because
//! serde's own text can echo a field's value (a token, D4).

use core_embed::{Command, CommandOutcome, EmbedError};
use serde_json::Value;

use crate::handle::LingaraEmbed;
use crate::status::{
    Fault, LINGARA_EMBED_ERR_INVALID_ARGUMENT, LINGARA_EMBED_ERR_INVALID_JSON, LINGARA_EMBED_ERR_INVALID_UTF8, LINGARA_EMBED_ERR_UNKNOWN_COMMAND,
    LINGARA_EMBED_OK, LINGARA_EMBED_REFUSED,
};

/// What the core said: `OK` and a `CommandOutcome`, or `REFUSED` and its `EmbedError`.
pub(crate) struct Answer {
    pub status: i32,
    pub json: Vec<u8>,
}

impl Answer {
    pub fn outcome(outcome: &CommandOutcome) -> Self {
        Self { status: LINGARA_EMBED_OK, json: to_json(outcome) }
    }

    pub fn refused(error: &EmbedError) -> Self {
        Self { status: LINGARA_EMBED_REFUSED, json: to_json(error) }
    }
}

/// K1's types serialise infallibly: plain owned data, string keys.
pub(crate) fn to_json<T: serde::Serialize>(value: &T) -> Vec<u8> {
    serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec())
}

pub(crate) fn utf8<'a>(bytes: &'a [u8], what: &str) -> Result<&'a str, Fault> {
    std::str::from_utf8(bytes)
        .map_err(|e| Fault::new(LINGARA_EMBED_ERR_INVALID_UTF8, format!("ERR_INVALID_UTF8: {what} is not UTF-8 (at byte {})", e.valid_up_to())))
}

pub(crate) fn not_json(what: &str, e: &serde_json::Error) -> Fault {
    Fault::new(LINGARA_EMBED_ERR_INVALID_JSON, format!("ERR_INVALID_JSON: {what} is not JSON (at line {}, column {})", e.line(), e.column()))
}

/// Step one: a JSON object with a string `type`, which is returned.
fn command_type(text: &str) -> Result<String, Fault> {
    let value: Value = serde_json::from_str(text).map_err(|e| not_json("the command", &e))?;
    match value.as_object().and_then(|o| o.get("type")).and_then(Value::as_str) {
        Some(tag) => Ok(tag.to_owned()),
        None if value.is_object() => Err(Fault::new(LINGARA_EMBED_ERR_INVALID_JSON, "ERR_INVALID_JSON: the command has no string \"type\"")),
        None => Err(Fault::new(LINGARA_EMBED_ERR_INVALID_JSON, "ERR_INVALID_JSON: the command is not a JSON object")),
    }
}

/// A `type` is named in a message only when it is a plain identifier: a
/// caller who put a token there gets `<unprintable>`, not an echo.
fn printable(tag: &str) -> &str {
    let plain = !tag.is_empty() && tag.len() <= 64 && tag.bytes().all(|b| b.is_ascii_lowercase() || b == b'_');
    if plain { tag } else { "<unprintable>" }
}

/// Step two: the `Command`, or which of the two decode statuses it is.
pub(crate) fn decode(bytes: &[u8]) -> Result<Command, Fault> {
    let text = utf8(bytes, "the command")?;
    let tag = command_type(text)?;
    serde_json::from_str::<Command>(text).map_err(|e| {
        let shown = printable(&tag);
        if e.to_string().starts_with("unknown variant") {
            Fault::new(LINGARA_EMBED_ERR_UNKNOWN_COMMAND, format!("ERR_UNKNOWN_COMMAND: this core has no command of type `{shown}`"))
        } else {
            Fault::new(
                LINGARA_EMBED_ERR_INVALID_ARGUMENT,
                format!("ERR_INVALID_ARGUMENT: the fields of command `{shown}` do not deserialise (at line {}, column {})", e.line(), e.column()),
            )
        }
    })
}

/// Decodes and runs one command through K1. A command is run once: its
/// answer is returned for the caller's buffer and the thread's last answer.
pub(crate) fn run(handle: &LingaraEmbed, bytes: &[u8]) -> Result<Answer, Fault> {
    #[cfg(test)]
    if bytes == crate::tests::PANIC_COMMAND {
        panic!("the test intercept panics inside execute");
    }
    let command = decode(bytes)?;
    Ok(match handle.embed.execute(command) {
        Ok(outcome) => Answer::outcome(&outcome),
        Err(error) => Answer::refused(&error),
    })
}
