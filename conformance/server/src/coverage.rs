//! `check-coverage` and `check-spec` (ADR 1.10.26x D5).
//!
//! The *always* rules hold on every tier: every `operations.toml` entry is in
//! the frozen `current`, every case deserialises with an id matching its
//! path, names only its tier's behaviours and operations, every named
//! vectors file parses, and a tier with languages has behaviours. Once a tier
//! has behaviours, each needs a case (or a vectors file) and each operation
//! needs a case. So the scaffold is green, and the first binding to join
//! cannot pass on nothing.

use std::collections::BTreeSet;
use std::path::Path;

use serde::Deserialize;

use crate::case::{load_dirs, Case, Tier};
use crate::spec::Frozen;

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Behaviours {
    #[serde(default)]
    pub behaviour: Vec<Behaviour>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Behaviour {
    pub id: String,
    pub title: String,
    /// A vectors-only behaviour: covered by this file under `conformance/vectors/`.
    #[serde(default)]
    pub vectors: Option<String>,
}

/// A tier's closed lists: OpenAPI `operationId`s and AsyncAPI message names.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Operations {
    #[serde(default)]
    pub operations: Vec<String>,
    #[serde(default)]
    pub messages: Vec<String>,
}

fn read_toml<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// `check-spec`: every entry of the tier's `operations.toml` is in `frozen`.
pub fn check_spec(tier_dir: &Path, frozen: &Frozen) -> Result<Vec<String>, String> {
    let ops: Operations = read_toml(&tier_dir.join("operations.toml"))?;
    let mut found: Vec<String> = ops
        .operations
        .iter()
        .filter(|o| frozen.operation(o).is_none())
        .map(|o| format!("operation {o} is not in the frozen current {}", frozen.id))
        .collect();
    found.extend(
        ops.messages.iter().filter(|m| !frozen.has_message(m)).map(|m| format!("message {m} is not in the frozen current {}", frozen.id)),
    );
    Ok(found)
}

/// What `check-coverage` found: problems fail the run, notes are printed.
#[derive(Debug, Default)]
pub struct Report {
    pub problems: Vec<String>,
    pub notes: Vec<String>,
}

/// `check-coverage` for one tier. `conformance` is the directory holding
/// `ek/`, `es/` and `vectors/`.
pub fn check_coverage(tier: Tier, conformance: &Path, frozen: &Frozen, langs: &[String]) -> Result<Report, String> {
    let tier_dir = conformance.join(tier.name());
    let behaviours: Behaviours = read_toml(&tier_dir.join("behaviours.toml"))?;
    let ops: Operations = read_toml(&tier_dir.join("operations.toml"))?;
    let mut report = Report { problems: check_spec(&tier_dir, frozen)?, ..Report::default() };
    let (loaded, load_problems) = load_dirs(&[tier_dir.join("cases")]);
    report.problems.extend(load_problems);
    let cases: Vec<&Case> = loaded.iter().map(|l| &l.case).collect();
    report.problems.extend(case_problems(tier, &cases, &behaviours, &ops));
    report.problems.extend(vectors_problems(&behaviours, &conformance.join("vectors")));
    let name = tier.name();
    let owner = if tier == Tier::Ek { "K1" } else { "V1" };
    if behaviours.behaviour.is_empty() {
        if !langs.is_empty() {
            report.problems.push(format!("tier {name}: behaviours.toml is empty, but languages have joined: {}", langs.join(" ")));
        }
        report.notes.push(format!("tier {name}: no behaviours yet ({owner})"));
        return Ok(report);
    }
    report.problems.extend(gap_problems(&cases, &behaviours, &ops));
    Ok(report)
}

/// Always: each case is its tier's, and names only that tier's lists.
fn case_problems(tier: Tier, cases: &[&Case], behaviours: &Behaviours, ops: &Operations) -> Vec<String> {
    let known: BTreeSet<&str> = behaviours.behaviour.iter().map(|b| b.id.as_str()).collect();
    let mut found = Vec::new();
    for case in cases {
        if case.tier != tier {
            found.push(format!("{}: tier {} is under {}/", case.id, case.tier.name(), tier.name()));
        }
        for b in case.behaviours.iter().filter(|b| !known.contains(b.as_str())) {
            found.push(format!("{}: behaviour {b} is not in behaviours.toml", case.id));
        }
        for op in case.serve.iter().filter_map(|s| s.route.operation.as_deref()).filter(|o| !ops.operations.iter().any(|x| x == o)) {
            found.push(format!("{}: operation {op} is not in operations.toml", case.id));
        }
        for m in case.deliver.iter().flatten().map(|d| d.message.as_str()).filter(|m| !ops.messages.iter().any(|x| x == m)) {
            found.push(format!("{}: message {m} is not in operations.toml", case.id));
        }
    }
    found
}

/// Always: a named vectors file exists and parses as JSON.
fn vectors_problems(behaviours: &Behaviours, vectors: &Path) -> Vec<String> {
    let named = behaviours.behaviour.iter().filter_map(|b| b.vectors.as_ref().map(|v| (b, v)));
    named
        .filter_map(|(b, file)| {
            let text = std::fs::read_to_string(vectors.join(file)).map_err(|e| format!("{}: vectors {file}: {e}", b.id));
            let parsed = text.and_then(|t| serde_json::from_str::<serde_json::Value>(&t).map_err(|e| format!("{}: vectors {file} is not JSON: {e}", b.id)));
            parsed.err()
        })
        .collect()
}

/// Once a tier has behaviours: each behaviour and each operation has a case.
fn gap_problems(cases: &[&Case], behaviours: &Behaviours, ops: &Operations) -> Vec<String> {
    let covered: BTreeSet<&str> = cases.iter().flat_map(|c| c.behaviours.iter().map(String::as_str)).collect();
    let exercised: BTreeSet<&str> = cases
        .iter()
        .flat_map(|c| c.serve.iter().filter_map(|s| s.route.operation.as_deref()).chain(c.deliver.iter().flatten().map(|d| d.message.as_str())))
        .collect();
    let mut found: Vec<String> = behaviours
        .behaviour
        .iter()
        .filter(|b| b.vectors.is_none() && !covered.contains(b.id.as_str()))
        .map(|b| format!("{} ({}): no case covers this behaviour", b.id, b.title))
        .collect();
    let entries = ops.operations.iter().chain(&ops.messages);
    found.extend(entries.filter(|o| !exercised.contains(o.as_str())).map(|o| format!("{o}: no case exercises it")));
    found
}
