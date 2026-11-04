//! The steering stance as a model reads it (ADR 3.11.26g §3): the author's
//! numbers become **bands**, computed here and only here, so the NPC prompt
//! (Backend 3.11.26h), the planner (3.11.26i) and `within_spec` cannot
//! disagree. The portal keeps one cited copy (`stanceBands.ts`, portal
//! 3.11.26l L3): change a threshold or `max_detours` there too.

use crate::ir::Disposition;

/// Below this, a member is Low.
pub const BAND_LOW_BELOW: f64 = 1.0 / 3.0;
/// From this, a member is High.
pub const BAND_HIGH_FROM: f64 = 2.0 / 3.0;
/// From this, initiative railroads.
pub const RAILROAD_FROM: f64 = 0.9;
/// What an absent disposition plans as, for both members.
pub const ABSENT: f64 = 0.5;

/// How readily the NPC follows the player off its goal (`suggestibility`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Following {
    Low,
    Medium,
    High,
}

/// How hard the NPC drives toward its goal (`initiative`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Drive {
    Low,
    Medium,
    High,
    /// Keeps returning to the goal and declines detours in character.
    Railroad,
}

/// `< 1/3` Low, `< 2/3` Medium, else High.
pub fn following(suggestibility: f64) -> Following {
    if suggestibility < BAND_LOW_BELOW {
        Following::Low
    } else if suggestibility < BAND_HIGH_FROM {
        Following::Medium
    } else {
        Following::High
    }
}

/// `< 1/3` Low, `< 2/3` Medium, `< 0.9` High, else Railroad.
pub fn drive(initiative: f64) -> Drive {
    if initiative < BAND_LOW_BELOW {
        Drive::Low
    } else if initiative < BAND_HIGH_FROM {
        Drive::Medium
    } else if initiative < RAILROAD_FROM {
        Drive::High
    } else {
        Drive::Railroad
    }
}

/// Both bands of one NPC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Stance {
    pub following: Following,
    pub drive: Drive,
}

impl Stance {
    /// An absent disposition plans as (0.5, 0.5): Medium, Medium. The prompt
    /// leaves an absent stance unsaid (3.11.26h).
    pub fn of(disposition: Option<&Disposition>) -> Stance {
        let member = |n: &serde_json::Number| n.as_f64().unwrap_or(ABSENT);
        let (s, i) = disposition.map_or((ABSENT, ABSENT), |d| (member(&d.suggestibility), member(&d.initiative)));
        Stance { following: following(s), drive: drive(i) }
    }

    /// The detours `within_spec` allows per planned beat (§8 W10): Low 0,
    /// Medium 1, High 2 by `following`; `Railroad` overrides it to 0.
    pub fn max_detours(&self) -> usize {
        if self.drive == Drive::Railroad {
            return 0;
        }
        match self.following {
            Following::Low => 0,
            Following::Medium => 1,
            Following::High => 2,
        }
    }

    /// Whether an expansion sets `steer_to` for this NPC (§7).
    pub fn steers(&self) -> bool {
        matches!(self.drive, Drive::High | Drive::Railroad)
    }

    /// Railroad with High following contradicts itself: railroad wins (§3).
    pub fn contradicts(&self) -> bool {
        self.drive == Drive::Railroad && self.following == Following::High
    }
}
