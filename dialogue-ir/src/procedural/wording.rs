//! The words an atom is spoken with (ADR 3.11.26g §5), separate from its kind.
//! A kind fixes parameters, checkpoints, edges, ids and the rules §8 checks; a
//! wording fills only the `goal` and `when` templates and the display label.
//! Another consumer re-words an atom by passing its own [`AtomWording`] to
//! `expand_with` (3.11.26r adds `TUTOR_WORDING` beside [`GAME_WORDING`]).

use super::AtomKind;
use crate::limits::{MAX_ABOUT_CHARS, MAX_ANGLE_CHARS, MAX_CONDITION_CHARS, MAX_DESCRIBE_CHARS, MAX_GOAL_CHARS};
use crate::text::one_line;
use crate::{IrError, Rule};

const TOPIC: &str = "‹topic›";
const GOAL: &str = "‹goal›";
/// Substituted with its leading `", "`, or with nothing when there is no angle.
const ANGLE: &str = "‹angle›";

/// The text one atom kind is spoken with. Placeholders: ‹topic›, ‹goal›,
/// ‹angle› (with its leading ", ").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtomText {
    pub label: &'static str,
    pub goal: &'static str,
    /// Practise's `done_when` is unused and must be empty: its done edge's
    /// `when` is the author's `goal.when`, verbatim, and `check()` refuses a
    /// non-empty one.
    pub done_when: &'static str,
    /// Practise only: its `_again` checkpoint's goal, and its try / retry conditions.
    pub again_goal: Option<&'static str>,
    pub try_when: Option<&'static str>,
    pub retry_when: Option<&'static str>,
}

/// One wording for every kind, plus the detour condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtomWording {
    pub open: AtomText,
    pub elicit: AtomText,
    pub reveal: AtomText,
    pub practise: AtomText,
    pub close: AtomText,
    /// "The player brings up ‹topic›."
    pub detour_when: &'static str,
}

const fn plain(label: &'static str, goal: &'static str, done_when: &'static str) -> AtomText {
    AtomText { label, goal, done_when, again_goal: None, try_when: None, retry_when: None }
}

/// The game's words, §5's table.
pub const GAME_WORDING: AtomWording = AtomWording {
    open: plain(
        "Open",
        "Greet the player and set the scene‹angle›",
        "The player has greeted the character or said why they are here.",
    ),
    elicit: plain("Elicit", "Find out from the player ‹topic›‹angle›", "The player has said something about ‹topic›."),
    reveal: plain(
        "Reveal",
        "Talk about ‹topic›; share what you know when it fits‹angle›",
        "The character has shared what they know about ‹topic›.",
    ),
    practise: AtomText {
        label: "Negotiate",
        goal: "Work with the player on ‹topic› until ‹goal›‹angle›",
        done_when: "",
        again_goal: Some("Respond to the player's attempt and invite another"),
        try_when: Some("The player makes an attempt at ‹goal›."),
        retry_when: Some("The player tries again."),
    },
    close: plain(
        "Close",
        "Wrap up the conversation and say goodbye‹angle›",
        "The player says goodbye or the conversation has come to a natural end.",
    ),
    detour_when: "The player brings up ‹topic›.",
};

impl AtomWording {
    pub fn text(&self, kind: AtomKind) -> &AtomText {
        match kind {
            AtomKind::Open => &self.open,
            AtomKind::Elicit => &self.elicit,
            AtomKind::Reveal => &self.reveal,
            AtomKind::Practise => &self.practise,
            AtomKind::Close => &self.close,
        }
    }

    /// Checked when used, not when declared: every template names only its
    /// kind's placeholders (any subset), is one visible line, and fits the
    /// bound it fills at the parameters' maximum lengths. Refuses with
    /// `atom_wording`.
    pub fn check(&self) -> Result<(), IrError> {
        for kind in [AtomKind::Open, AtomKind::Elicit, AtomKind::Reveal, AtomKind::Practise, AtomKind::Close] {
            check_text(kind, self.text(kind))?;
        }
        template("wording.detour_when", self.detour_when, &[TOPIC], MAX_CONDITION_CHARS)
    }
}

fn check_text(kind: AtomKind, text: &AtomText) -> Result<(), IrError> {
    let at = |field: &str| format!("wording.{}.{field}", kind.as_str());
    let allowed: &[&str] = match kind {
        AtomKind::Open | AtomKind::Close => &[ANGLE],
        AtomKind::Elicit | AtomKind::Reveal => &[TOPIC, ANGLE],
        AtomKind::Practise => &[TOPIC, GOAL, ANGLE],
    };
    if text.label.trim().is_empty() {
        return Err(IrError::new(Rule::AtomWording, at("label"), "is blank"));
    }
    template(&at("goal"), text.goal, allowed, MAX_GOAL_CHARS)?;
    let practise = kind == AtomKind::Practise;
    let extras = [("again_goal", text.again_goal, MAX_GOAL_CHARS), ("try_when", text.try_when, MAX_CONDITION_CHARS), ("retry_when", text.retry_when, MAX_CONDITION_CHARS)];
    if practise {
        if !text.done_when.is_empty() {
            return Err(IrError::new(Rule::AtomWording, at("done_when"), "must be empty: Practise's done edge uses the goal's `when`"));
        }
        for (field, value, max) in extras {
            let value = value.ok_or_else(|| IrError::new(Rule::AtomWording, at(field), "is required for Practise"))?;
            template(&at(field), value, &[TOPIC, GOAL], max)?;
        }
        return Ok(());
    }
    if let Some((field, _, _)) = extras.iter().find(|(_, value, _)| value.is_some()) {
        return Err(IrError::new(Rule::AtomWording, at(field), "is Practise's only"));
    }
    template(&at("done_when"), text.done_when, allowed, MAX_CONDITION_CHARS)
}

/// One template: known placeholders only, one line, within `max` filled.
fn template(path: &str, text: &str, allowed: &[&str], max: usize) -> Result<(), IrError> {
    if let Some(name) = placeholders(text).find(|name| !allowed.contains(name)) {
        return Err(IrError::new(Rule::AtomWording, path, format!("names {name}, which this kind does not have")));
    }
    one_line(text).map_err(|fault| IrError::new(Rule::AtomWording, path, fault.describe()))?;
    let longest = fill(text, &"x".repeat(MAX_ABOUT_CHARS), &"x".repeat(MAX_DESCRIBE_CHARS), Some(&"x".repeat(MAX_ANGLE_CHARS)));
    let n = longest.chars().count();
    if n > max {
        return Err(IrError::new(Rule::AtomWording, path, format!("is {n} characters at its parameters' longest; at most {max}")));
    }
    Ok(())
}

/// Every `‹…›` a template names, delimiters included.
fn placeholders(text: &str) -> impl Iterator<Item = &str> {
    text.match_indices('‹').filter_map(move |(start, _)| {
        let end = text[start..].find('›')? + start + '›'.len_utf8();
        Some(&text[start..end])
    })
}

/// A template with its parameters substituted verbatim; ‹angle› becomes
/// `", angle"`, or disappears with no angle.
pub(crate) fn fill(template: &str, topic: &str, goal: &str, angle: Option<&str>) -> String {
    let angle = angle.map(|a| format!(", {a}")).unwrap_or_default();
    template.replace(TOPIC, topic).replace(GOAL, goal).replace(ANGLE, &angle)
}
