//! The `server` feature costs a game nothing (ADR 1.10.26af D2): it enables
//! no optional dependency, the crate's own normal dependency tree is the
//! same with and without it, and the module exists only under it. Outside
//! the gated module, so it runs under the default features.

use std::path::Path;
use std::process::Command;

fn cargo(args: &[&str]) -> String {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let out = Command::new(cargo).args(args).current_dir(env!("CARGO_MANIFEST_DIR")).output().expect("cargo runs");
    assert!(out.status.success(), "cargo {args:?}: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout).expect("utf-8")
}

fn tree(features: &[&str]) -> String {
    let mut args = vec!["tree", "--offline", "--locked", "-p", "lingara-embed", "-e", "normal", "--prefix", "none"];
    args.extend_from_slice(features);
    let mut lines: Vec<String> = cargo(&args).lines().map(|l| l.trim_end_matches(" (*)").to_string()).collect();
    lines.sort();
    lines.dedup();
    lines.join("\n")
}

// 1.10.26af AC27
#[test]
fn server_adds_no_dependency() {
    let metadata: serde_json::Value =
        serde_json::from_str(&cargo(&["metadata", "--offline", "--locked", "--format-version", "1", "--no-deps"])).expect("metadata");
    let package = metadata["packages"].as_array().expect("packages").iter().find(|p| p["name"] == "lingara-embed").expect("lingara-embed");
    let server = package["features"]["server"].as_array().expect("a server feature");
    assert!(server.is_empty(), "the server feature enables nothing: {server:?}");

    assert_eq!(tree(&[]), tree(&["--features", "server"]), "the normal dependency tree moved under --features server");

    let lib = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs")).expect("lib.rs");
    let gated = lib.lines().collect::<Vec<_>>().windows(2).any(|w| w[0].trim() == "#[cfg(feature = \"server\")]" && w[1].trim() == "pub mod server;");
    assert!(gated, "lib.rs declares `pub mod server` under #[cfg(feature = \"server\")]");
    assert_eq!(lib.matches("mod server").count(), 1, "one declaration, and it is the gated one");
}
