//! `lingara-embed-ffi` (ADR 1.10.26aa): the C ABI over `lingara-embed`.
//!
//! Nine functions move K1's own JSON through caller-owned buffers: a command
//! in and its answer back in the same call, an event out per poll. No memory
//! crosses the boundary, no callback is ever made, and no panic unwinds into
//! a foreign frame. `include/lingara_embed.h` is cbindgen's view of this file
//! and is committed; `header_tests.rs` holds the two equal.

mod events;
mod execute;
mod handle;
mod redact;
mod status;
mod thread_slots;

#[cfg(test)]
mod tests;

use std::ffi::c_char;
use std::panic::{AssertUnwindSafe, catch_unwind};

use core_embed::Embed;

pub use handle::LingaraEmbed;
pub use status::*;

use status::Fault;
use thread_slots::{Out, clear_answer, fail, set_answer};

/// Adding a function is a minor bump; changing or removing one, or a status
/// code's meaning, is a major bump (D5).
pub const LINGARA_EMBED_ABI_VERSION_MAJOR: u32 = 1;
pub const LINGARA_EMBED_ABI_VERSION_MINOR: u32 = 0;

const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "\0");

/// Runs one entry point: a `Fault` becomes its status and this thread's last
/// error, and a panic becomes `ERR_PANIC` and poisons `handle` (D4).
fn guard(handle: Option<&LingaraEmbed>, body: impl FnOnce() -> Result<i32, Fault>) -> i32 {
    match catch_unwind(AssertUnwindSafe(body)) {
        Ok(Ok(status)) => status,
        Ok(Err(fault)) => fail(fault),
        Err(payload) => {
            if let Some(handle) = handle {
                handle.poison();
            }
            let message = payload.downcast_ref::<&str>().map(|s| (*s).to_owned()).or_else(|| payload.downcast_ref::<String>().cloned());
            fail(Fault::new(LINGARA_EMBED_ERR_PANIC, format!("ERR_PANIC: {}", message.unwrap_or_else(|| "a panic with no message".into()))))
        }
    }
}

/// # Safety
/// `ptr` is NULL or valid for `len` bytes.
unsafe fn bytes<'a>(ptr: *const u8, len: usize, what: &str) -> Result<&'a [u8], Fault> {
    match (ptr.is_null(), len) {
        (true, 0) => Ok(&[]),
        (true, _) => Err(Fault::null(what)),
        (false, _) => Ok(unsafe { std::slice::from_raw_parts(ptr, len) }),
    }
}

/// # Safety
/// `handle` is NULL or came from `lingara_embed_new` and is not yet freed.
unsafe fn handle_ref<'a>(handle: *const LingaraEmbed) -> Option<&'a LingaraEmbed> {
    unsafe { handle.as_ref() }
}

/// `(MAJOR << 16) | MINOR`.
#[unsafe(no_mangle)]
pub extern "C" fn lingara_embed_abi_version() -> u32 {
    (LINGARA_EMBED_ABI_VERSION_MAJOR << 16) | LINGARA_EMBED_ABI_VERSION_MINOR
}

/// The package's semver: static, NUL-terminated, never freed.
#[unsafe(no_mangle)]
pub extern "C" fn lingara_embed_version() -> *const c_char {
    VERSION.as_ptr().cast()
}

/// Starts one player's connection. `abi_major` is the major the caller was
/// built against. `config_json` is K1's `EmbedConfig` (NULL with length 0 is
/// `{}`). `REFUSED`: K1's `Config` error is this thread's last answer.
///
/// # Safety
/// `config_json` is NULL or valid for `config_len` bytes; `out_handle` is
/// NULL or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lingara_embed_new(abi_major: u32, config_json: *const u8, config_len: usize, out_handle: *mut *mut LingaraEmbed) -> i32 {
    guard(None, || {
        let out_handle = unsafe { out_handle.as_mut() }.ok_or_else(|| Fault::null("out_handle"))?;
        *out_handle = std::ptr::null_mut();
        handle::check_abi(abi_major)?;
        let config = handle::parse_config(unsafe { bytes(config_json, config_len, "config_json") }?)?;
        clear_answer();
        match Embed::start(config) {
            Ok(embed) => {
                *out_handle = Box::into_raw(Box::new(LingaraEmbed::new(embed)));
                Ok(LINGARA_EMBED_OK)
            }
            Err(error) => {
                let answer = execute::Answer::refused(&error);
                set_answer(answer.status, answer.json);
                Ok(LINGARA_EMBED_REFUSED)
            }
        }
    })
}

/// `close_and_wait(1000)`, then drop. NULL is a no-op. Must not race any
/// other call on the same handle, and is the last call made on it.
///
/// # Safety
/// `handle` is NULL or came from `lingara_embed_new` and is not yet freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lingara_embed_free(handle: *mut LingaraEmbed) {
    if handle.is_null() {
        return;
    }
    let owned = unsafe { Box::from_raw(handle) };
    let _status = guard(None, move || {
        owned.close();
        drop(owned);
        Ok(LINGARA_EMBED_OK)
    });
}

/// Runs one K1 `Command` and writes its answer: `OK` and a `CommandOutcome`,
/// or `REFUSED` and an `EmbedError`. On `BUFFER_TOO_SMALL` the command has
/// run, and `lingara_embed_last_answer` on this thread copies its answer.
///
/// # Safety
/// `handle` came from `lingara_embed_new`; `command_json` is valid for
/// `command_len` bytes; `buf` is NULL or valid for `cap` bytes; `out_len` is writable.
// code-health: allow-arity — the C ABI's shape (ADR 1.10.26aa D3): a command in and its answer buffer out, in one call
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lingara_embed_execute(
    handle: *mut LingaraEmbed,
    command_json: *const u8,
    command_len: usize,
    buf: *mut u8,
    cap: usize,
    out_len: *mut usize,
) -> i32 {
    let handle = unsafe { handle_ref(handle) };
    guard(handle, || {
        let out = Out { buf, cap, out_len };
        out.check()?;
        let handle = handle.ok_or_else(|| Fault::null("handle"))?.live()?;
        if command_json.is_null() {
            return Err(Fault::null("command_json"));
        }
        clear_answer();
        let answer = execute::run(handle, unsafe { bytes(command_json, command_len, "command_json") }?)?;
        let status = unsafe { out.write(&answer.json, answer.status) };
        set_answer(answer.status, answer.json);
        Ok(status)
    })
}

/// One K1 `EmbedEvent`, or `EMPTY`. On `BUFFER_TOO_SMALL` the event is kept,
/// and the next call returns it first.
///
/// # Safety
/// `handle` came from `lingara_embed_new`; `buf` is NULL or valid for `cap`
/// bytes; `out_len` is writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lingara_embed_poll(handle: *mut LingaraEmbed, buf: *mut u8, cap: usize, out_len: *mut usize) -> i32 {
    let handle = unsafe { handle_ref(handle) };
    guard(handle, || {
        let out = Out { buf, cap, out_len };
        out.check()?;
        let handle = handle.ok_or_else(|| Fault::null("handle"))?.live()?;
        Ok(unsafe { events::poll(handle, &out) })
    })
}

/// K1's `EmbedStatus`: `{stream, cursor, needs_token, closed}`.
///
/// # Safety
/// `handle` came from `lingara_embed_new`; `buf` is NULL or valid for `cap`
/// bytes; `out_len` is writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lingara_embed_status(handle: *const LingaraEmbed, buf: *mut u8, cap: usize, out_len: *mut usize) -> i32 {
    let handle = unsafe { handle_ref(handle) };
    guard(handle, || {
        let out = Out { buf, cap, out_len };
        out.check()?;
        let handle = handle.ok_or_else(|| Fault::null("handle"))?.live()?;
        Ok(unsafe { out.write(&execute::to_json(&handle.embed.status()), LINGARA_EMBED_OK) })
    })
}

/// This thread's last `new` or `execute` answer, with the status that call
/// would have returned (`OK` or `REFUSED`). `EMPTY` when there is none.
///
/// # Safety
/// `buf` is NULL or valid for `cap` bytes; `out_len` is writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lingara_embed_last_answer(buf: *mut u8, cap: usize, out_len: *mut usize) -> i32 {
    guard(None, || {
        let out = Out { buf, cap, out_len };
        out.check()?;
        Ok(unsafe { thread_slots::copy_answer(&out) })
    })
}

/// This thread's last negative status, as English text. `EMPTY` when there is none.
///
/// # Safety
/// `buf` is NULL or valid for `cap` bytes; `out_len` is writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lingara_embed_last_error(buf: *mut u8, cap: usize, out_len: *mut usize) -> i32 {
    guard(None, || {
        let out = Out { buf, cap, out_len };
        out.check()?;
        Ok(unsafe { thread_slots::copy_error(&out) })
    })
}
