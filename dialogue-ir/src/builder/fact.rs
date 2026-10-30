//! A fact and its `default`, checked against D2's builder ranges.

use serde_json::Number;

use crate::IrError;
use crate::ir::{FactType, FactValue};

/// The largest integer magnitude a builder writes (D2 rule 3): 2^53 − 1.
pub const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

/// A fact the game tracks.
pub struct Fact {
    key: String,
    kind: FactType,
    default: Option<FactDefault>,
}

impl Fact {
    pub fn number(key: &str) -> Self {
        Fact { key: key.into(), kind: FactType::Number, default: None }
    }

    pub fn boolean(key: &str) -> Self {
        Fact { key: key.into(), kind: FactType::Boolean, default: None }
    }

    pub fn string(key: &str) -> Self {
        Fact { key: key.into(), kind: FactType::String, default: None }
    }

    /// The value the fact holds until the game sets one. A value of another
    /// type is `validate`'s rule 5 refusal at `build()`.
    pub fn default(mut self, value: impl Into<FactDefault>) -> Self {
        self.default = Some(value.into());
        self
    }

    pub(super) fn into_parts(self) -> (String, FactType, Option<FactDefault>) {
        (self.key, self.kind, self.default)
    }
}

/// A fact's `default` as the author wrote it; checked against D2's builder
/// ranges at `build()`.
#[derive(Debug, Clone)]
pub enum FactDefault {
    Number(f64),
    Boolean(bool),
    String(String),
}

impl From<f64> for FactDefault {
    fn from(value: f64) -> Self {
        FactDefault::Number(value)
    }
}

impl From<i32> for FactDefault {
    fn from(value: i32) -> Self {
        FactDefault::Number(value.into())
    }
}

impl From<bool> for FactDefault {
    fn from(value: bool) -> Self {
        FactDefault::Boolean(value)
    }
}

impl From<&str> for FactDefault {
    fn from(value: &str) -> Self {
        FactDefault::String(value.into())
    }
}

impl FactDefault {
    pub(super) fn into_value(self, path: &str) -> Result<FactValue, IrError> {
        match self {
            FactDefault::Boolean(b) => Ok(FactValue::Boolean(b)),
            FactDefault::String(s) => Ok(FactValue::String(s)),
            FactDefault::Number(x) => builder_number(x).map(FactValue::Number).ok_or_else(|| {
                IrError::new(path, format!("{x} is outside the builder's range; set it at runtime with DialogueSetFact"))
            }),
        }
    }
}

/// D2 rules 3 and 4's builder range: an integer of magnitude at most
/// 2^53 − 1, or a non-integral number in `1e-4 ≤ |x| < 1e9` with at most
/// four decimal places. An integer is held as one, so it writes as `0`.
pub fn builder_number(x: f64) -> Option<Number> {
    if !x.is_finite() {
        return None;
    }
    if x.fract() == 0.0 {
        return (x.abs() <= MAX_SAFE_INTEGER).then(|| Number::from(x as i64));
    }
    let four_places = (x * 1e4).round() / 1e4 == x;
    ((1e-4..1e9).contains(&x.abs()) && four_places).then(|| Number::from_f64(x)).flatten()
}
