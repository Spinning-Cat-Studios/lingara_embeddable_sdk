use std::time::Duration;

use crate::case::Tier;
use crate::run::{check_only, parse_results, spawn, verify};
use crate::state::LogEntry;

fn line(case: &str, result: &str) -> String {
    format!(r#"{{"case":"{case}","lang":"go","result":"{result}"}}"#)
}

fn sh(script: &str) -> Vec<String> {
    vec!["sh".into(), "-c".into(), script.into()]
}

// 1.10.26x AC8
#[test]
fn results_must_match_the_arm_log() {
    let expected = vec!["es1.a".to_string(), "es1.b".to_string()];
    let log = vec![
        LogEntry::Arm("es1.a".into()),
        LogEntry::Finish("es1.a".into()),
        LogEntry::Arm("es1.b".into()),
        LogEntry::Finish("es1.b".into()),
    ];
    let good = format!("{}\n{}\n", line("es1.a", "pass"), line("es1.b", "pass"));
    assert_eq!(verify("go", &parse_results(&good), &expected, &log), Vec::<String>::new());

    let has = |text: &str, needle: &str| {
        let found = verify("go", &parse_results(text), &expected, &log);
        assert!(found.iter().any(|p| p.contains(needle)), "no `{needle}` in {found:?}");
    };
    // An armed case the results omit.
    has(&line("es1.a", "pass"), "es1.b: missing from the results");
    // A case reported that was never armed.
    has(&format!("{good}{}\n", line("es1.c", "pass")), "es1.c: reported but never armed");
    // Anything but pass, `skip` included.
    has(&format!("{}\n{}\n", line("es1.a", "pass"), line("es1.b", "skip")), "es1.b: skip");
    has(&format!("{}\n{}\n", line("es1.a", "pass"), line("es1.b", "fail")), "es1.b: fail");

    // An ES harness that never prints `listening <port>`.
    let silent = spawn(Tier::Es, &sh("sleep 2"), &[], Duration::from_millis(300));
    assert!(silent.is_err_and(|e| e.contains("did not print `listening <port>`")));
    let (mut child, port) = spawn(Tier::Es, &sh("echo 'listening 4242'"), &[], Duration::from_secs(5)).expect("prints");
    assert_eq!(port, Some(4242));
    let _ = child.wait();

    // --only is refused under CI, and allowed locally.
    assert!(check_only(Some("es1.a"), Some("true")).is_err_and(|e| e.contains("refused when CI is set")));
    assert_eq!(check_only(Some("es1.a, es1.b"), None).ok().flatten().map(|s| s.len()), Some(2));
    assert_eq!(check_only(None, Some("true")), Ok(None));
}
