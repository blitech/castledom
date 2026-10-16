use serde::{Deserialize, Serialize};

use crate::types::{FactionId, MAX_FACTIONS};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnitType {
    Melee,
    Ranged,
    Tank,
}

impl UnitType {
    pub fn cost(self) -> f32 {
        match self {
            UnitType::Melee => 10.0,
            UnitType::Ranged => 14.0,
            UnitType::Tank => 22.0,
        }
    }

    pub fn hp(self) -> f32 {
        match self {
            UnitType::Melee => 40.0,
            UnitType::Ranged => 28.0,
            UnitType::Tank => 90.0,
        }
    }

    pub fn damage(self) -> f32 {
        match self {
            UnitType::Melee => 8.0,
            UnitType::Ranged => 6.0,
            UnitType::Tank => 10.0,
        }
    }

    pub fn range(self) -> f32 {
        match self {
            UnitType::Melee => 18.0,
            UnitType::Ranged => 55.0,
            UnitType::Tank => 22.0,
        }
    }

    pub fn speed(self) -> f32 {
        match self {
            UnitType::Melee => 55.0,
            UnitType::Ranged => 48.0,
            UnitType::Tank => 35.0,
        }
    }

    pub fn radius(self) -> f32 {
        match self {
            UnitType::Melee => 6.0,
            UnitType::Ranged => 5.5,
            UnitType::Tank => 9.0,
        }
    }

    pub fn all() -> [UnitType; 3] {
        [UnitType::Melee, UnitType::Ranged, UnitType::Tank]
    }
}

/// Per-tick input for one faction (lockstep payload).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameInput {
    /// Produce this unit type if affordable (None = idle).
    pub produce: Option<UnitType>,
}

impl GameInput {
    pub fn idle() -> Self {
        Self { produce: None }
    }

    pub fn produce(unit: UnitType) -> Self {
        Self {
            produce: Some(unit),
        }
    }
}

/// Collects inputs for all factions for the current tick.
#[derive(Clone, Debug)]
pub struct InputBus {
    inputs: [GameInput; MAX_FACTIONS],
}

impl Default for InputBus {
    fn default() -> Self {
        Self {
            inputs: [GameInput::idle(); MAX_FACTIONS],
        }
    }
}

impl InputBus {
    pub fn clear(&mut self) {
        self.inputs = [GameInput::idle(); MAX_FACTIONS];
    }

    pub fn set(&mut self, faction: FactionId, input: GameInput) {
        if faction.index() < MAX_FACTIONS {
            self.inputs[faction.index()] = input;
        }
    }

    pub fn get(&self, faction: FactionId) -> GameInput {
        self.inputs
            .get(faction.index())
            .copied()
            .unwrap_or_default()
    }

    pub fn as_slice(&self) -> &[GameInput; MAX_FACTIONS] {
        &self.inputs
    }
}
