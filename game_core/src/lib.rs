//! Deterministic hybrid RTS simulation (Castle Wars × Domination Wars).
//! No rendering — pure tick logic for solo, AI, and lockstep multiplayer.

mod ai;
mod flow_field;
mod input;
mod lobby;
mod types;
mod world;

pub use ai::AiController;
pub use flow_field::FlowFieldSet;
pub use input::{GameInput, InputBus, UnitType};
pub use lobby::{
    normalize_code, random_code, LobbyPhase, LobbySlot, LobbyState, SlotKind,
};
pub use types::{faction_color, FactionId, MatchConfig, MatchMode, TeamId, MAX_FACTIONS};
pub use world::{
    CapturePoint, Castle, SimUnit, SimWorld, TickResult, VictoryState, MAP_HEIGHT, MAP_WIDTH,
};

pub const TICK_HZ: u32 = 20;
pub const TICK_DT: f32 = 1.0 / TICK_HZ as f32;
