use std::ffi::CStr;

use crate::tests::{OFFLINE, call, last_error, on_fresh_thread, open};
use crate::*;

// 1.10.26aa AC1
#[test]
fn new_refuses_another_abi_major() {
    assert_eq!(lingara_embed_abi_version(), (LINGARA_EMBED_ABI_VERSION_MAJOR << 16) | LINGARA_EMBED_ABI_VERSION_MINOR);
    let version = unsafe { CStr::from_ptr(lingara_embed_version()) };
    assert_eq!(version.to_str().unwrap(), env!("CARGO_PKG_VERSION"));

    on_fresh_thread(|| {
        let mut handle: *mut LingaraEmbed = std::ptr::dangling_mut();
        let other = LINGARA_EMBED_ABI_VERSION_MAJOR + 1;
        let status = unsafe { lingara_embed_new(other, OFFLINE.as_ptr(), OFFLINE.len(), &mut handle) };
        assert_eq!(status, LINGARA_EMBED_ERR_ABI_MISMATCH);
        assert!(handle.is_null(), "no handle is made");
        let message = last_error();
        assert!(message.contains(&format!("major {other}")), "{message}");
        assert!(message.contains(&format!("ABI {LINGARA_EMBED_ABI_VERSION_MAJOR}.")), "{message}");
    });
}

// 1.10.26aa AC2
#[test]
fn every_entry_point_refuses_null() {
    let null = std::ptr::null_mut::<LingaraEmbed>();
    let cmd = br#"{"type": "stream_pause"}"#;
    let mut len = 0usize;
    let mut buf = [0u8; 8];
    let len_p = &raw mut len;
    let buf_p = buf.as_mut_ptr();
    unsafe {
        assert_eq!(lingara_embed_new(1, std::ptr::null(), 0, std::ptr::null_mut()), LINGARA_EMBED_ERR_NULL, "new: out_handle");
        let mut h = std::ptr::null_mut();
        assert_eq!(lingara_embed_new(1, std::ptr::null(), 3, &mut h), LINGARA_EMBED_ERR_NULL, "new: config with a length");
        assert_eq!(lingara_embed_execute(null, cmd.as_ptr(), cmd.len(), buf_p, 8, len_p), LINGARA_EMBED_ERR_NULL, "execute: handle");
        assert_eq!(lingara_embed_poll(null, buf_p, 8, len_p), LINGARA_EMBED_ERR_NULL, "poll: handle");
        assert_eq!(lingara_embed_status(null, buf_p, 8, len_p), LINGARA_EMBED_ERR_NULL, "status: handle");
        assert_eq!(lingara_embed_last_answer(buf_p, 8, std::ptr::null_mut()), LINGARA_EMBED_ERR_NULL, "last_answer: out_len");
        assert_eq!(lingara_embed_last_error(buf_p, 8, std::ptr::null_mut()), LINGARA_EMBED_ERR_NULL, "last_error: out_len");
        assert_eq!(lingara_embed_last_error(std::ptr::null_mut(), 8, len_p), LINGARA_EMBED_ERR_NULL, "a NULL buf with a cap");
        lingara_embed_free(null);
    }

    let handle = open(OFFLINE);
    unsafe {
        assert_eq!(lingara_embed_execute(handle, std::ptr::null(), 0, buf_p, 8, len_p), LINGARA_EMBED_ERR_NULL, "execute: command");
        assert_eq!(lingara_embed_execute(handle, cmd.as_ptr(), cmd.len(), buf_p, 8, std::ptr::null_mut()), LINGARA_EMBED_ERR_NULL);
        assert_eq!(lingara_embed_execute(handle, cmd.as_ptr(), cmd.len(), std::ptr::null_mut(), 8, len_p), LINGARA_EMBED_ERR_NULL);
        assert_eq!(lingara_embed_poll(handle, buf_p, 8, std::ptr::null_mut()), LINGARA_EMBED_ERR_NULL, "poll: out_len");
        assert_eq!(lingara_embed_poll(handle, std::ptr::null_mut(), 8, len_p), LINGARA_EMBED_ERR_NULL, "poll: buf with a cap");
        assert_eq!(lingara_embed_status(handle, buf_p, 8, std::ptr::null_mut()), LINGARA_EMBED_ERR_NULL, "status: out_len");
        assert_eq!(lingara_embed_status(handle, std::ptr::null_mut(), 8, len_p), LINGARA_EMBED_ERR_NULL, "status: buf with a cap");
    }
    assert!(last_error().starts_with("ERR_NULL"), "{}", last_error());

    // A NULL buf with cap 0 is the size probe, not a refusal.
    let (status, needed, _) = call(0, |b, c, l| unsafe { lingara_embed_status(handle, b, c, l) });
    assert_eq!(status, LINGARA_EMBED_BUFFER_TOO_SMALL);
    assert!(needed > 0);
    unsafe { lingara_embed_free(handle) };
}
