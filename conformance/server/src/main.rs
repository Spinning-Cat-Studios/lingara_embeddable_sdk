//! The embed kits' conformance server (ADR 1.10.26x D5). One process plays
//! Lingara for both tiers: it answers player kits (EK), and mints for and
//! delivers signed webhooks to server kits (ES).

mod case;
mod coverage;
mod judge;
mod run;
mod serve;
mod sign;
mod spec;
mod sse;
mod state;

#[cfg(test)]
mod coverage_tests;
#[cfg(test)]
mod judge_tests;
#[cfg(test)]
mod run_tests;
#[cfg(test)]
mod serve_tests;
#[cfg(test)]
mod sign_tests;
#[cfg(test)]
mod test_support;

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand, ValueEnum};

use crate::case::{load_dirs, Tier};
use crate::spec::Frozen;
use crate::state::Shared;

#[derive(Parser)]
#[command(name = "conformance-server", about = "Plays Lingara for the embed kits' conformance tiers")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Clone, Copy, ValueEnum)]
enum TierArg {
    Ek,
    Es,
}

impl From<TierArg> for Tier {
    fn from(t: TierArg) -> Self {
        match t {
            TierArg::Ek => Tier::Ek,
            TierArg::Es => Tier::Es,
        }
    }
}

#[derive(Subcommand)]
enum Cmd {
    /// Run one binding's harness against every case of a tier.
    Run {
        #[arg(long)]
        tier: TierArg,
        #[arg(long)]
        lang: String,
        /// Comma-separated case ids, for local debugging; refused when CI is set.
        #[arg(long)]
        only: Option<String>,
        #[arg(long, default_value = "conformance")]
        conformance: PathBuf,
        #[arg(long, default_value = "spec/versions.toml")]
        registry: PathBuf,
        #[arg(last = true)]
        command: Vec<String>,
    },
    /// Serve case folders with no harness; each finish answers its verdict.
    Serve {
        #[arg(long, required = true, num_args = 1..)]
        cases: Vec<PathBuf>,
        /// `0` picks a free port; the first stdout line is `listening <port>`.
        #[arg(long, default_value_t = 0)]
        port: u16,
        #[arg(long, default_value = "spec/versions.toml")]
        registry: PathBuf,
    },
    /// The case inventory against the tier's lists and the frozen current.
    CheckCoverage {
        #[arg(long)]
        tier: TierArg,
        #[arg(long)]
        registry: PathBuf,
        #[arg(long, default_value = "")]
        langs: String,
        #[arg(long, default_value = "conformance")]
        conformance: PathBuf,
    },
    /// Only that every operations.toml entry is in the frozen current.
    CheckSpec {
        #[arg(long)]
        tier: TierArg,
        #[arg(long)]
        registry: PathBuf,
        #[arg(long, default_value = "conformance")]
        conformance: PathBuf,
    },
}

fn main() {
    let code = match Cli::parse().command {
        Cmd::Run { tier, lang, only, conformance, registry, command } => {
            report(run_tier(tier.into(), &lang, only.as_deref(), &Inputs { conformance, registry }, &command))
        }
        Cmd::Serve { cases, port, registry } => serve_only(&cases, port, &registry),
        Cmd::CheckCoverage { tier, registry, langs, conformance } => coverage(tier.into(), &registry, &langs, &conformance),
        Cmd::CheckSpec { tier, registry, conformance } => {
            let tier: Tier = tier.into();
            report(Frozen::load(&registry).and_then(|f| coverage::check_spec(&conformance.join(tier.name()), &f)))
        }
    };
    std::process::exit(code);
}

/// Exit 0 when there is nothing to report, 1 otherwise.
fn report(result: Result<Vec<String>, String>) -> i32 {
    match result {
        Ok(problems) if problems.is_empty() => 0,
        Ok(problems) => {
            problems.iter().for_each(|p| eprintln!("✗ {p}"));
            1
        }
        Err(e) => {
            eprintln!("✗ {e}");
            1
        }
    }
}

fn coverage(tier: Tier, registry: &Path, langs: &str, conformance: &Path) -> i32 {
    let langs: Vec<String> = langs.split_whitespace().map(String::from).collect();
    let result = Frozen::load(registry).and_then(|f| coverage::check_coverage(tier, conformance, &f, &langs));
    if let Ok(r) = &result {
        r.notes.iter().for_each(|n| println!("{n}"));
    }
    report(result.map(|r| r.problems))
}

fn serve_only(dirs: &[PathBuf], port: u16, registry: &Path) -> i32 {
    let (loaded, problems) = load_dirs(dirs);
    let frozen = match Frozen::load(registry) {
        Ok(f) if problems.is_empty() => f,
        Ok(_) => return report(Ok(problems)),
        Err(e) => return report(Err(e)),
    };
    let shared = Shared::new(loaded.into_iter().map(|l| l.case).collect(), frozen);
    let Ok(runtime) = tokio::runtime::Runtime::new() else { return 1 };
    runtime.block_on(async {
        match serve::start(shared, port).await {
            Ok((port, task)) => {
                println!("{}", serve::listening(port));
                let _ = task.await;
                0
            }
            Err(e) => report(Err(e.to_string())),
        }
    })
}

/// Where a run reads its cases and its frozen spec.
struct Inputs {
    conformance: PathBuf,
    registry: PathBuf,
}

fn run_tier(tier: Tier, lang: &str, only: Option<&str>, inputs: &Inputs, command: &[String]) -> Result<Vec<String>, String> {
    let only = run::check_only(only, std::env::var("CI").ok().as_deref())?;
    let (loaded, problems) = load_dirs(&[inputs.conformance.join(tier.name()).join("cases")]);
    if !problems.is_empty() {
        return Err(problems.join("\n"));
    }
    let mut expected: Vec<String> = loaded.iter().map(|l| l.case.id.clone()).collect();
    if let Some(only) = &only {
        expected.retain(|id| only.contains(id));
    }
    let shared = Shared::new(loaded.into_iter().map(|l| l.case).collect(), Frozen::load(&inputs.registry)?);
    let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    let (port, _server) = runtime.block_on(serve::start(shared.clone(), 0)).map_err(|e| e.to_string())?;
    let results = run::results_path(tier, lang);
    let _ = std::fs::remove_file(&results);
    let env = run::harness_env(tier, port, &results);
    let (mut child, receiver) = run::spawn(tier, command, &env, run::LISTEN_TIMEOUT)?;
    shared.lock().receiver = receiver.map(|p| format!("http://127.0.0.1:{p}/"));
    let status = child.wait().map_err(|e| e.to_string())?;
    let text = std::fs::read_to_string(&results).unwrap_or_default();
    let log = shared.lock().log.clone();
    let mut problems = run::verify(lang, &run::parse_results(&text), &expected, &log);
    problems.extend(shared.lock().stray.iter().map(|s| format!("stray request: {s}")));
    if !status.success() {
        problems.push(format!("the harness exited with {status}"));
    }
    if problems.is_empty() {
        println!("conformance {} {lang}: every case passed", tier.name());
    }
    Ok(problems)
}
