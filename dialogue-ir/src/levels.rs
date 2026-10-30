//! Levels as the builders take them (ADR 28.10.26g D3): the IR stores
//! `speaking` and `comprehension` as `1..=9`, and a label is accepted only
//! where it maps offline. HSK, TOPIK and `General` labels *are* their
//! number, so they have no helper.
//!
//! A helper does not check the language: `Level::cefr("A2")` on `zh` stores
//! 2. What a level does on the turn is N1's: under `subtle` the speech level
//! is `min(NPC speaking, learner level)`, under `none` the NPC's own.

use crate::IrError;

/// A level on the IR's `1..=9` scale. `build()` refuses one outside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Level(pub u8);

/// CEFR's six labels, in order: `A1` is 1.
const CEFR: [&str; 6] = ["A1", "A2", "B1", "B2", "C1", "C2"];
/// JLPT's five, from the easiest: `N5` is 1.
const JLPT: [&str; 5] = ["N5", "N4", "N3", "N2", "N1"];

impl Level {
    /// The top of the scale. N1 treats an NPC with no profile as this.
    pub const NATIVE: Level = Level(9);

    /// `A1`–`C2` as 1–6.
    pub fn cefr(label: &str) -> Result<Level, IrError> {
        position(&CEFR, label, "CEFR")
    }

    /// `N5`–`N1` as 1–5: reversed, which is the mistake this prevents.
    pub fn jlpt(label: &str) -> Result<Level, IrError> {
        position(&JLPT, label, "JLPT")
    }
}

impl From<u8> for Level {
    fn from(value: u8) -> Self {
        Level(value)
    }
}

fn position(labels: &[&str], label: &str, scale: &str) -> Result<Level, IrError> {
    match labels.iter().position(|l| *l == label) {
        Some(i) => Ok(Level(i as u8 + 1)),
        None => Err(IrError::new("", format!("\"{label}\" is not a {scale} level ({})", labels.join(", ")))),
    }
}
