//! The IR's canonical form (ADR 28.10.26g D2): RFC 8785 (JCS) over the
//! normal form. Written by hand rather than through `serde_json`'s writer,
//! whose key order depends on a feature any crate in the graph may switch on
//! (`preserve_order`) and whose numbers keep their fraction (`2.0`).
//!
//! This writer covers the whole of JCS, because it also canonicalises what a
//! parsed file holds: a `default` as written, and anything in an `x-` member.
//! The other kits' writers implement only their builders' range.

use serde_json::{Map, Number, Value};

use crate::ir::DialogueIr;

impl DialogueIr {
    /// The dialogue's canonical text: JCS over its normal form, no
    /// whitespace, no trailing newline, `x-` members kept.
    pub fn to_canonical_json(&self) -> String {
        to_string(&normal_form(self))
    }

    /// The canonical text with every `x-` member removed at every depth: two
    /// IRs are the same dialogue when this is byte-equal (D2).
    pub fn comparison_form(&self) -> String {
        comparison_form(&normal_form(self))
    }
}

/// The IR as a JSON value in normal form (`ir.rs` serialises it so).
fn normal_form(ir: &DialogueIr) -> Value {
    // Serialising the IR's own types cannot fail: every map key is a string
    // and every number is a `Number`.
    serde_json::to_value(ir).unwrap_or(Value::Null)
}

/// Any JSON value in canonical form.
pub fn to_string(value: &Value) -> String {
    let mut out = String::new();
    write_value(&mut out, value);
    out
}

/// [`to_string`] after removing every member whose name starts `x-`, at
/// every depth.
pub fn comparison_form(value: &Value) -> String {
    to_string(&strip_extensions(value))
}

fn strip_extensions(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter().filter(|(key, _)| !key.starts_with("x-")).map(|(k, v)| (k.clone(), strip_extensions(v))).collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(strip_extensions).collect()),
        other => other.clone(),
    }
}

fn write_value(out: &mut String, value: &Value) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&number(n)),
        Value::String(s) => write_string(out, s),
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_value(out, item);
            }
            out.push(']');
        }
        Value::Object(map) => write_object(out, map),
    }
}

/// Members sorted by UTF-16 code unit (JCS), which is byte order for the
/// ASCII keys outside an `x-` member and differs from it above U+FFFF.
fn write_object(out: &mut String, map: &Map<String, Value>) {
    let mut members: Vec<(&String, &Value)> = map.iter().collect();
    members.sort_by(|(a, _), (b, _)| a.encode_utf16().cmp(b.encode_utf16()));
    out.push('{');
    for (i, (key, value)) in members.into_iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        write_string(out, key);
        out.push(':');
        write_value(out, value);
    }
    out.push('}');
}

/// D2 rule 5: only `"`, `\` and U+0000–U+001F are escaped, the controls with
/// a short form where JSON has one and `\u00xx` in lowercase hex otherwise.
pub fn write_string(out: &mut String, text: &str) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{c}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// D2 rules 3 and 4: a number as JCS writes it, ECMAScript's
/// `Number.prototype.toString` over the `f64` the text denotes.
pub fn number(n: &Number) -> String {
    // An i64 or u64 beyond 2^53 becomes the nearest f64, as JCS requires.
    format_f64(n.as_f64().unwrap_or(0.0))
}

/// ECMAScript's `Number::toString` for a finite `f64`, from the shortest
/// round-trip digits Rust's `{:e}` gives.
pub fn format_f64(x: f64) -> String {
    if x == 0.0 {
        return "0".into(); // `-0` included
    }
    if x < 0.0 {
        return format!("-{}", format_f64(-x));
    }
    let sci = format!("{x:e}");
    let (mantissa, exponent) = sci.split_once('e').unwrap_or((&sci, "0"));
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let k = digits.len() as i32;
    // ECMAScript's n: the value is 0.digits × 10^n.
    let n = exponent.parse::<i32>().unwrap_or(0) + 1;
    if k <= n && n <= 21 {
        format!("{digits}{}", "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        format!("{}.{}", &digits[..n as usize], &digits[n as usize..])
    } else if -6 < n && n <= 0 {
        format!("0.{}{digits}", "0".repeat((-n) as usize))
    } else {
        exponential(&digits, n)
    }
}

/// ECMAScript's exponent form: `d[.ddd]e±x`, the exponent `n - 1`.
fn exponential(digits: &str, n: i32) -> String {
    let sign = if n - 1 < 0 { '-' } else { '+' };
    let (first, rest) = digits.split_at(1);
    let fraction = if rest.is_empty() { String::new() } else { format!(".{rest}") };
    format!("{first}{fraction}e{sign}{}", (n - 1).abs())
}
