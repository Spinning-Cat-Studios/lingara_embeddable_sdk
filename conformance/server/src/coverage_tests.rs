use crate::case::Tier;
use crate::coverage::{check_coverage, check_spec};
use crate::test_support::{frozen, tree};

const OPS: &str = "operations = [\"createEmbedToken\"]\nmessages = [\"practice.completed\"]\n";
const BEHAVIOURS: &str = "[[behaviour]]\nid = \"ES1\"\ntitle = \"mints\"\n\n[[behaviour]]\nid = \"ES4\"\ntitle = \"the snippet\"\nvectors = \"snippet.json\"\n";
const CASE: &str = "id: es1.mint\nbehaviours: [ES1]\ntier: es\nserve:\n  - route: { operation: createEmbedToken }\n    respond: { json: {} }\ndeliver:\n  - message: practice.completed\n    sign: { secrets: [primary] }\n    expect: { status: 204, handler: not_called }\n";

fn problems(files: &[(&str, &str)], langs: &[&str]) -> Vec<String> {
    let dir = tree(files);
    let langs: Vec<String> = langs.iter().map(|s| s.to_string()).collect();
    check_coverage(Tier::Es, dir.path(), &frozen(), &langs).expect("reads").problems
}

fn has(found: &[String], needle: &str) -> bool {
    found.iter().any(|p| p.contains(needle))
}

fn full<'a>(extra: &[(&'a str, &'a str)]) -> Vec<(&'a str, &'a str)> {
    let mut files = vec![
        ("es/operations.toml", OPS),
        ("es/behaviours.toml", BEHAVIOURS),
        ("es/cases/es1/mint.yaml", CASE),
        ("vectors/snippet.json", "{}"),
    ];
    for (k, v) in extra {
        files.retain(|(p, _)| p != k);
        files.push((k, v));
    }
    files
}

// 1.10.26x AC9
#[test]
fn coverage_fails_every_gap() {
    // A complete tier: clean.
    assert_eq!(problems(&full(&[]), &["go"]), Vec::<String>::new());

    // A behaviour with no case and no vectors.
    let lonely = format!("{BEHAVIOURS}\n[[behaviour]]\nid = \"ES2\"\ntitle = \"routes\"\n");
    assert!(has(&problems(&full(&[("es/behaviours.toml", &lonely)]), &[]), "ES2 (routes): no case covers"));
    // A vectors file missing, or not JSON.
    assert!(has(&problems(&full(&[("vectors/snippet.json", "not json")]), &[]), "is not JSON"));
    let no_vectors: Vec<_> = full(&[]).into_iter().filter(|(p, _)| *p != "vectors/snippet.json").collect();
    assert!(has(&problems(&no_vectors, &[]), "vectors snippet.json"));
    // A case naming a behaviour, or an operation, outside its tier's lists.
    let stranger = CASE.replace("[ES1]", "[ES9]").replace("operation: createEmbedToken", "operation: sendDialogueTurn");
    let found = problems(&full(&[("es/cases/es1/mint.yaml", &stranger)]), &[]);
    assert!(has(&found, "behaviour ES9 is not in behaviours.toml") && has(&found, "operation sendDialogueTurn is not in operations.toml"), "{found:?}");
    // An operation no case exercises, in a tier with behaviours; a deliver's message counts.
    let wider = "operations = [\"createEmbedToken\", \"deleteEmbedPlayer\"]\nmessages = [\"practice.completed\"]\n";
    let found = problems(&full(&[("es/operations.toml", wider)]), &[]);
    assert!(has(&found, "deleteEmbedPlayer: no case exercises it") && !has(&found, "practice.completed: no case"), "{found:?}");
    // An operation missing from the frozen current.
    assert!(has(&problems(&full(&[("es/operations.toml", "operations = [\"linkEmbedPlayer\"]\n")]), &[]), "linkEmbedPlayer is not in the frozen current"));
    // A case whose id does not match its path.
    assert!(has(&problems(&full(&[("es/cases/es1/mint.yaml", &CASE.replace("es1.mint", "es1.other"))]), &[]), "does not match its path"));

    // An empty behaviours.toml beside a non-empty --langs fails...
    let scaffold = [("es/operations.toml", OPS), ("es/behaviours.toml", "")];
    assert!(has(&problems(&scaffold, &["go"]), "behaviours.toml is empty, but languages have joined: go"));
    // ...and with no languages it exits 0 and says so, yet still holds operations to the current.
    let dir = tree(&scaffold);
    let report = check_coverage(Tier::Es, dir.path(), &frozen(), &[]).expect("reads");
    assert_eq!((report.problems.clone(), report.notes.clone()), (vec![], vec!["tier es: no behaviours yet (V1)".to_string()]));
    let stale = tree(&[("es/operations.toml", "operations = [\"linkEmbedPlayer\"]\n"), ("es/behaviours.toml", "")]);
    assert!(has(&check_coverage(Tier::Es, stale.path(), &frozen(), &[]).expect("reads").problems, "linkEmbedPlayer"));
    assert!(has(&check_spec(&stale.path().join("es"), &frozen()).expect("reads"), "linkEmbedPlayer is not in the frozen current"));
}
