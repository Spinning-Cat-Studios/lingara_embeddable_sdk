//! The EK harness on the Rust surface (ADR 1.10.26y D12): every EK case's
//! `host:` script, driven through `lingara_embed::Embed` and nothing else.
//!
//! `conformance-server run --tier ek --lang core -- <this>` starts it with
//! `LINGARA_EMBED_SERVER_URL` and `LINGARA_EMBED_RESULTS`. For each case
//! under `conformance/ek/cases/` it fetches the case, arms it, runs the
//! script, finishes it, and writes one result line merging the server's
//! verdict with its own. It reports every case: there is no skip.

mod host;
mod matching;

#[cfg(test)]
mod handoff_tests;
#[cfg(test)]
mod json_shape_tests;
#[cfg(test)]
mod spec_limits_tests;

use std::io::Write as _;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

/// The language this harness reports: `make/core.mk`'s `EK_LANGS` entry.
const LANG: &str = "core";

fn main() {
    let server = std::env::var("LINGARA_EMBED_SERVER_URL").expect("LINGARA_EMBED_SERVER_URL");
    let results = std::env::var("LINGARA_EMBED_RESULTS").expect("LINGARA_EMBED_RESULTS");
    let cases = std::env::var("LINGARA_EMBED_CASES").unwrap_or_else(|_| "conformance/ek/cases".into());
    let mut out = std::fs::File::create(&results).expect("the results file");
    let http = reqwest::blocking::Client::new();
    // A failing case is in the results, which the server judges; the exit
    // code says only that every case was run and reported.
    for id in case_ids(Path::new(&cases)) {
        let line = run_case(&http, &server, &id);
        writeln!(out, "{line}").expect("write a result");
    }
}

/// `<group>/<slug>.yaml` → `<group>.<slug>`, sorted.
fn case_ids(dir: &Path) -> Vec<String> {
    let mut ids = Vec::new();
    let groups = std::fs::read_dir(dir).map(|d| d.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect::<Vec<PathBuf>>()).unwrap_or_default();
    for group in groups {
        for file in std::fs::read_dir(&group).into_iter().flatten().flatten().map(|e| e.path()) {
            if file.extension().is_some_and(|e| e == "yaml") {
                let (g, s) = (group.file_name().unwrap_or_default(), file.file_stem().unwrap_or_default());
                ids.push(format!("{}.{}", g.to_string_lossy(), s.to_string_lossy()));
            }
        }
    }
    ids.sort();
    ids
}

/// One case end to end. Any failure to reach the server is a mismatch, so
/// the case is still reported.
fn run_case(http: &reqwest::blocking::Client, server: &str, id: &str) -> Value {
    let control = |verb: &str| format!("{server}/__conformance/{verb}/{id}");
    let case: Value = match http.get(control("cases")).send().and_then(|r| r.error_for_status()).and_then(|r| r.json()) {
        Ok(case) => case,
        Err(e) => return json!({ "case": id, "lang": LANG, "result": "fail", "mismatches": [format!("cannot read the case: {e}")] }),
    };
    let mut mismatches: Vec<Value> = Vec::new();
    if let Err(e) = http.post(control("arm")).send().and_then(|r| r.error_for_status()) {
        mismatches.push(format!("cannot arm: {e}").into());
    }
    let run = host::run(server, &case);
    mismatches.extend(run.problems.into_iter().map(Value::from));
    if let Some(want) = case.pointer("/expect/events").and_then(Value::as_array) {
        let (got, want) = (matching::without_nulls(&Value::from(run.matched)), matching::without_nulls(&Value::from(want.clone())));
        if got != want {
            mismatches.push(format!("{id}: the host saw {got}, expected {want}").into());
        }
    }
    match http.post(control("finish")).json(&json!({})).send().and_then(|r| r.json::<Value>()) {
        Ok(verdict) => mismatches.extend(verdict["mismatches"].as_array().cloned().unwrap_or_default()),
        Err(e) => mismatches.push(format!("cannot finish: {e}").into()),
    }
    let result = if mismatches.is_empty() { "pass" } else { "fail" };
    json!({ "case": id, "lang": LANG, "result": result, "mismatches": mismatches })
}
