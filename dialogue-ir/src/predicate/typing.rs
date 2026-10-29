//! The type check against a dialogue's declarations (ADR 28.10.26f D3). A
//! `number` fact compares with a number under all six operators; a `string`
//! fact with a string, and a `boolean` fact with `true` / `false`, under `==`
//! and `!=` only; a `boolean` fact may also stand bare. Every fact read must
//! be declared.

use super::{Literal, Predicate, PredicateError};
use crate::ir::{FactDecl, FactType};

pub(super) fn check(predicate: &Predicate, facts: &[FactDecl]) -> Result<(), PredicateError> {
    match predicate {
        Predicate::Const(_) => Ok(()),
        Predicate::Fact(fact) => match declared(fact, facts)? {
            FactType::Boolean => Ok(()),
            other => Err(PredicateError::new(format!("`{fact}` is a {} fact; only a boolean fact stands alone", name(other)))),
        },
        Predicate::Compare { fact, op, value } => {
            let kind = declared(fact, facts)?;
            if kind != literal_type(value) {
                let message = format!("`{fact}` is a {} fact, compared with a {}", name(kind), name(literal_type(value)));
                return Err(PredicateError::new(message));
            }
            if kind != FactType::Number && !op.is_equality() {
                return Err(PredicateError::new(format!("`{fact}` is a {} fact, which compares only with == and !=", name(kind))));
            }
            Ok(())
        }
        Predicate::Not(inner) => check(inner, facts),
        Predicate::And(terms) | Predicate::Or(terms) => terms.iter().try_for_each(|term| check(term, facts)),
    }
}

fn declared(fact: &str, facts: &[FactDecl]) -> Result<FactType, PredicateError> {
    match facts.iter().find(|decl| decl.key == fact) {
        Some(decl) => Ok(decl.kind),
        None => Err(PredicateError::new(format!("unknown fact \"{fact}\""))),
    }
}

fn literal_type(literal: &Literal) -> FactType {
    match literal {
        Literal::Number(_) => FactType::Number,
        Literal::String(_) => FactType::String,
        Literal::Boolean(_) => FactType::Boolean,
    }
}

fn name(kind: FactType) -> &'static str {
    match kind {
        FactType::String => "string",
        FactType::Number => "number",
        FactType::Boolean => "boolean",
    }
}
