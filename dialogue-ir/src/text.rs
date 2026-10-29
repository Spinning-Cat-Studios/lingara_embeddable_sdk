//! The turn's "one visible line" rule (Backend ADR 28.10.26e §2), as far as
//! a load can check it: text that is blank after trim, or that breaks a line.
//!
//! The schema cannot say either, so a dialogue holding such text loads, with
//! a warning (ADR 28.10.26f D2), and the core never sends a fact value that
//! breaks it. The server's own check is stricter (it also refuses format and
//! private-use characters); it still answers a turn that carries one with
//! `400 invalid_request`.

/// Why a text is not one visible line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineFault {
    /// Nothing left after trim.
    Blank,
    /// A line break, or another control character, inside the trimmed text.
    LineBreak,
}

impl LineFault {
    /// The fault in words, for a warning or a refusal.
    pub fn describe(self) -> &'static str {
        match self {
            LineFault::Blank => "is blank after trim, which a turn refuses",
            LineFault::LineBreak => "is not one visible line, which a turn refuses",
        }
    }
}

/// Whether `text` would pass the turn's one-line rule.
pub fn one_line(text: &str) -> Result<(), LineFault> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(LineFault::Blank);
    }
    let breaks = |c: char| c.is_control() || matches!(c, '\u{2028}' | '\u{2029}');
    if trimmed.chars().any(breaks) { Err(LineFault::LineBreak) } else { Ok(()) }
}
