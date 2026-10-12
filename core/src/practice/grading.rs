//! Local grading (D7): `/v1/` has no grading route, so a question is graded
//! by its shape. An unknown `type` is never an error: it is a typed answer.
//!
//! Normalisation trims, folds case, removes punctuation wherever it occurs
//! and collapses runs of whitespace, so `"Ni hao!"` matches `"ni  hao"`.

use crate::limits;

/// True for the three types Lingara's own apps send to a model.
pub(crate) fn is_self_graded(kind: &str) -> bool {
    limits::SELF_GRADED_TYPES.contains(&kind)
}

/// Unicode general category P, by block: ASCII and Latin-1, General
/// Punctuation, and the CJK, fullwidth, Arabic, Devanagari, Thai, Armenian,
/// Hebrew and Ethiopic marks of the scripts Lingara teaches. The crate takes
/// no Unicode table for one predicate.
fn is_punctuation(c: char) -> bool {
    const RANGES: [(u32, u32); 24] = [
        (0x00A1, 0x00A1), (0x00A7, 0x00A7), (0x00AB, 0x00AB), (0x00B6, 0x00B7), (0x00BB, 0x00BB), (0x00BF, 0x00BF),
        (0x055A, 0x055F), (0x0589, 0x058A), (0x05BE, 0x05BE), (0x05C0, 0x05C6), (0x060C, 0x060D), (0x061B, 0x061F),
        (0x066A, 0x066D), (0x06D4, 0x06D4), (0x0964, 0x0965), (0x0E4F, 0x0E5B), (0x1361, 0x1368), (0x2010, 0x2027),
        (0x2030, 0x205E), (0x3001, 0x3003), (0x3008, 0x3011), (0x3014, 0x301F), (0x30FB, 0x30FB), (0xFF01, 0xFF65),
    ];
    if c.is_ascii() {
        return c.is_ascii_punctuation() && !matches!(c, '$' | '+' | '<' | '=' | '>' | '^' | '`' | '|' | '~');
    }
    let cp = u32::from(c);
    // The fullwidth block mixes symbols with punctuation; its letters and
    // digits are excluded here.
    RANGES.iter().any(|&(lo, hi)| (lo..=hi).contains(&cp)) && !c.is_alphanumeric()
}

pub(crate) fn normalise(text: &str) -> String {
    let lowered = text.to_lowercase();
    let stripped: String = lowered.chars().filter(|c| !is_punctuation(*c)).collect();
    stripped.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Equality after normalisation.
pub(crate) fn matches(given: &str, expected: &str) -> bool {
    normalise(given) == normalise(expected)
}
