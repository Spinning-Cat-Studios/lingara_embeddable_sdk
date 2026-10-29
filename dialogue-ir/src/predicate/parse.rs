//! Recursive descent over `predicate.ebnf`: `not` binds tighter than `and`,
//! which binds tighter than `or`. A primary is decided by its first token,
//! plus one more after a fact or `true` / `false`: a comparison operator
//! there makes it a comparison.

use super::lex::{Token, lex};
use super::{Literal, Predicate, PredicateError};
use crate::limits::{MAX_PREDICATE_CHARS, PREDICATE_MAX_DEPTH};

pub(super) fn parse(text: &str) -> Result<Predicate, PredicateError> {
    let n = text.chars().count();
    if n > MAX_PREDICATE_CHARS {
        return Err(PredicateError::new(format!("is {n} characters; at most {MAX_PREDICATE_CHARS}")));
    }
    let mut parser = Parser { tokens: lex(text)?, at: 0, depth: 0 };
    let predicate = parser.or_expr()?;
    match parser.peek() {
        None => Ok(predicate),
        Some(token) => Err(PredicateError::new(format!("unexpected {} after a complete predicate", describe(token)))),
    }
}

struct Parser {
    tokens: Vec<Token>,
    at: usize,
    /// Open parentheses.
    depth: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.at)
    }

    fn next(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.at).cloned();
        self.at += usize::from(token.is_some());
        token
    }

    fn eat(&mut self, token: &Token) -> bool {
        let found = self.peek() == Some(token);
        self.at += usize::from(found);
        found
    }

    fn or_expr(&mut self) -> Result<Predicate, PredicateError> {
        let mut terms = vec![self.and_expr()?];
        while self.eat(&Token::Or) {
            terms.push(self.and_expr()?);
        }
        Ok(if terms.len() == 1 { terms.remove(0) } else { Predicate::Or(terms) })
    }

    fn and_expr(&mut self) -> Result<Predicate, PredicateError> {
        let mut terms = vec![self.unary()?];
        while self.eat(&Token::And) {
            terms.push(self.unary()?);
        }
        Ok(if terms.len() == 1 { terms.remove(0) } else { Predicate::And(terms) })
    }

    fn unary(&mut self) -> Result<Predicate, PredicateError> {
        if self.eat(&Token::Not) { Ok(Predicate::Not(Box::new(self.unary()?))) } else { self.primary() }
    }

    fn primary(&mut self) -> Result<Predicate, PredicateError> {
        match self.next() {
            Some(Token::LParen) => self.group(),
            Some(Token::Ident(fact)) => self.after_fact(fact),
            Some(Token::True) => self.after_bool(true),
            Some(Token::False) => self.after_bool(false),
            Some(Token::Number(n)) => self.literal_first(Literal::Number(n)),
            Some(Token::Str(s)) => self.literal_first(Literal::String(s)),
            Some(token) => Err(PredicateError::new(format!("expected a fact, a literal, `not` or `(`; found {}", describe(&token)))),
            None => Err(PredicateError::new("ends where a fact, a literal, `not` or `(` was expected")),
        }
    }

    fn group(&mut self) -> Result<Predicate, PredicateError> {
        self.depth += 1;
        if self.depth > PREDICATE_MAX_DEPTH {
            return Err(PredicateError::new(format!("nests parentheses deeper than {PREDICATE_MAX_DEPTH}")));
        }
        let inner = self.or_expr()?;
        if !self.eat(&Token::RParen) {
            return Err(PredicateError::new("a `(` is not closed"));
        }
        self.depth -= 1;
        Ok(inner)
    }

    /// `fact cmp literal`, or a bare fact.
    fn after_fact(&mut self, fact: String) -> Result<Predicate, PredicateError> {
        match self.peek() {
            Some(Token::Cmp(op)) => {
                let op = *op;
                self.at += 1;
                let value = self.literal(&fact)?;
                Ok(Predicate::Compare { fact, op, value })
            }
            Some(Token::LParen) => Err(PredicateError::new(format!("`{fact}(…)`: a function call is not in the predicate language"))),
            _ => Ok(Predicate::Fact(fact)),
        }
    }

    /// `true` or `false`: a comparison's left side if a `cmp` follows, else bare.
    fn after_bool(&mut self, b: bool) -> Result<Predicate, PredicateError> {
        if matches!(self.peek(), Some(Token::Cmp(_))) { self.literal_first(Literal::Boolean(b)) } else { Ok(Predicate::Const(b)) }
    }

    /// `literal cmp fact`, stored as `fact cmp' literal`.
    fn literal_first(&mut self, value: Literal) -> Result<Predicate, PredicateError> {
        let Some(Token::Cmp(op)) = self.next() else {
            return Err(PredicateError::new("a literal must be compared with a fact"));
        };
        match self.next() {
            Some(Token::Ident(fact)) => Ok(Predicate::Compare { fact, op: op.flipped(), value }),
            _ => Err(PredicateError::new("a literal must be compared with a fact")),
        }
    }

    /// The literal after `fact cmp`.
    fn literal(&mut self, fact: &str) -> Result<Literal, PredicateError> {
        match self.next() {
            Some(Token::Number(n)) => Ok(Literal::Number(n)),
            Some(Token::Str(s)) => Ok(Literal::String(s)),
            Some(Token::True) => Ok(Literal::Boolean(true)),
            Some(Token::False) => Ok(Literal::Boolean(false)),
            Some(Token::Ident(other)) => Err(PredicateError::new(format!("`{fact}` is compared with the fact `{other}`; a fact compares only with a literal"))),
            _ => Err(PredicateError::new(format!("`{fact}` is compared with nothing; a literal must follow the operator"))),
        }
    }
}

fn describe(token: &Token) -> String {
    match token {
        Token::Ident(name) => format!("the fact `{name}`"),
        Token::Number(n) => format!("the number {n}"),
        Token::Str(s) => format!("the string \"{s}\""),
        Token::LParen => "`(`".into(),
        Token::RParen => "`)`".into(),
        Token::Cmp(_) => "a comparison operator".into(),
        Token::And => "`and`".into(),
        Token::Or => "`or`".into(),
        Token::Not => "`not`".into(),
        Token::True => "`true`".into(),
        Token::False => "`false`".into(),
    }
}
