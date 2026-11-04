//! What a planner returns, and what the core keeps as its frontier: a rooted
//! tree of beats (ADR 3.11.26g §6). Beats name atoms and parameters only; the
//! one free text is `angle`.

use serde::{Deserialize, Serialize};

use super::AtomKind;

/// A rooted tree of beats, `beats[0]` the anchor at depth 0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct PlanFragment {
    pub anchor: String,
    pub beats: Vec<Beat>,
}

impl PlanFragment {
    /// The beat named `id`.
    pub fn beat(&self, id: &str) -> Option<&Beat> {
        self.beats.iter().find(|beat| beat.id == id)
    }
}

/// One beat: an atom instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Beat {
    /// `^[a-z][a-z0-9_]{0,23}$`.
    pub id: String,
    pub atom: AtomKind,
    /// 0 is the anchor.
    pub depth: u8,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub sketch: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
    /// Practise only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goal: Option<String>,
    /// Close only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending: Option<String>,
    /// The planner's only free text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub angle: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exits: Vec<Exit>,
}

impl Beat {
    /// The done exit, if the beat has one.
    pub fn done_exit(&self) -> Option<&Exit> {
        self.exits.iter().find(|exit| exit.detour.is_none())
    }

    /// The parameters a beat is compared on when it is re-anchored (§6).
    pub(crate) fn same_instance(&self, other: &Beat) -> bool {
        (self.atom, &self.topic, &self.goal, &self.ending, &self.angle)
            == (other.atom, &other.topic, &other.goal, &other.ending, &other.angle)
    }
}

/// A way out of a beat, one level deeper.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Exit {
    /// A beat of this fragment.
    pub to: String,
    /// Absent: the done exit. A topic id: a player-led branch on that topic.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detour: Option<String>,
}
