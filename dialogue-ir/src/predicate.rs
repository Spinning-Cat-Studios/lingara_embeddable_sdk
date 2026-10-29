//! The fact-predicate language (ADR 28.10.26f D3): comparisons, `and` /
//! `or` / `not`, fact lookups; no arithmetic and no functions. The grammar is
//! `predicate.ebnf`, which this crate ships and its tests hold the parser to.
//!
//! [`parse`] reads a sentence, [`check`] also type-checks it against a
//! dialogue's declarations (what a save or a load runs), and [`eval`] decides
//! it over the game's facts. Hand-written recursive descent, no dependency.

mod eval;
mod lex;
mod parse;
mod typing;

use std::fmt;

use crate::ir::FactDecl;

pub use eval::{eval, value};

/// A comparison operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl CmpOp {
    /// The operator with its operands swapped: `0.4 <= trust` is `trust >= 0.4`.
    fn flipped(self) -> CmpOp {
        match self {
            CmpOp::Lt => CmpOp::Gt,
            CmpOp::Le => CmpOp::Ge,
            CmpOp::Gt => CmpOp::Lt,
            CmpOp::Ge => CmpOp::Le,
            same => same,
        }
    }

    /// `==` or `!=`: the two a string or boolean comparison may use.
    fn is_equality(self) -> bool {
        matches!(self, CmpOp::Eq | CmpOp::Ne)
    }
}

/// A literal a fact is compared with.
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Number(f64),
    String(String),
    Boolean(bool),
}

/// A parsed predicate. `literal cmp fact` is stored as `fact cmp literal`.
#[derive(Debug, Clone, PartialEq)]
pub enum Predicate {
    /// `true` or `false`.
    Const(bool),
    /// A bare `boolean` fact.
    Fact(String),
    Compare { fact: String, op: CmpOp, value: Literal },
    Not(Box<Predicate>),
    And(Vec<Predicate>),
    Or(Vec<Predicate>),
}

/// Why a predicate is not a sentence of the language, or not a well-typed one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredicateError {
    pub message: String,
}

impl PredicateError {
    fn new(message: impl Into<String>) -> Self {
        PredicateError { message: message.into() }
    }
}

impl fmt::Display for PredicateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for PredicateError {}

/// Reads `text` as a predicate, with no reference to any declaration.
pub fn parse(text: &str) -> Result<Predicate, PredicateError> {
    parse::parse(text)
}

/// [`parse`], then the type check against `facts`: every fact it reads is
/// declared, and each is used as its type allows. This is what "parses"
/// means in rule 4, and what a server validating a save should call.
pub fn check(text: &str, facts: &[FactDecl]) -> Result<Predicate, PredicateError> {
    let predicate = parse::parse(text)?;
    typing::check(&predicate, facts)?;
    Ok(predicate)
}
