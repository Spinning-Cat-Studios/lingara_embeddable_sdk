//! The procedural kind of checkpointed graph (ADR 3.11.26g §4–§8): the box an
//! author draws ([`ProceduralSpec`]), the atoms a planner composes inside it,
//! the fragment it returns ([`PlanFragment`]), and the deterministic
//! [`expand`] that turns a fragment into an ordinary static [`DialogueIr`].
//! The planner never writes a checkpoint; this module does, the same way
//! every time.

mod atoms;
mod expand;
mod fragment;
mod spec_rules;
mod spec_shape;
mod within_spec;
mod within_spec_rules;
mod wording;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};

use crate::ir::{DialogueIr, Ending, FactDecl, NpcProfile, Pedagogy};
use crate::limits::ABANDONED;

pub use atoms::{AtomShape, shape};
pub use expand::{expand, expand_with, seed};
pub use fragment::{Beat, Exit, PlanFragment};
pub(crate) use spec_rules::check as check_spec;
pub(crate) use spec_shape::walk_spec;
pub use within_spec::{within_spec, within_spec_restored};
pub use wording::{AtomText, AtomWording, GAME_WORDING};

/// A procedural graph: the cast, the facts and the endings, and the box the
/// planner fills. It has no `start` and no `checkpoints`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ProceduralDialogue {
    pub schema: String,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default)]
    pub pedagogy: Pedagogy,
    pub npcs: Vec<NpcProfile>,
    #[serde(default)]
    pub facts: Vec<FactDecl>,
    pub endings: Vec<Ending>,
    pub procedural: ProceduralSpec,
    /// Top-level `x-` members, kept as written and ignored.
    #[serde(flatten)]
    pub extensions: BTreeMap<String, Value>,
}

impl ProceduralDialogue {
    /// Whether `id` is a declared ending or `abandoned`.
    pub fn is_ending(&self, id: &str) -> bool {
        id == ABANDONED || self.endings.iter().any(|ending| ending.id == id)
    }

    /// The cast member named `id`.
    pub fn npc(&self, id: &str) -> Option<&NpcProfile> {
        self.npcs.iter().find(|npc| npc.id == id)
    }

    pub(crate) fn topic(&self, id: &str) -> Option<&Topic> {
        self.procedural.topics.iter().find(|topic| topic.id == id)
    }

    pub(crate) fn goal(&self, id: &str) -> Option<&Goal> {
        self.procedural.goals.iter().find(|goal| goal.id == id)
    }
}

/// The box a procedural graph is planned in (§4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ProceduralSpec {
    /// What the whole conversation is about. 1–240, one line.
    pub premise: String,
    /// The knowledge pool: declared fact keys no generated beat may disclose
    /// beyond (§8 W6).
    #[serde(default)]
    pub disclosable: Vec<String>,
    /// What may come up, with whom and when. 1–16.
    pub topics: Vec<Topic>,
    /// Never raised. Planner guidance: free text, so not a guarantee (§8).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub forbidden: Vec<Forbidden>,
    /// Named success targets a Practise beat or the termination aims at.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub goals: Vec<Goal>,
    /// The atoms the planner may compose: unique, `open` and `close` required.
    pub atoms: Vec<AtomUse>,
    #[serde(default)]
    pub horizon: Horizon,
    pub termination: Termination,
}

/// Something that may come up in a procedural conversation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Topic {
    pub id: String,
    /// What the topic is, as a phrase. 1–80.
    pub about: String,
    /// The cast ids that may raise it; absent is every cast member.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub npcs: Option<Vec<String>>,
    /// A K1 predicate over facts; absent is always open.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    /// The facts a Reveal beat on this topic discloses: within `disclosable`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub discloses: Vec<String>,
}

/// Something never to raise, as planner guidance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Forbidden {
    pub about: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub npcs: Option<Vec<String>>,
}

/// A named success target. At least one of `predicate` (the game decides,
/// auto-crossed) and `when` (the judge decides).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Goal {
    pub id: String,
    /// A phrase, 1–60.
    pub describe: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub predicate: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
}

/// One atom the planner may use.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct AtomUse {
    pub atom: AtomKind,
    /// Planner hint, 0..=1, default 0.5. Never validated against a fragment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefer: Option<Number>,
}

/// The five beat templates (§5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum AtomKind {
    Open,
    Elicit,
    Reveal,
    Practise,
    Close,
}

impl AtomKind {
    /// The wire word.
    pub fn as_str(self) -> &'static str {
        match self {
            AtomKind::Open => "open",
            AtomKind::Elicit => "elicit",
            AtomKind::Reveal => "reveal",
            AtomKind::Practise => "practise",
            AtomKind::Close => "close",
        }
    }
}

/// How far ahead to plan: the roadmap's tutor numbers, as defaults.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Horizon {
    /// 1..=3, default 2.
    #[serde(default = "Horizon::default_plan_depth")]
    pub plan_depth: u8,
    /// 0..=1, default 1.
    #[serde(default = "Horizon::default_sketch_depth")]
    pub sketch_depth: u8,
    /// 3..=16, default 10.
    #[serde(default = "Horizon::default_max_nodes")]
    pub max_nodes: u8,
}

impl Horizon {
    fn default_plan_depth() -> u8 {
        2
    }
    fn default_sketch_depth() -> u8 {
        1
    }
    fn default_max_nodes() -> u8 {
        10
    }
}

impl Default for Horizon {
    fn default() -> Self {
        Horizon {
            plan_depth: Horizon::default_plan_depth(),
            sketch_depth: Horizon::default_sketch_depth(),
            max_nodes: Horizon::default_max_nodes(),
        }
    }
}

/// When a procedural run stops.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Termination {
    /// 1..=200.
    pub max_turns: u16,
    /// Beats crossed, 1..=64.
    pub max_beats: u16,
    /// When this goal holds, the run ends on its ending.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success: Option<SuccessRule>,
    #[serde(default)]
    pub stall: StallRule,
    /// What `max_turns` or `max_beats` ends on. Default `abandoned`.
    #[serde(default = "abandoned")]
    pub on_limit: String,
}

/// The goal that ends a run well, and the ending it ends on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct SuccessRule {
    pub goal: String,
    pub ending: String,
}

/// `turns` consecutive turns with no crossing end the run on `ending`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct StallRule {
    /// 2..=20, default 6.
    #[serde(default = "StallRule::default_turns")]
    pub turns: u8,
    #[serde(default = "abandoned")]
    pub ending: String,
}

impl StallRule {
    fn default_turns() -> u8 {
        6
    }
}

impl Default for StallRule {
    fn default() -> Self {
        StallRule { turns: StallRule::default_turns(), ending: abandoned() }
    }
}

fn abandoned() -> String {
    ABANDONED.into()
}

/// The spec is `DialogueIr`'s procedural sibling; the shared members are
/// checked by one function over both.
pub(crate) fn shared_view(doc: &ProceduralDialogue) -> DialogueIr {
    DialogueIr {
        schema: doc.schema.clone(),
        id: doc.id.clone(),
        title: doc.title.clone(),
        pedagogy: doc.pedagogy,
        npcs: doc.npcs.clone(),
        facts: doc.facts.clone(),
        endings: doc.endings.clone(),
        start: String::new(),
        checkpoints: Vec::new(),
        extensions: BTreeMap::new(),
    }
}
