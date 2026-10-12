//! `poll` (D2, D4): one K1 `EmbedEvent` per call, in queue order. K1's
//! `poll_event` pops, so an event that does not fit the caller's buffer is
//! kept serialised in the handle's one-slot stash and served first on the
//! next call: a retry with a bigger buffer gets the same event.

use std::sync::PoisonError;

use crate::execute::to_json;
use crate::handle::LingaraEmbed;
use crate::status::LINGARA_EMBED_OK;
use crate::thread_slots::Out;

/// # Safety
/// `out` passed `check`.
pub(crate) unsafe fn poll(handle: &LingaraEmbed, out: &Out) -> i32 {
    let mut stash = handle.stash.lock().unwrap_or_else(PoisonError::into_inner);
    let event = match stash.take() {
        Some(kept) => kept,
        None => match handle.embed.poll_event() {
            Some(event) => to_json(&event),
            None => return unsafe { out.empty() },
        },
    };
    let status = unsafe { out.write(&event, LINGARA_EMBED_OK) };
    if status != LINGARA_EMBED_OK {
        *stash = Some(event);
    }
    status
}
