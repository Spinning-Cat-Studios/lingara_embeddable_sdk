use std::path::PathBuf;

/// `LINGARA_EMBED_BLESS=1` rewrites the committed file instead (`make ffi-header`).
pub fn bless() -> bool {
    std::env::var_os("LINGARA_EMBED_BLESS").is_some_and(|v| v == "1")
}

pub fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn generate() -> Vec<u8> {
    let config = cbindgen::Config::from_file(crate_dir().join("cbindgen.toml")).expect("cbindgen.toml");
    let bindings = cbindgen::Builder::new().with_crate(crate_dir()).with_config(config).generate().expect("cbindgen");
    let mut out = Vec::new();
    bindings.write(&mut out);
    out
}

// 1.10.26aa AC10
#[test]
fn the_committed_header_is_cbindgens_output() {
    let path = crate_dir().join("include/lingara_embed.h");
    let fresh = generate();
    if bless() {
        std::fs::write(&path, &fresh).expect("write the header");
    }
    let committed = std::fs::read(&path).expect("include/lingara_embed.h is committed");
    assert!(committed == fresh, "include/lingara_embed.h is stale: run `make ffi-header`");
    let text = String::from_utf8(fresh).expect("UTF-8");
    for name in ["new", "free", "execute", "poll", "status", "last_answer", "last_error", "abi_version", "version"] {
        assert!(text.contains(&format!("lingara_embed_{name}(")), "the header declares lingara_embed_{name}");
    }
    assert!(text.contains("#define LINGARA_EMBED_ERR_ABI_MISMATCH -8"));
}
