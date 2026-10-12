//! `run --tier ek|es --lang <lang> -- <harness…>` (ADR 1.10.26x D5): serve
//! every case of the tier, spawn one harness against it, and trust neither
//! side alone. The run fails when the results file omits an armed case,
//! reports a case never armed, or holds anything but `pass` (C2 D12's
//! cross-check). There is no `skip`.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use serde::Deserialize;

use crate::case::Tier;
use crate::sign;
use crate::state::LogEntry;

/// How long an ES harness has to print `listening <port>`.
pub const LISTEN_TIMEOUT: Duration = Duration::from_secs(30);

/// One line of the results file. Other keys are informational and ignored.
#[derive(Debug, Clone, Deserialize)]
pub struct ResultLine {
    pub case: String,
    pub lang: String,
    pub result: String,
    #[serde(default)]
    pub mismatches: Vec<serde_json::Value>,
}

/// `--only` is for local debugging; CI runs every case.
pub fn check_only(only: Option<&str>, ci: Option<&str>) -> Result<Option<BTreeSet<String>>, String> {
    let Some(only) = only.filter(|o| !o.trim().is_empty()) else { return Ok(None) };
    if ci.is_some_and(|c| !c.is_empty() && c != "false" && c != "0") {
        return Err("--only is refused when CI is set: CI runs every case".into());
    }
    Ok(Some(only.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()))
}

/// One JSON object per non-blank line.
pub fn parse_results(text: &str) -> Vec<Result<ResultLine, String>> {
    let lines = text.lines().enumerate().filter(|(_, l)| !l.trim().is_empty());
    lines.map(|(n, l)| serde_json::from_str(l).map_err(|e| format!("line {}: {e}", n + 1))).collect()
}

/// Every problem with a results file, given the cases expected and the log.
pub fn verify(lang: &str, lines: &[Result<ResultLine, String>], expected: &[String], log: &[LogEntry]) -> Vec<String> {
    let armed: BTreeSet<&str> = log.iter().filter_map(|e| if let LogEntry::Arm(id) = e { Some(id.as_str()) } else { None }).collect();
    let finished: BTreeSet<&str> = log.iter().filter_map(|e| if let LogEntry::Finish(id) = e { Some(id.as_str()) } else { None }).collect();
    let mut problems = Vec::new();
    let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
    for line in lines {
        let line = match line {
            Ok(line) => line,
            Err(e) => {
                problems.push(format!("results: {e}"));
                continue;
            }
        };
        *seen.entry(line.case.as_str()).or_default() += 1;
        if line.lang != lang {
            problems.push(format!("{}: lang `{}` is not `{lang}`", line.case, line.lang));
        }
        if !armed.contains(line.case.as_str()) {
            problems.push(format!("{}: reported but never armed", line.case));
        }
        if line.result != "pass" || !line.mismatches.is_empty() {
            problems.push(format!("{}: {} {}", line.case, line.result, serde_json::Value::from(line.mismatches.clone())));
        }
    }
    problems.extend(seen.iter().filter(|(_, n)| **n > 1).map(|(id, n)| format!("{id}: reported {n} times")));
    problems.extend(expected.iter().filter(|id| !seen.contains_key(id.as_str())).map(|id| format!("{id}: missing from the results")));
    problems.extend(armed.difference(&finished).map(|id| format!("{id}: armed but never finished")));
    problems
}

/// The harness's environment. ES adds the receiver's two secrets and the
/// visibly fake client credentials its token grant sends.
pub fn harness_env(tier: Tier, port: u16, results: &Path) -> Vec<(String, String)> {
    let mut env = vec![
        ("LINGARA_EMBED_SERVER_URL".to_string(), format!("http://127.0.0.1:{port}")),
        ("LINGARA_EMBED_RESULTS".to_string(), results.display().to_string()),
        ("LINGARA_EMBED_TIER".to_string(), tier.name().to_string()),
    ];
    if tier == Tier::Es {
        env.push(("LINGARA_EMBED_WEBHOOK_SECRETS".into(), sign::receiver_secrets()));
        env.push(("LINGARA_EMBED_CLIENT_ID".into(), "lgr_ci_conformance".into()));
        env.push(("LINGARA_EMBED_CLIENT_SECRET".into(), "lgr_cs_conformance-not-a-secret".into()));
    }
    env
}

/// Spawns the harness. ES harnesses must print `listening <port>` within
/// `timeout`; the port is their webhook receiver's.
pub fn spawn(tier: Tier, command: &[String], env: &[(String, String)], timeout: Duration) -> Result<(Child, Option<u16>), String> {
    let (program, args) = command.split_first().ok_or("no harness command")?;
    let mut cmd = Command::new(program);
    cmd.args(args).envs(env.iter().map(|(k, v)| (k, v)));
    if tier == Tier::Ek {
        return cmd.spawn().map(|c| (c, None)).map_err(|e| format!("cannot start `{program}`: {e}"));
    }
    let mut child = cmd.stdout(Stdio::piped()).spawn().map_err(|e| format!("cannot start `{program}`: {e}"))?;
    let stdout = child.stdout.take().ok_or("the harness has no stdout")?;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut lines = BufReader::new(stdout).lines();
        let port = lines.next().and_then(|l| l.ok()).and_then(|l| l.strip_prefix("listening ").and_then(|p| p.trim().parse::<u16>().ok()));
        let _ = tx.send(port);
        for _ in lines {} // keep draining so the harness never blocks on a full pipe
    });
    match rx.recv_timeout(timeout) {
        Ok(Some(port)) => Ok((child, Some(port))),
        _ => {
            let _ = child.kill();
            Err(format!("the es harness did not print `listening <port>` within {}s", timeout.as_secs()))
        }
    }
}

pub fn results_path(tier: Tier, lang: &str) -> PathBuf {
    std::env::temp_dir().join(format!("lingara-embed-conformance-{}-{lang}-{}.jsonl", tier.name(), std::process::id()))
}
