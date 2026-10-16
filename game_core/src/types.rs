use serde::{Deserialize, Serialize};

/// Maximum supported factions (slots) in a match.
pub const MAX_FACTIONS: usize = 8;

/// Unique player/AI slot (camp).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FactionId(pub u8);

impl FactionId {
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Alliance grouping — allied factions share victory and do not fight each other.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TeamId(pub u8);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchMode {
    /// Two factions, two teams (1v1).
    Duel,
    /// Four factions, two teams of two (2v2).
    TwoVTwo,
    /// Free-for-all up to `faction_count` (each faction alone).
    FreeForAll { faction_count: u8 },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MatchConfig {
    pub mode: MatchMode,
    /// Which slots are controlled by AI (true = AI).
    pub ai_slots: Vec<bool>,
    /// Human local faction when playing solo / hotseat host view.
    pub local_faction: FactionId,
}

impl MatchConfig {
    pub fn duel_vs_ai() -> Self {
        Self {
            mode: MatchMode::Duel,
            ai_slots: vec![false, true],
            local_faction: FactionId(0),
        }
    }

    pub fn two_v_two_solo() -> Self {
        // Human on team A slot 0; rest AI. Teams: (0,1) vs (2,3)
        Self {
            mode: MatchMode::TwoVTwo,
            ai_slots: vec![false, true, true, true],
            local_faction: FactionId(0),
        }
    }

    pub fn faction_count(&self) -> usize {
        match self.mode {
            MatchMode::Duel => 2,
            MatchMode::TwoVTwo => 4,
            MatchMode::FreeForAll { faction_count } => faction_count as usize,
        }
    }

    pub fn team_for(&self, faction: FactionId) -> TeamId {
        match self.mode {
            MatchMode::Duel => TeamId(faction.0),
            MatchMode::TwoVTwo => TeamId(faction.0 / 2),
            MatchMode::FreeForAll { .. } => TeamId(faction.0),
        }
    }

    pub fn is_ai(&self, faction: FactionId) -> bool {
        self.ai_slots
            .get(faction.index())
            .copied()
            .unwrap_or(true)
    }
}

/// Per-faction display colors (RGBA 0–1), used by render layer.
pub fn faction_color(faction: FactionId) -> [f32; 4] {
    match faction.0 % 8 {
        0 => [0.25, 0.45, 0.95, 1.0], // blue
        1 => [0.95, 0.30, 0.30, 1.0], // red
        2 => [0.25, 0.85, 0.40, 1.0], // green
        3 => [0.95, 0.80, 0.20, 1.0], // yellow
        4 => [0.70, 0.35, 0.95, 1.0], // purple
        5 => [0.20, 0.85, 0.85, 1.0], // cyan
        6 => [0.95, 0.55, 0.20, 1.0], // orange
        _ => [0.85, 0.85, 0.85, 1.0], // white
    }
}
