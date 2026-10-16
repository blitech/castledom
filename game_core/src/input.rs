use serde::{Deserialize, Serialize};

use crate::types::{FactionId, MAX_FACTIONS};

/// World pixels per grid tile (art + footprint unit).
pub const TILE_SIZE: f32 = 32.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DamageType {
    Physical,
    Magical,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Targeting {
    Single,
    Multitarget { count: u8 },
    Aoe { radius: f32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnitType {
    Soldier,
    Archer,
    Mage,
    Priest,
}

impl UnitType {
    pub fn hp(self) -> f32 {
        match self {
            UnitType::Soldier => 55.0,
            UnitType::Archer => 32.0,
            UnitType::Mage => 28.0,
            UnitType::Priest => 36.0,
        }
    }

    pub fn damage(self) -> f32 {
        match self {
            UnitType::Soldier => 10.0,
            UnitType::Archer => 8.0,
            UnitType::Mage => 12.0,
            UnitType::Priest => 0.0,
        }
    }

    pub fn heal_power(self) -> f32 {
        match self {
            UnitType::Priest => 14.0,
            _ => 0.0,
        }
    }

    pub fn damage_type(self) -> DamageType {
        match self {
            UnitType::Soldier | UnitType::Archer => DamageType::Physical,
            UnitType::Mage | UnitType::Priest => DamageType::Magical,
        }
    }

    pub fn armor_phys(self) -> f32 {
        match self {
            UnitType::Soldier => 25.0,
            UnitType::Archer => 5.0,
            UnitType::Mage => 0.0,
            UnitType::Priest => 8.0,
        }
    }

    pub fn armor_mag(self) -> f32 {
        match self {
            UnitType::Soldier => 5.0,
            UnitType::Archer => 5.0,
            UnitType::Mage => 20.0,
            UnitType::Priest => 15.0,
        }
    }

    pub fn pen_phys(self) -> f32 {
        match self {
            UnitType::Soldier => 5.0,
            UnitType::Archer => 10.0,
            _ => 0.0,
        }
    }

    pub fn pen_mag(self) -> f32 {
        match self {
            UnitType::Mage => 8.0,
            _ => 0.0,
        }
    }

    pub fn targeting(self) -> Targeting {
        match self {
            UnitType::Soldier | UnitType::Archer | UnitType::Priest => Targeting::Single,
            UnitType::Mage => Targeting::Aoe { radius: 28.0 },
        }
    }

    pub fn range(self) -> f32 {
        match self {
            UnitType::Soldier => 20.0,
            UnitType::Archer => 70.0,
            UnitType::Mage => 55.0,
            UnitType::Priest => 50.0,
        }
    }

    pub fn speed(self) -> f32 {
        match self {
            UnitType::Soldier => 52.0,
            UnitType::Archer => 48.0,
            UnitType::Mage => 44.0,
            UnitType::Priest => 46.0,
        }
    }

    pub fn radius(self) -> f32 {
        match self {
            UnitType::Soldier => 7.0,
            UnitType::Archer => 6.0,
            UnitType::Mage => 6.5,
            UnitType::Priest => 6.5,
        }
    }

    pub fn attack_cooldown(self) -> f32 {
        match self {
            UnitType::Soldier => 0.50,
            UnitType::Archer => 0.55,
            UnitType::Mage => 0.75,
            UnitType::Priest => 0.70,
        }
    }

    pub fn all() -> [UnitType; 4] {
        [
            UnitType::Soldier,
            UnitType::Archer,
            UnitType::Mage,
            UnitType::Priest,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            UnitType::Soldier => "Soldat",
            UnitType::Archer => "Archer",
            UnitType::Mage => "Mage",
            UnitType::Priest => "Prêtre",
        }
    }
}

/// Mitigate raw damage with armor after penetration.
pub fn mitigate_damage(raw: f32, armor: f32, pen: f32) -> f32 {
    let effective = (armor - pen).max(0.0);
    raw * (100.0 / (100.0 + effective))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BuildingKind {
    Barracks,
    Archery,
    MageTower,
    Temple,
}

impl BuildingKind {
    pub fn cost(self) -> f32 {
        match self {
            BuildingKind::Barracks => 40.0,
            BuildingKind::Archery => 55.0,
            BuildingKind::MageTower => 70.0,
            BuildingKind::Temple => 60.0,
        }
    }

    pub fn produces(self) -> UnitType {
        match self {
            BuildingKind::Barracks => UnitType::Soldier,
            BuildingKind::Archery => UnitType::Archer,
            BuildingKind::MageTower => UnitType::Mage,
            BuildingKind::Temple => UnitType::Priest,
        }
    }

    /// Produce interval in seconds.
    pub fn produce_interval(self) -> f32 {
        match self {
            BuildingKind::Barracks => 2.0,
            BuildingKind::Archery => 2.4,
            BuildingKind::MageTower => 3.0,
            BuildingKind::Temple => 2.8,
        }
    }

    /// Width × height in tiles. Engine supports 1×2, 2×2, 3×3; v1 all 2×2.
    pub fn footprint(self) -> (u8, u8) {
        match self {
            BuildingKind::Barracks
            | BuildingKind::Archery
            | BuildingKind::MageTower
            | BuildingKind::Temple => (2, 2),
        }
    }

    pub fn all() -> [BuildingKind; 4] {
        [
            BuildingKind::Barracks,
            BuildingKind::Archery,
            BuildingKind::MageTower,
            BuildingKind::Temple,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            BuildingKind::Barracks => "Caserne",
            BuildingKind::Archery => "Archerie",
            BuildingKind::MageTower => "Tour de mage",
            BuildingKind::Temple => "Temple",
        }
    }

    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(BuildingKind::Barracks),
            1 => Some(BuildingKind::Archery),
            2 => Some(BuildingKind::MageTower),
            3 => Some(BuildingKind::Temple),
            _ => None,
        }
    }

    pub fn as_u8(self) -> u8 {
        match self {
            BuildingKind::Barracks => 0,
            BuildingKind::Archery => 1,
            BuildingKind::MageTower => 2,
            BuildingKind::Temple => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaceBuildingCmd {
    pub kind: BuildingKind,
    pub tile_x: i16,
    pub tile_y: i16,
}

/// Per-tick input for one faction (lockstep payload).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameInput {
    pub place: Option<PlaceBuildingCmd>,
}

impl GameInput {
    pub fn idle() -> Self {
        Self { place: None }
    }

    pub fn place(kind: BuildingKind, tile_x: i16, tile_y: i16) -> Self {
        Self {
            place: Some(PlaceBuildingCmd {
                kind,
                tile_x,
                tile_y,
            }),
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

pub fn world_to_tile(x: f32, y: f32) -> (i16, i16) {
    (
        (x / TILE_SIZE).floor() as i16,
        (y / TILE_SIZE).floor() as i16,
    )
}

pub fn tile_to_world_center(tx: i16, ty: i16) -> (f32, f32) {
    (
        (tx as f32 + 0.5) * TILE_SIZE,
        (ty as f32 + 0.5) * TILE_SIZE,
    )
}

pub fn footprint_center(tx: i16, ty: i16, fw: u8, fh: u8) -> (f32, f32) {
    (
        (tx as f32 + fw as f32 * 0.5) * TILE_SIZE,
        (ty as f32 + fh as f32 * 0.5) * TILE_SIZE,
    )
}
