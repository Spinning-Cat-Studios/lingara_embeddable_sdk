//! K1's own bounds on `NpcDialogue`'s fields, checked at construction so a
//! bad value fails in the game's code rather than at the server.

use lingara_embed::{EmbedError, ErrorKind, limits};

/// 1 to `max` Unicode scalar values, not blank: K1's own rule.
pub(super) fn text(field: &str, max: usize, value: &str) -> Result<(), EmbedError> {
    let n = value.chars().count();
    if value.trim().is_empty() || n > max {
        return Err(EmbedError::new(ErrorKind::InvalidInput, format!("{field} must be 1–{max} characters, not blank (got {n})")));
    }
    Ok(())
}

pub(super) fn check_level(level: u8) -> Result<u8, EmbedError> {
    if !(limits::LEVEL_MIN..=limits::LEVEL_MAX).contains(&level) {
        let message = format!("level must be {}–{} (got {level})", limits::LEVEL_MIN, limits::LEVEL_MAX);
        return Err(EmbedError::new(ErrorKind::InvalidInput, message));
    }
    Ok(level)
}
