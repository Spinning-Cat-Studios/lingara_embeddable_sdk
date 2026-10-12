//! Compiles `c/smoke.c` against the header alone, links the staticlib, and
//! runs it (ADR 1.10.26aa D12, AC12). `#[ignore]`, so the workspace-wide
//! `cargo test` never runs it: `make test-ffi` builds the staticlib first and
//! passes its path and rustc's native-static-libs list. Run that way it
//! fails, never skips, when either is unset, so it cannot pass on a stale
//! artefact.

use std::path::{Path, PathBuf};
use std::process::Command;

fn required(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("{name} is unset: run this test through `make test-ffi`"))
}

/// The compiler line for this host: MSVC `cl` on Windows, `cc` elsewhere.
fn compile(source: &Path, include: &Path, staticlib: &str, native_libs: &str, out: &Path) -> Command {
    let libs = native_libs.split_whitespace();
    if cfg!(target_env = "msvc") {
        let mut cl = Command::new("cl");
        cl.args(["/nologo", "/std:c11", "/W4", "/WX"]).arg(format!("/I{}", include.display())).arg(source).arg(staticlib);
        cl.args(libs).arg(format!("/Fe:{}", out.display()));
        cl
    } else {
        let mut cc = Command::new(std::env::var("CC").unwrap_or_else(|_| "cc".into()));
        cc.args(["-std=c99", "-Wall", "-Werror", "-pedantic"]).arg("-I").arg(include).arg(source).arg(staticlib);
        cc.args(libs).arg("-o").arg(out);
        cc
    }
}

// 1.10.26aa AC12
#[test]
#[ignore = "run by `make test-ffi`, after the staticlib is built"]
fn the_header_compiles_and_links_as_c() {
    let staticlib = required("LINGARA_EMBED_STATICLIB");
    let native_libs = required("LINGARA_EMBED_NATIVE_LIBS");
    assert!(Path::new(&staticlib).is_file(), "no staticlib at {staticlib}");
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    let exe = out_dir.join(if cfg!(windows) { "smoke.exe" } else { "smoke" });

    let mut compiler = compile(&dir.join("tests/c/smoke.c"), &dir.join("include"), &staticlib, &native_libs, &exe);
    let built = compiler.current_dir(&out_dir).output().expect("a C compiler");
    assert!(built.status.success(), "smoke.c did not compile:\n{}{}", String::from_utf8_lossy(&built.stdout), String::from_utf8_lossy(&built.stderr));

    let ran = Command::new(&exe).output().expect("run smoke");
    let stdout = String::from_utf8_lossy(&ran.stdout);
    assert!(ran.status.success(), "smoke failed:\n{stdout}{}", String::from_utf8_lossy(&ran.stderr));
    assert!(stdout.contains(": ok"), "{stdout}");
}
