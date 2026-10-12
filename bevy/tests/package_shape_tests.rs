//! The dependency rule (ADR 1.10.26z D1, D12): a game that adds the plugin
//! gains exactly the core and Bevy's ECS, never a renderer.

use toml::Value;

fn manifest() -> Value {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).expect("bevy/Cargo.toml");
    text.parse().expect("valid TOML")
}

/// 1.10.26z AC12: `[dependencies]` is exactly `lingara-embed` and `bevy`,
/// with `bevy`'s default features off and none enabled.
#[test]
fn only_the_core_and_bevy_without_defaults() {
    let manifest = manifest();
    let deps = manifest["dependencies"].as_table().expect("[dependencies]");
    let mut names: Vec<&str> = deps.keys().map(String::as_str).collect();
    names.sort_unstable();
    assert_eq!(names, ["bevy", "lingara-embed"]);
    let bevy = &deps["bevy"];
    assert_eq!(bevy.get("default-features").and_then(Value::as_bool), Some(false));
    assert!(bevy.get("features").and_then(Value::as_array).is_none_or(Vec::is_empty));
    let core = deps["lingara-embed"]["version"].as_str().expect("a version");
    assert!(core.starts_with('='), "the core is pinned exactly: {core}");
    assert!(manifest.get("features").is_none(), "the plugin has no features of its own");
}
