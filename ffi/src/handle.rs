//! The opaque handle (D4): K1's `Embed`, the one-slot poll stash, and the
//! poison flag a caught panic sets. `new` validates the caller's ABI and
//! config before anything is allocated; `free` joins the I/O thread, bounded.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use core_embed::{Embed, EmbedConfig};

use crate::status::{Fault, LINGARA_EMBED_ERR_ABI_MISMATCH, LINGARA_EMBED_ERR_INVALID_ARGUMENT, LINGARA_EMBED_ERR_INVALID_JSON};
use crate::{LINGARA_EMBED_ABI_VERSION_MAJOR, LINGARA_EMBED_ABI_VERSION_MINOR};

/// How long `free` waits for the I/O thread before detaching it (D4).
pub(crate) const FREE_JOIN_MS: u32 = 1_000;

/// One player's connection. Opaque to C: `typedef struct LingaraEmbed LingaraEmbed;`.
pub struct LingaraEmbed {
    pub(crate) embed: Embed,
    /// A polled event that did not fit the caller's buffer, served first next time.
    pub(crate) stash: Mutex<Option<Vec<u8>>>,
    poisoned: AtomicBool,
}

impl LingaraEmbed {
    pub(crate) fn new(embed: Embed) -> Self {
        Self { embed, stash: Mutex::new(None), poisoned: AtomicBool::new(false) }
    }

    pub(crate) fn poison(&self) {
        self.poisoned.store(true, Ordering::SeqCst);
    }

    /// `ERR_CLOSED` once a panic has poisoned the handle.
    pub(crate) fn live(&self) -> Result<&Self, Fault> {
        if self.poisoned.load(Ordering::SeqCst) { Err(Fault::closed()) } else { Ok(self) }
    }

    /// `close_and_wait(1000)`; past the bound the thread is detached and
    /// `free` returns anyway, since its `false` has nowhere to go.
    pub(crate) fn close(&self) {
        let _joined = self.embed.close_and_wait(FREE_JOIN_MS);
    }
}

/// `ERR_ABI_MISMATCH` naming both majors, before anything is allocated.
pub(crate) fn check_abi(requested: u32) -> Result<(), Fault> {
    if requested == LINGARA_EMBED_ABI_VERSION_MAJOR {
        return Ok(());
    }
    Err(Fault::new(
        LINGARA_EMBED_ERR_ABI_MISMATCH,
        format!(
            "ERR_ABI_MISMATCH: the caller was built against ABI major {requested}; this library is ABI {LINGARA_EMBED_ABI_VERSION_MAJOR}.{LINGARA_EMBED_ABI_VERSION_MINOR}"
        ),
    ))
}

/// K1's `EmbedConfig` from its JSON shape; empty input is `{}`. The message
/// names serde's category and position, never the input: an Http source's
/// header values are secrets.
pub(crate) fn parse_config(bytes: &[u8]) -> Result<EmbedConfig, Fault> {
    if bytes.is_empty() {
        return Ok(EmbedConfig::default());
    }
    let text = crate::execute::utf8(bytes, "config_json")?;
    let value: serde_json::Value = serde_json::from_str(text).map_err(|e| crate::execute::not_json("config_json", &e))?;
    if !value.is_object() {
        return Err(Fault::new(LINGARA_EMBED_ERR_INVALID_JSON, "ERR_INVALID_JSON: config_json is not a JSON object"));
    }
    serde_json::from_str(text).map_err(|e| {
        Fault::new(
            LINGARA_EMBED_ERR_INVALID_ARGUMENT,
            format!("ERR_INVALID_ARGUMENT: config_json does not deserialise as EmbedConfig (at line {}, column {})", e.line(), e.column()),
        )
    })
}
