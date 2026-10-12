//! The status codes (D3): `int32_t`, never a C enum, whose size is the
//! compiler's choice. Zero and positive: the call reached the core, so read
//! the buffer. Negative: it did not, so read `lingara_embed_last_error`.
//! Changing any code's meaning is an ABI major bump (D5).

/// `new`: a handle. `execute`: a `CommandOutcome`. `poll`: an event. `status`: `EmbedStatus`.
pub const LINGARA_EMBED_OK: i32 = 0;
/// `poll`: the queue is empty. `last_answer` / `last_error`: this thread has none.
pub const LINGARA_EMBED_EMPTY: i32 = 1;
/// `*out_len` is the bytes needed. `poll` / `status` / `last_*`: nothing consumed.
/// `execute`: the command HAS run, and its answer waits in `last_answer`.
pub const LINGARA_EMBED_BUFFER_TOO_SMALL: i32 = 2;
/// `new` / `execute`: the core refused, and the answer is an `EmbedError`.
pub const LINGARA_EMBED_REFUSED: i32 = 3;
pub const LINGARA_EMBED_ERR_NULL: i32 = -1;
pub const LINGARA_EMBED_ERR_INVALID_UTF8: i32 = -2;
/// Not a JSON object (`execute`: or no string `type`).
pub const LINGARA_EMBED_ERR_INVALID_JSON: i32 = -3;
/// `execute` only: a `type` this core has no `Command` for.
pub const LINGARA_EMBED_ERR_UNKNOWN_COMMAND: i32 = -4;
/// Fields that do not deserialise: a known command's, or the config's.
pub const LINGARA_EMBED_ERR_INVALID_ARGUMENT: i32 = -5;
/// The handle was poisoned by a panic in an FFI frame.
pub const LINGARA_EMBED_ERR_CLOSED: i32 = -6;
pub const LINGARA_EMBED_ERR_PANIC: i32 = -7;
pub const LINGARA_EMBED_ERR_ABI_MISMATCH: i32 = -8;

/// A negative status: its code and the English text `last_error` copies out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Fault {
    pub status: i32,
    pub message: String,
}

impl Fault {
    pub fn new(status: i32, message: impl Into<String>) -> Self {
        Self { status, message: message.into() }
    }

    pub fn null(what: &str) -> Self {
        Self::new(LINGARA_EMBED_ERR_NULL, format!("ERR_NULL: {what} is NULL"))
    }

    pub fn closed() -> Self {
        Self::new(LINGARA_EMBED_ERR_CLOSED, "ERR_CLOSED: the handle was poisoned by an earlier panic; free it")
    }
}
