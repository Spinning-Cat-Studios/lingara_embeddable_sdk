use crate::tests::{DIALOGUE, OFFLINE, PANIC_COMMAND, call, exec, exec_bytes, last_error, open};
use crate::*;

// 1.10.26aa AC4
#[test]
fn a_panic_becomes_a_status_and_poisons_the_handle() {
    let handle = open(OFFLINE);
    assert_eq!(exec_bytes(handle, PANIC_COMMAND).0, LINGARA_EMBED_ERR_PANIC);
    assert!(last_error().starts_with("ERR_PANIC"), "{}", last_error());

    assert_eq!(exec(handle, DIALOGUE).0, LINGARA_EMBED_ERR_CLOSED);
    assert!(last_error().starts_with("ERR_CLOSED"), "{}", last_error());
    assert_eq!(call(256, |b, c, l| unsafe { lingara_embed_poll(handle, b, c, l) }).0, LINGARA_EMBED_ERR_CLOSED);
    assert_eq!(call(256, |b, c, l| unsafe { lingara_embed_status(handle, b, c, l) }).0, LINGARA_EMBED_ERR_CLOSED);
    // free still frees, and the process is still here to say so.
    unsafe { lingara_embed_free(handle) };
}
