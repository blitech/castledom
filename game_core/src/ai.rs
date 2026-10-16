//! Simple heuristic AI that emits the same `GameInput` as a human player.

use crate::input::{GameInput, UnitType};
use crate::types::FactionId;
use crate::world::SimWorld;

#[derive(Clone, Debug, Default)]
pub struct AiController {
    /// Prefer producing this type; rotates over time.
    prefer: u8,
}

impl AiController {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn decide(&mut self, world: &SimWorld, faction: FactionId) -> GameInput {
        let i = faction.index();
        if i >= world.gold.len() {
            return GameInput::idle();
        }
        let gold = world.gold[i];
        let my_units = world.units.iter().filter(|u| u.faction == faction).count();
        let enemy_units = world
            .units
            .iter()
            .filter(|u| !world.are_allied(u.faction, faction))
            .count();

        // Composition heuristic
        let melee = world
            .units
            .iter()
            .filter(|u| u.faction == faction && u.unit_type == UnitType::Melee)
            .count();
        let ranged = world
            .units
            .iter()
            .filter(|u| u.faction == faction && u.unit_type == UnitType::Ranged)
            .count();
        let tanks = world
            .units
            .iter()
            .filter(|u| u.faction == faction && u.unit_type == UnitType::Tank)
            .count();

        let want = if tanks * 4 < my_units.max(1) && enemy_units > my_units {
            UnitType::Tank
        } else if ranged < melee {
            UnitType::Ranged
        } else if self.prefer % 5 == 0 {
            UnitType::Tank
        } else if self.prefer % 2 == 0 {
            UnitType::Ranged
        } else {
            UnitType::Melee
        };

        self.prefer = self.prefer.wrapping_add(1);

        // Always try to produce if we can afford something useful
        if gold >= want.cost() {
            return GameInput::produce(want);
        }
        for ut in UnitType::all() {
            if gold >= ut.cost() {
                return GameInput::produce(ut);
            }
        }
        GameInput::idle()
    }
}
