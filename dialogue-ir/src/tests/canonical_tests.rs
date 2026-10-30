//! The canonical form (ADR 28.10.26g D2, D4): AC2, AC3 and AC16.

use serde_json::{Number, Value, json};

use super::builder_tests::vector;
use crate::canonical::{self, format_f64};
use crate::limits::MAX_IR_BYTES;
use crate::{from_json, from_value, validate};

/// K1's vendored copy of N2's `two-keepers.json`.
fn vendored_two_keepers() -> String {
    let path = format!("{}/../spec/dialogue/examples/two-keepers.json", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("{path}: {err}"))
}

fn fixture(name: &str) -> Value {
    serde_json::from_str(&vector(name)).unwrap()
}

/// Every `{ input, canonical }` pair of one list, with `input` turned into
/// the value it denotes by `read`.
fn pairs(list: &Value, read: impl Fn(&Value) -> Value) -> Vec<(Value, String)> {
    let rows = list.as_array().expect("a list of pairs");
    assert!(!rows.is_empty());
    rows.iter().map(|row| (read(&row["input"]), row["canonical"].as_str().unwrap().to_string())).collect()
}

/// A number vector's `input` is the number's text: parsed with Rust's own
/// parser, and with serde_json's (the one a parsed IR goes through).
fn number_text(input: &Value) -> Value {
    let text = input.as_str().unwrap();
    let parsed: f64 = text.parse().unwrap();
    let via_json: Value = serde_json::from_str(text).unwrap();
    assert_eq!(canonical::to_string(&via_json), format_f64(parsed), "{text}");
    via_json
}

// 28.10.26g AC2
#[test]
fn the_example_the_corpus_and_the_normal_form() {
    // N2's example, canonicalised through the IR and as a raw value.
    let expected = vector("two-keepers.canonical.json");
    let text = vendored_two_keepers();
    assert_eq!(from_json(&text).unwrap().to_canonical_json(), expected);
    assert_eq!(canonical::to_string(&serde_json::from_str(&text).unwrap()), expected);

    let strings = fixture("canonical-strings.json");
    for (input, canonical) in pairs(&strings["strings"], Value::clone) {
        assert_eq!(canonical::to_string(&input), canonical, "{input:?}");
    }
    for (input, canonical) in pairs(&strings["keys"], Value::clone) {
        assert_eq!(canonical::to_string(&input), canonical, "{input:?}");
    }
    let numbers = fixture("canonical-numbers.json");
    for list in ["builder", "full"] {
        for (input, canonical) in pairs(&numbers[list], number_text) {
            assert_eq!(canonical::to_string(&input), canonical, "{list} {input}");
        }
    }

    // The normal form: `pedagogy`, `facts` and a checkpoint's two arrays
    // written though absent; absent optionals omitted; no `null`.
    let minimal = json!({
        "schema": "dialogue.v1", "id": "d", "npcs": [{ "id": "n", "name": "N" }], "start": "a",
        "checkpoints": [{ "id": "a", "goal": "A" }]
    });
    let written = from_value(&minimal).unwrap().to_canonical_json();
    let expected = r#"{"checkpoints":[{"edges":[],"goal":"A","id":"a","may_disclose":[]}],"facts":[],"id":"d","npcs":[{"id":"n","name":"N"}],"pedagogy":"subtle","schema":"dialogue.v1","start":"a"}"#;
    assert_eq!(written, expected);

    // No vector ends in a newline, and so neither does the writer.
    for name in ["two-keepers.canonical.json", "two-keepers.comparison.json", "missing-brother.comparison.json"] {
        assert!(!vector(name).ends_with('\n'), "{name}");
    }
}

// 28.10.26g AC3
#[test]
fn x_members_drop_out_of_the_comparison_form() {
    let canonical: Value = fixture("two-keepers.canonical.json");
    assert!(canonical.get("x-layout").is_some());
    assert_eq!(canonical::comparison_form(&canonical), vector("two-keepers.comparison.json"));
    assert_eq!(from_value(&canonical).unwrap().comparison_form(), vector("two-keepers.comparison.json"));

    // At every depth, not only the top level.
    let nested = json!({ "a": [{ "x-b": 1, "c": { "x-d": 2, "e": 3 } }], "x-f": 4 });
    assert_eq!(canonical::comparison_form(&nested), r#"{"a":[{"c":{"e":3}}]}"#);
}

/// Two-keepers padded by an `x-` member of integral `f64`s, which
/// `serde_json` writes as `1.0` and the canonical form as `1`, plus a
/// string that brings the canonical form to `MAX_IR_BYTES + extra`.
fn padded(extra: usize) -> crate::DialogueIr {
    let ones = || Value::Array(vec![Value::Number(Number::from_f64(1.0).unwrap()); 2_000]);
    let mut ir = from_json(&vendored_two_keepers()).unwrap();
    ir.extensions.insert("x-pad".into(), json!({ "n": ones(), "s": "" }));
    let short = MAX_IR_BYTES + extra - ir.to_canonical_json().len();
    ir.extensions.insert("x-pad".into(), json!({ "n": ones(), "s": "s".repeat(short) }));
    ir
}

// 28.10.26g AC16
#[test]
fn the_byte_cap_is_measured_on_the_canonical_form() {
    let at_cap = padded(0);
    assert_eq!(at_cap.to_canonical_json().len(), MAX_IR_BYTES);
    // serde_json's compact form is longer, by the `.0` of every one.
    assert!(serde_json::to_vec(&at_cap).unwrap().len() > MAX_IR_BYTES);
    assert!(validate(&at_cap).is_ok());

    let over = padded(1);
    assert_eq!(over.to_canonical_json().len(), MAX_IR_BYTES + 1);
    let err = validate(&over).unwrap_err();
    assert!(err.path.is_empty() && err.message.contains("canonical form"), "{err}");
}
