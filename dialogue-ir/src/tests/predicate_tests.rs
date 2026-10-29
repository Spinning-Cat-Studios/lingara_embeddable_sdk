//! The fact-predicate language (ADR 28.10.26f D3): AC3, AC4, AC5 and AC21.

use std::collections::BTreeMap;

use serde_json::{Number, json};

use crate::predicate::{CmpOp, Literal, Predicate, check, eval, parse, value};
use crate::{FactDecl, FactType, FactValue, from_value, validate};

fn fact(name: &str) -> Predicate {
    Predicate::Fact(name.into())
}

fn compare(fact: &str, op: CmpOp, value: Literal) -> Predicate {
    Predicate::Compare { fact: fact.into(), op, value }
}

/// Every form D3 allows: each operator, a literal on either side, strings,
/// booleans, constants.
fn forms() -> Vec<(&'static str, Predicate)> {
    vec![
        ("t == 1", compare("t", CmpOp::Eq, Literal::Number(1.0))),
        ("t != -2.5", compare("t", CmpOp::Ne, Literal::Number(-2.5))),
        ("t < 3", compare("t", CmpOp::Lt, Literal::Number(3.0))),
        ("t <= 3", compare("t", CmpOp::Le, Literal::Number(3.0))),
        ("t > 3", compare("t", CmpOp::Gt, Literal::Number(3.0))),
        ("0.4 <= t", compare("t", CmpOp::Ge, Literal::Number(0.4))),
        ("s == \"a \\\"b\\\" \\\\\"", compare("s", CmpOp::Eq, Literal::String("a \"b\" \\".into()))),
        ("true == b", compare("b", CmpOp::Eq, Literal::Boolean(true))),
        ("b != false", compare("b", CmpOp::Ne, Literal::Boolean(false))),
        ("true", Predicate::Const(true)),
        ("\tfalse ", Predicate::Const(false)),
    ]
}

#[test]
fn the_grammar_and_its_refusals() {
    // Precedence: `not` over `and` over `or`.
    let expected = Predicate::Or(vec![
        fact("a"),
        Predicate::And(vec![compare("b", CmpOp::Ge, Literal::Number(0.4)), Predicate::Not(Box::new(fact("c")))]),
    ]);
    assert_eq!(parse("a or b >= 0.4 and not c").unwrap(), expected);
    assert_eq!(
        parse("(a or b) and c").unwrap(),
        Predicate::And(vec![Predicate::Or(vec![fact("a"), fact("b")]), fact("c")])
    );
    assert_eq!(parse("not a and b").unwrap(), Predicate::And(vec![Predicate::Not(Box::new(fact("a"))), fact("b")]));

    // Every form: each operator, a literal on either side, strings, booleans, constants.
    for (text, expected) in forms() {
        assert_eq!(parse(text).unwrap(), expected, "{text}");
    }

    // Eight parentheses open at once is the deepest a predicate goes.
    assert!(parse(&format!("{}a{}", "(".repeat(8), ")".repeat(8))).is_ok());
    assert!(parse(&format!("{}a{}", "(".repeat(9), ")".repeat(9))).unwrap_err().message.contains("deeper"));

    // 200 characters is the longest; 201 is refused.
    let long = |n: usize| format!("{} == 1", "t".repeat(n - 5));
    assert!(parse(&long(200)).is_ok());
    assert!(parse(&long(201)).unwrap_err().message.contains("201 characters"));

    let refusals = [
        ("t + 1 > 2", "arithmetic"),
        ("t - 1 > 2", "arithmetic"),
        ("has_item(\"lantern\")", "function call"),
        ("t >= other", "compared with the fact"),
        ("t >= 1e3", "exponent"),
        ("s == \"open", "not terminated"),
        ("s == \"a\\nb\"", "escapes"),
        ("t >= 1.", "digit after"),
    ];
    for (text, reason) in refusals {
        let err = parse(text).unwrap_err();
        assert!(err.message.contains(reason), "{text}: {err}");
    }
}

fn decls() -> Vec<FactDecl> {
    let decl = |key: &str, kind| FactDecl { key: key.into(), kind, default: None };
    vec![
        decl("trust", FactType::Number),
        decl("gate", FactType::String),
        decl("alarm", FactType::Boolean),
        decl("not", FactType::Boolean),
    ]
}

#[test]
fn predicates_are_type_checked_against_declarations() {
    let facts = decls();
    for ok in ["trust >= 0.4", "gate == \"east\"", "alarm", "alarm != true", "not alarm or trust < 1"] {
        assert!(check(ok, &facts).is_ok(), "{ok}");
    }
    let refusals = [
        ("trst >= 0.4", "unknown fact \"trst\""),
        ("gate < \"m\"", "only with == and !="),
        ("trust == \"high\"", "compared with a string"),
        ("trust", "only a boolean fact stands alone"),
        ("not == true", "expected a fact"),
    ];
    for (text, reason) in refusals {
        let err = check(text, &facts).unwrap_err();
        assert!(err.message.contains(reason), "{text}: {err}");
    }

    // At load, the refusal carries the predicate's path.
    for text in ["trst >= 0.4", "gate < \"m\"", "trust == \"high\"", "trust", "not"] {
        let value = json!({
            "schema": "dialogue.v1", "id": "typed", "npcs": [{ "id": "npc", "name": "Npc" }],
            "facts": [{ "key": "trust", "type": "number" }, { "key": "gate", "type": "string" },
                      { "key": "not", "type": "boolean" }],
            "start": "a",
            "checkpoints": [{ "id": "a", "goal": "Talk", "edges": [{ "id": "go", "to": "success", "predicate": text }] }]
        });
        let err = validate(&from_value(&value).unwrap()).unwrap_err();
        assert_eq!(err.path, "checkpoints[0].edges[0].predicate", "{text}");
    }
}

#[test]
fn values_defaults_and_unset_facts() {
    let mut facts = decls();
    facts[0].default = Some(FactValue::Number(Number::from(0)));
    let mut set = BTreeMap::new();
    let holds = |text: &str, set: &BTreeMap<String, FactValue>| eval(&check(text, &facts).unwrap(), &|key| value(set, &facts, key));

    // The default, until the game sets a value.
    assert!(holds("trust == 0", &set));
    set.insert("trust".to_string(), FactValue::Number(Number::from_f64(0.5).unwrap()));
    assert!(holds("trust >= 0.4", &set) && !holds("trust < 0.4", &set));

    // Unset (no value, no default): every comparison and a bare boolean are
    // false, and `not` of each is true.
    for text in ["gate == \"east\"", "gate != \"east\"", "alarm", "alarm == true", "alarm != true", "alarm == false"] {
        assert!(!holds(text, &set), "{text}");
        assert!(holds(&format!("not ({text})"), &set), "not {text}");
    }

    set.insert("gate".to_string(), FactValue::String("east".into()));
    set.insert("alarm".to_string(), FactValue::Boolean(true));
    assert!(holds("gate == \"east\" and alarm and alarm != false", &set));
    assert!(!holds("gate != \"east\" or not alarm", &set));
}

#[test]
fn the_grammar_file_is_the_documented_one() {
    let file = include_str!("../../predicate.ebnf");
    let section = |marker: &str| -> Vec<&str> {
        let start = file.find(marker).unwrap_or_else(|| panic!("no {marker}")) + marker.len();
        file[start..].lines().take_while(|line| !line.starts_with('#')).filter(|line| !line.is_empty()).collect()
    };
    let accepts = section("\n# accepts\n");
    let refuses = section("\n# refuses\n");
    assert!(accepts.len() >= 10 && refuses.len() >= 10);
    for line in accepts {
        assert!(parse(line).is_ok(), "accepts: {line}: {:?}", parse(line));
    }
    for line in refuses {
        assert!(parse(line).is_err(), "refuses: {line}");
    }
    // The grammar block is D3's, verbatim.
    assert!(file.contains("comparison = fact cmp literal | literal cmp fact\n"));
    assert!(file.contains("number     = [ \"-\" ] digit { digit } [ \".\" digit { digit } ]\n"));
}
