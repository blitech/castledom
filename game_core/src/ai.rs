//! Heuristic AI that places production buildings (same `GameInput` as humans).

use crate::input::{BuildingKind, GameInput};
use crate::types::FactionId;
use crate::world::SimWorld;

#[derive(Clone, Debug, Default)]
pub struct AiController {
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

        let has = |kind: BuildingKind| {
            world
                .buildings
                .iter()
                .any(|b| b.faction == faction && b.kind == kind)
        };

        // Build missing production in priority order.
        let order = [
            BuildingKind::Barracks,
            BuildingKind::Archery,
            BuildingKind::MageTower,
            BuildingKind::Temple,
        ];

        self.prefer = self.prefer.wrapping_add(1);

        // Occasionally skip temple until other buildings exist.
        for kind in order {
            if kind == BuildingKind::Temple && self.prefer % 4 != 0 {
                let others = world
                    .buildings
                    .iter()
                    .filter(|b| b.faction == faction && b.kind != BuildingKind::Temple)
                    .count();
                if others < 2 {
                    continue;
                }
            }
            if has(kind) {
                // Allow a second barracks if rich
                if kind == BuildingKind::Barracks && gold >= kind.cost() * 2.0 && self.prefer % 7 == 0
                {
                    // fall through to try place another
                } else {
                    continue;
                }
            }
            if gold < kind.cost() {
                continue;
            }
            if let Some((tx, ty)) = world.find_build_site(faction, kind) {
                return GameInput::place(kind, tx, ty);
            }
        }

        // Extra barracks / archery when economy is strong
        if gold >= 80.0 {
            let extras = [BuildingKind::Barracks, BuildingKind::Archery];
            let kind = extras[self.prefer as usize % extras.len()];
            if let Some((tx, ty)) = world.find_build_site(faction, kind) {
                return GameInput::place(kind, tx, ty);
            }
        }

        GameInput::idle()
    }
}
