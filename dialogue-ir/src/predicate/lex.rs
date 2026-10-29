//! The tokeniser. Space and tab separate tokens; anything else outside a
//! token is refused by name, so `trust + 1` says arithmetic is not in the
//! language rather than "unexpected character".

use super::{CmpOp, PredicateError};

#[derive(Debug, Clone, PartialEq)]
pub(super) enum Token {
    Ident(String),
    Number(f64),
    Str(String),
    LParen,
    RParen,
    Cmp(CmpOp),
    And,
    Or,
    Not,
    True,
    False,
}

/// Longest first, so `<=` is never read as `<` then `=`.
const OPERATORS: [(&str, CmpOp); 6] =
    [("==", CmpOp::Eq), ("!=", CmpOp::Ne), ("<=", CmpOp::Le), (">=", CmpOp::Ge), ("<", CmpOp::Lt), (">", CmpOp::Gt)];

pub(super) fn lex(text: &str) -> Result<Vec<Token>, PredicateError> {
    let mut tokens = Vec::new();
    let mut rest = text.trim_start_matches([' ', '\t']);
    while let Some(c) = rest.chars().next() {
        let (token, len) = token(rest, c)?;
        tokens.push(token);
        rest = rest[len..].trim_start_matches([' ', '\t']);
    }
    Ok(tokens)
}

/// The token `rest` starts with, and its length in bytes.
fn token(rest: &str, c: char) -> Result<(Token, usize), PredicateError> {
    match c {
        'a'..='z' => Ok(word(rest)),
        '0'..='9' | '-' => number(rest),
        '"' => string(rest),
        '(' => Ok((Token::LParen, 1)),
        ')' => Ok((Token::RParen, 1)),
        '+' | '*' | '/' | '%' => Err(PredicateError::new(format!("arithmetic (`{c}`) is not in the predicate language"))),
        _ => operator(rest, c),
    }
}

/// A keyword or a fact name: the IR's id alphabet.
fn word(rest: &str) -> (Token, usize) {
    let len = rest.find(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')).unwrap_or(rest.len());
    let token = match &rest[..len] {
        "and" => Token::And,
        "or" => Token::Or,
        "not" => Token::Not,
        "true" => Token::True,
        "false" => Token::False,
        name => Token::Ident(name.to_string()),
    };
    (token, len)
}

/// `[-] digit {digit} [. digit {digit}]`, with nothing glued on after it.
fn number(rest: &str) -> Result<(Token, usize), PredicateError> {
    let digits = |from: usize| rest[from..].find(|c: char| !c.is_ascii_digit()).map_or(rest.len(), |n| from + n);
    let sign = usize::from(rest.starts_with('-'));
    let mut end = digits(sign);
    if end == sign {
        return Err(PredicateError::new("arithmetic (`-`) is not in the predicate language"));
    }
    if rest[end..].starts_with('.') {
        let fraction = digits(end + 1);
        if fraction == end + 1 {
            return Err(PredicateError::new(format!("`{}` needs a digit after its `.`", &rest[..=end])));
        }
        end = fraction;
    }
    match rest[end..].chars().next() {
        Some('e' | 'E') => Err(PredicateError::new(format!("`{}…` has an exponent, which a number may not", &rest[..end]))),
        Some(c) if c.is_alphanumeric() || c == '_' || c == '.' => {
            Err(PredicateError::new(format!("`{}{c}` is not a number", &rest[..end])))
        }
        _ => Ok((Token::Number(rest[..end].parse().unwrap_or_default()), end)),
    }
}

/// A double-quoted string whose only escapes are `\"` and `\\`.
fn string(rest: &str) -> Result<(Token, usize), PredicateError> {
    let mut value = String::new();
    let mut chars = rest.char_indices().skip(1);
    while let Some((at, c)) = chars.next() {
        match c {
            '"' => return Ok((Token::Str(value), at + 1)),
            '\\' => match chars.next() {
                Some((_, escaped @ ('"' | '\\'))) => value.push(escaped),
                _ => return Err(PredicateError::new("a string's only escapes are \\\" and \\\\")),
            },
            _ => value.push(c),
        }
    }
    Err(PredicateError::new("a string is not terminated"))
}

fn operator(rest: &str, c: char) -> Result<(Token, usize), PredicateError> {
    match OPERATORS.iter().find(|(spelling, _)| rest.starts_with(spelling)) {
        Some((spelling, op)) => Ok((Token::Cmp(*op), spelling.len())),
        None => Err(PredicateError::new(format!("`{c}` is not in the predicate language"))),
    }
}
