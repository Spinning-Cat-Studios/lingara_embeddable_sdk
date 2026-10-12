//! The dialogue window (D8, EK6), pure: what a turn sends as `history`.
//!
//! A finished turn appends the player's line, then the NPC's reply cut to
//! its first 500 characters (as B3 tells clients to store it); a reply empty
//! after trimming appends nothing, because the server would refuse it. The
//! window is then trimmed oldest first to 12 entries. B3 sets no total, so
//! there is no character trim. Characters are Unicode scalar values.

use crate::limits;
use crate::types::{Speaker, WindowEntry};

/// `text`'s first `max` Unicode scalar values.
pub(crate) fn cut(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Window {
    entries: Vec<WindowEntry>,
}

impl Window {
    /// A window a game supplies at `dialogue_open`, under the append rule.
    pub fn from_supplied(entries: Vec<WindowEntry>) -> Self {
        let mut window = Self::default();
        for entry in entries {
            window.push(entry.speaker, &entry.text);
        }
        window.trim();
        window
    }

    fn push(&mut self, speaker: Speaker, text: &str) {
        if !text.trim().is_empty() {
            self.entries.push(WindowEntry { speaker, text: cut(text, limits::DIALOGUE_TEXT_MAX_CHARS) });
        }
    }

    fn trim(&mut self) {
        let over = self.entries.len().saturating_sub(limits::DIALOGUE_HISTORY_MAX_ITEMS);
        self.entries.drain(..over);
    }

    /// A finished turn: the line, then the reply.
    pub fn push_turn(&mut self, line: &str, reply: &str) {
        self.push(Speaker::Player, line);
        self.push(Speaker::Npc, reply);
        self.trim();
    }

    pub fn entries(&self) -> &[WindowEntry] {
        &self.entries
    }
}
