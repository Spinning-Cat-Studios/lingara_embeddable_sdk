//! Evaluation is two-valued (ADR 28.10.26f D3). A fact's value is the game's
//! last set value, else its `default`, else **unset**; a comparison over an
//! unset fact is `false`, and so is a bare unset `boolean` fact. `not`
//! inverts whatever its operand yields. Numbers compare as `f64`.

use std::collections::BTreeMap;

use super::{CmpOp, Literal, Predicate};
use crate::ir::{FactDecl, FactValue};

/// A fact's value: `set[key]`, else its declared `default`, else `None` (unset).
pub fn value<'a>(set: &'a BTreeMap<String, FactValue>, facts: &'a [FactDecl], key: &str) -> Option<&'a FactValue> {
    set.get(key).or_else(|| facts.iter().find(|decl| decl.key == key).and_then(|decl| decl.default.as_ref()))
}

/// Decides `predicate`, reading each fact through `lookup` (usually [`value`]).
/// A value of the wrong type, which a validated dialogue never holds, makes
/// its comparison `false`, as an unset one does.
pub fn eval<'a, F>(predicate: &Predicate, lookup: &F) -> bool
where
    F: Fn(&str) -> Option<&'a FactValue>,
{
    match predicate {
        Predicate::Const(b) => *b,
        Predicate::Fact(fact) => matches!(lookup(fact), Some(FactValue::Boolean(true))),
        Predicate::Compare { fact, op, value } => lookup(fact).is_some_and(|held| compare(held, *op, value)),
        Predicate::Not(inner) => !eval(inner, lookup),
        Predicate::And(terms) => terms.iter().all(|term| eval(term, lookup)),
        Predicate::Or(terms) => terms.iter().any(|term| eval(term, lookup)),
    }
}

fn compare(held: &FactValue, op: CmpOp, literal: &Literal) -> bool {
    match (held, literal) {
        (FactValue::Number(n), Literal::Number(l)) => n.as_f64().is_some_and(|n| ordered(n.partial_cmp(l), op)),
        (FactValue::String(s), Literal::String(l)) => equality(s == l, op),
        (FactValue::Boolean(b), Literal::Boolean(l)) => equality(b == l, op),
        _ => false,
    }
}

fn equality(equal: bool, op: CmpOp) -> bool {
    match op {
        CmpOp::Eq => equal,
        CmpOp::Ne => !equal,
        _ => false,
    }
}

fn ordered(ordering: Option<std::cmp::Ordering>, op: CmpOp) -> bool {
    use std::cmp::Ordering::{Equal, Greater, Less};
    let Some(ordering) = ordering else { return false };
    match op {
        CmpOp::Eq => ordering == Equal,
        CmpOp::Ne => ordering != Equal,
        CmpOp::Lt => ordering == Less,
        CmpOp::Le => ordering != Greater,
        CmpOp::Gt => ordering == Greater,
        CmpOp::Ge => ordering != Less,
    }
}
