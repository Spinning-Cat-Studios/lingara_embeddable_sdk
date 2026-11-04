//! Which check refused, and which a warning is (ADR 3.11.26g §9). The words
//! are Backend's published `IrRule` and `WarningCode` names (Backend ADR
//! 28.10.26h D2), plus the ones `dialogue.v2` adds, so Backend maps a refusal
//! from this crate to its wire rule without a second validator.

use crate::text::TextFault;

/// Why a document, a spec or a fragment was refused, as the wire spells it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Rule {
    MissingMember,
    UnknownMember,
    IdPattern,
    /// A length, a count, the canonical size, or a member of the wrong JSON type.
    Bound,
    NumberRange,
    DuplicateId,
    UnknownTarget,
    /// An id shared by an ending, a checkpoint or `abandoned`.
    OutcomeName,
    UndeclaredFact,
    PredicateUnparsed,
    PredicateType,
    DefaultType,
    WhenLessCycle,
    TextBlank,
    TextMultiline,
    TextChar,
    /// `dialogue.v2`: a member of the other kind, or a procedural document read as a graph.
    KindMismatch,
    /// `dialogue.v2`: `steer_to` names no edge of its checkpoint with a `when`.
    SteerTarget,
    /// `dialogue.v2`: a disclosed key outside the knowledge pool.
    PoolEscape,
    /// `dialogue.v2`: the spec's atom set.
    AtomSet,
    /// A wording a consumer passed fails its check (§5).
    AtomWording,
    PlanAnchor,
    PlanAtom,
    PlanParams,
    PlanReference,
    PlanTopicClosed,
    PlanHorizon,
    PlanShape,
    PlanIds,
    PlanExits,
    PlanText,
}

impl Rule {
    /// Every rule, in the order above.
    pub const ALL: [Rule; 31] = [
        Rule::MissingMember,
        Rule::UnknownMember,
        Rule::IdPattern,
        Rule::Bound,
        Rule::NumberRange,
        Rule::DuplicateId,
        Rule::UnknownTarget,
        Rule::OutcomeName,
        Rule::UndeclaredFact,
        Rule::PredicateUnparsed,
        Rule::PredicateType,
        Rule::DefaultType,
        Rule::WhenLessCycle,
        Rule::TextBlank,
        Rule::TextMultiline,
        Rule::TextChar,
        Rule::KindMismatch,
        Rule::SteerTarget,
        Rule::PoolEscape,
        Rule::AtomSet,
        Rule::AtomWording,
        Rule::PlanAnchor,
        Rule::PlanAtom,
        Rule::PlanParams,
        Rule::PlanReference,
        Rule::PlanTopicClosed,
        Rule::PlanHorizon,
        Rule::PlanShape,
        Rule::PlanIds,
        Rule::PlanExits,
        Rule::PlanText,
    ];

    /// The wire word.
    pub fn as_str(self) -> &'static str {
        match self {
            Rule::MissingMember => "missing_member",
            Rule::UnknownMember => "unknown_member",
            Rule::IdPattern => "id_pattern",
            Rule::Bound => "bound",
            Rule::NumberRange => "number_range",
            Rule::DuplicateId => "duplicate_id",
            Rule::UnknownTarget => "unknown_target",
            Rule::OutcomeName => "outcome_name",
            Rule::UndeclaredFact => "undeclared_fact",
            Rule::PredicateUnparsed => "predicate_unparsed",
            Rule::PredicateType => "predicate_type",
            Rule::DefaultType => "default_type",
            Rule::WhenLessCycle => "when_less_cycle",
            Rule::TextBlank => "text_blank",
            Rule::TextMultiline => "text_multiline",
            Rule::TextChar => "text_char",
            Rule::KindMismatch => "kind_mismatch",
            Rule::SteerTarget => "steer_target",
            Rule::PoolEscape => "pool_escape",
            Rule::AtomSet => "atom_set",
            Rule::AtomWording => "atom_wording",
            Rule::PlanAnchor => "plan_anchor",
            Rule::PlanAtom => "plan_atom",
            Rule::PlanParams => "plan_params",
            Rule::PlanReference => "plan_reference",
            Rule::PlanTopicClosed => "plan_topic_closed",
            Rule::PlanHorizon => "plan_horizon",
            Rule::PlanShape => "plan_shape",
            Rule::PlanIds => "plan_ids",
            Rule::PlanExits => "plan_exits",
            Rule::PlanText => "plan_text",
        }
    }

    /// The refusal a text fault becomes under [`crate::Strictness::Save`].
    pub fn of_text(fault: TextFault) -> Rule {
        match fault {
            TextFault::Blank => Rule::TextBlank,
            TextFault::LineBreak => Rule::TextMultiline,
        }
    }
}

/// What a warning is about. A warning never refuses under
/// [`crate::Strictness::Load`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WarningCode {
    CheckpointUnreachable,
    /// "no ending reachable" (§11.1).
    OutcomeUnreachable,
    /// A railroading NPC that also follows the player readily (§3).
    StanceContradiction,
    /// A `steer_to` no cast member drives hard enough to act on (§3).
    SteerWeak,
    /// Text the schema allows and a turn refuses.
    Text(TextFault),
}

impl WarningCode {
    /// The wire word. A text warning spells the refusal it becomes on save.
    pub fn as_str(self) -> &'static str {
        match self {
            WarningCode::CheckpointUnreachable => "checkpoint_unreachable",
            WarningCode::OutcomeUnreachable => "outcome_unreachable",
            WarningCode::StanceContradiction => "stance_contradiction",
            WarningCode::SteerWeak => "steer_weak",
            WarningCode::Text(fault) => Rule::of_text(fault).as_str(),
        }
    }
}
