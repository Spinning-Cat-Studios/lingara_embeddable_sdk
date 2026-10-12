//! The compatibility table (ADR 1.10.26z D2): its newest row is the Bevy
//! minor `Cargo.toml` pins, so a pin bump that forgets the README fails.

const README: &str = include_str!("../README.md");
const MANIFEST: &str = include_str!("../Cargo.toml");

/// `0.20` from `bevy = { version = "0.20", … }`.
fn pinned_bevy() -> String {
    let manifest: toml::Value = MANIFEST.parse().expect("valid TOML");
    let version = manifest["dependencies"]["bevy"]["version"].as_str().expect("bevy's version");
    version.trim_start_matches(['^', '=', '~']).split('.').take(2).collect::<Vec<_>>().join(".")
}

/// The Bevy column of the table's rows, in order.
fn table_rows() -> Vec<String> {
    let table = README.split("| bevy_lingara | Bevy |").nth(1).expect("the compatibility table");
    table
        .lines()
        .skip(2)
        .take_while(|line| line.starts_with('|'))
        .filter_map(|line| line.split('|').nth(2).map(|cell| cell.trim().to_owned()))
        .collect()
}

/// 1.10.26z AC13: the newest row names the pinned minor.
#[test]
fn the_table_names_the_pinned_bevy() {
    let rows = table_rows();
    assert_eq!(rows.last(), Some(&pinned_bevy()), "README rows {rows:?}");
}
