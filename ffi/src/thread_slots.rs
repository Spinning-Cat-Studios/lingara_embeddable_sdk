//! This thread's last answer and last error (D3, D4), and the one copy into
//! a caller's buffer every function shares. Nothing here is allocated for
//! the caller: the slots are copied out, never handed over.

use std::cell::RefCell;

use crate::redact::redact;
use crate::status::{Fault, LINGARA_EMBED_BUFFER_TOO_SMALL, LINGARA_EMBED_EMPTY, LINGARA_EMBED_OK};

thread_local! {
    /// The status `new` or `execute` returned (`OK` or `REFUSED`) and its JSON.
    static LAST_ANSWER: RefCell<Option<(i32, Vec<u8>)>> = const { RefCell::new(None) };
    static LAST_ERROR: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// A caller's output buffer. `buf` may be NULL only with `cap` 0, the size probe.
pub(crate) struct Out {
    pub buf: *mut u8,
    pub cap: usize,
    pub out_len: *mut usize,
}

impl Out {
    /// Refuses a NULL `out_len`, and a NULL `buf` with a non-zero `cap`.
    pub fn check(&self) -> Result<(), Fault> {
        if self.out_len.is_null() {
            return Err(Fault::null("out_len"));
        }
        if self.buf.is_null() && self.cap != 0 {
            return Err(Fault::null("buf (with a non-zero cap)"));
        }
        Ok(())
    }

    /// Sets `*out_len` and copies `bytes` when they fit: `fits` when they
    /// did, `BUFFER_TOO_SMALL` when they did not.
    ///
    /// # Safety
    /// `check` passed, and `buf` is valid for `cap` bytes.
    pub unsafe fn write(&self, bytes: &[u8], fits: i32) -> i32 {
        unsafe { *self.out_len = bytes.len() };
        if bytes.len() > self.cap {
            return LINGARA_EMBED_BUFFER_TOO_SMALL;
        }
        if !bytes.is_empty() {
            unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), self.buf, bytes.len()) };
        }
        fits
    }

    /// `*out_len = 0`: nothing to read.
    ///
    /// # Safety
    /// `check` passed.
    pub unsafe fn empty(&self) -> i32 {
        unsafe { *self.out_len = 0 };
        LINGARA_EMBED_EMPTY
    }
}

pub(crate) fn set_answer(status: i32, json: Vec<u8>) {
    LAST_ANSWER.with(|slot| *slot.borrow_mut() = Some((status, json)));
}

pub(crate) fn clear_answer() {
    LAST_ANSWER.with(|slot| *slot.borrow_mut() = None);
}

/// Records a fault as this thread's last error and returns its status.
pub(crate) fn fail(fault: Fault) -> i32 {
    LAST_ERROR.with(|slot| *slot.borrow_mut() = Some(redact(&fault.message)));
    fault.status
}

/// # Safety
/// `out` passed `check`.
pub(crate) unsafe fn copy_answer(out: &Out) -> i32 {
    LAST_ANSWER.with(|slot| match slot.borrow().as_ref() {
        Some((status, json)) => unsafe { out.write(json, *status) },
        None => unsafe { out.empty() },
    })
}

/// # Safety
/// `out` passed `check`.
pub(crate) unsafe fn copy_error(out: &Out) -> i32 {
    LAST_ERROR.with(|slot| match slot.borrow().as_ref() {
        Some(text) => unsafe { out.write(text.as_bytes(), LINGARA_EMBED_OK) },
        None => unsafe { out.empty() },
    })
}
