use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::types::*;

fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap().map(Result::unwrap) {
        let path = entry.path();
        // `server/` is the server tier (1.10.26af D12), which takes the
        // library's client and verifier by design; the player API never does.
        if path.is_dir() && path.file_name().is_some_and(|n| n != "tests" && n != "server") {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// The names a file imports from the library: `use lingara::{A, b::C}`.
fn imported(text: &str) -> BTreeSet<String> {
    let uses = text.lines().filter(|l| l.starts_with("use lingara"));
    let words = uses.flat_map(|l| l.split(|c: char| !c.is_alphanumeric() && c != '_').map(str::to_owned).collect::<Vec<_>>());
    words.filter(|w| w.chars().next().is_some_and(char::is_uppercase)).collect()
}

// 1.10.26y AC19
#[test]
fn no_library_type_crosses_the_public_api() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let lib = std::fs::read_to_string(root.join("lib.rs")).unwrap();
    assert!(!lib.contains("pub use lingara"), "lib.rs re-exports nothing from the library");
    let mut files = Vec::new();
    sources(&root, &mut files);
    let mut found = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        let names = imported(&text);
        for line in text.lines().map(str::trim_start).filter(|l| l.starts_with("pub ") && !l.starts_with("pub(crate)")) {
            let named = line.contains("lingara::") || line.split(|c: char| !c.is_alphanumeric() && c != '_').any(|w| names.contains(w));
            if named {
                found.push(format!("{}: {line}", file.display()));
            }
        }
    }
    assert!(found.is_empty(), "public items naming a library type:\n{}", found.join("\n"));
}

// 1.10.26y AC20
#[test]
fn dependencies_and_default_features_are_d1s() {
    let manifest: toml::Table = include_str!("../../Cargo.toml").parse().unwrap();
    // The native-only half sits under a target table, so wasm32 meets the
    // crate's own refusal first (make check-bevy-wasm-fails); D1 is the union.
    let mut deps = manifest["dependencies"].as_table().unwrap().clone();
    deps.extend(manifest["target"]["cfg(not(target_arch = \"wasm32\"))"]["dependencies"].as_table().unwrap().clone());
    let required: BTreeSet<&str> = deps.iter().filter(|(_, v)| !v.get("optional").and_then(toml::Value::as_bool).unwrap_or(false)).map(|(k, _)| k.as_str()).collect();
    let d1: BTreeSet<&str> = ["getrandom", "lingara", "log", "reqwest", "serde", "serde_json", "thiserror", "tokio"].into();
    assert_eq!(required, d1);
    assert!(deps["lingara"]["default-features"].as_bool() == Some(false) && deps["reqwest"]["default-features"].as_bool() == Some(false));
    let features = manifest["features"].as_table().unwrap();
    assert_eq!(features["default"].as_array().unwrap(), &[toml::Value::from("rustls")]);
    assert_eq!(features["schema"].as_array().unwrap(), &[toml::Value::from("dep:schemars")]);
    assert!(features.keys().all(|k| ["default", "rustls", "native-tls", "schema", "server"].contains(&k.as_str())), "the server half is V1's feature, not a default");
    assert_eq!(features["server"].as_array().map(Vec::len), Some(0), "the server feature enables nothing (1.10.26af AC27)");
}

fn clone<T: Clone>() {}

// 1.10.26y AC32
#[test]
fn bindings_clone_events_read_limits_and_omit_config_fields() {
    clone::<EmbedEvent>();
    clone::<EmbedError>();
    clone::<PracticeSnapshot>();
    clone::<DialogueSnapshot>();
    clone::<LingaraEvent>();
    clone::<StreamState>();
    clone::<NpcTurnUpdate>();
    clone::<Command>();
    clone::<CommandOutcome>();

    let limits = include_str!("../limits.rs");
    let consts: Vec<&str> = limits.lines().filter(|l| l.contains("const ")).collect();
    assert!(!consts.is_empty() && consts.iter().all(|l| l.starts_with("pub const ")), "every limit is pub");
    assert!(include_str!("../lib.rs").contains("\npub mod limits;"), "and its module is public");

    assert_eq!(serde_json::from_str::<EmbedConfig>("{}").unwrap(), EmbedConfig::default());
    let eight: EmbedConfig = serde_json::from_str(r#"{"queue_capacity": 8}"#).unwrap();
    assert_eq!(eight, EmbedConfig { queue_capacity: 8, ..EmbedConfig::default() });
    let d = EmbedConfig::default();
    let defaults = (d.base_url.as_str(), &d.token_source, d.stream_on_start, d.queue_capacity, d.token_wait_timeout_ms, d.stream_idle_timeout_ms, d.log_level);
    assert_eq!(defaults, ("https://api.getlingara.com", &TokenSourceConfig::Poll, true, 1024, 30_000, 45_000, LogLevel::Warn));
    assert!(d.api_version.is_none() && d.event_types.is_empty() && d.resume_cursor.is_none() && d.user_agent_suffix.is_none());
}
