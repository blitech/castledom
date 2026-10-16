//! Shared lobby state (local solo + multiplayer), Cémantix-style slots.

use serde::{Deserialize, Serialize};

use crate::types::{FactionId, MatchConfig, MatchMode, TeamId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlotKind {
    Empty,
    Human,
    Ai,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LobbySlot {
    pub faction: FactionId,
    pub team: TeamId,
    pub kind: SlotKind,
    pub label: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LobbyPhase {
    Waiting,
    Locked,
    Playing,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LobbyState {
    pub code: String,
    pub mode: MatchMode,
    pub phase: LobbyPhase,
    pub slots: Vec<LobbySlot>,
    pub host_faction: FactionId,
    pub local_faction: FactionId,
    pub is_host: bool,
    /// When true, match runs without Matchbox (solo / local bots).
    pub local_only: bool,
}

impl LobbyState {
    pub fn new_local(mode: MatchMode) -> Self {
        let mut lobby = Self::new_host(mode, "LOCAL".into(), true);
        for slot in lobby.slots.iter_mut().skip(1) {
            slot.kind = SlotKind::Ai;
            slot.label = format!("IA {}", slot.faction.0);
        }
        lobby
    }

    pub fn new_host(mode: MatchMode, code: String, local_only: bool) -> Self {
        let n = mode_faction_count(mode);
        let mut slots = Vec::with_capacity(n);
        for i in 0..n {
            let faction = FactionId(i as u8);
            let team = team_for_mode(mode, faction);
            let (kind, label) = if i == 0 {
                (SlotKind::Human, "Toi (host)".into())
            } else if local_only {
                (SlotKind::Ai, format!("IA {i}"))
            } else {
                (SlotKind::Empty, format!("Libre"))
            };
            slots.push(LobbySlot {
                faction,
                team,
                kind,
                label,
            });
        }
        Self {
            code,
            mode,
            phase: LobbyPhase::Waiting,
            slots,
            host_faction: FactionId(0),
            local_faction: FactionId(0),
            is_host: true,
            local_only,
        }
    }

    pub fn new_joiner(mode: MatchMode, code: String) -> Self {
        let mut lobby = Self::new_host(mode, code, false);
        lobby.is_host = false;
        lobby.local_only = false;
        // Tentative assignment — host will sync real slot via LobbySync
        lobby.local_faction = FactionId(1);
        if let Some(slot) = lobby.slots.get_mut(1) {
            slot.kind = SlotKind::Human;
            slot.label = "Toi".into();
        }
        if let Some(slot) = lobby.slots.get_mut(0) {
            slot.kind = SlotKind::Human;
            slot.label = "Host".into();
        }
        lobby
    }

    pub fn rebuild_for_mode(&mut self, mode: MatchMode) {
        if !self.is_host || self.phase != LobbyPhase::Waiting {
            return;
        }
        let code = self.code.clone();
        let local_only = self.local_only;
        let is_host = self.is_host;
        *self = if local_only {
            Self::new_local(mode)
        } else {
            Self::new_host(mode, code, false)
        };
        self.is_host = is_host;
    }

    pub fn toggle_slot_ai(&mut self, index: usize) {
        if !self.is_host || self.phase != LobbyPhase::Waiting {
            return;
        }
        let Some(slot) = self.slots.get_mut(index) else {
            return;
        };
        if index == 0 {
            return;
        }
        match slot.kind {
            SlotKind::Empty | SlotKind::Human => {
                slot.kind = SlotKind::Ai;
                slot.label = format!("IA {}", slot.faction.0);
            }
            SlotKind::Ai => {
                slot.kind = SlotKind::Empty;
                slot.label = "Libre".into();
            }
        }
    }

    pub fn claim_next_empty(&mut self, label: &str) -> Option<FactionId> {
        for slot in &mut self.slots {
            if slot.kind == SlotKind::Empty {
                slot.kind = SlotKind::Human;
                slot.label = label.into();
                return Some(slot.faction);
            }
        }
        None
    }

    /// Lock lobby: empty → AI. Returns false if invalid.
    pub fn lock_and_prepare(&mut self) -> bool {
        if !self.is_host || self.phase != LobbyPhase::Waiting {
            return false;
        }
        for slot in &mut self.slots {
            if slot.kind == SlotKind::Empty {
                slot.kind = SlotKind::Ai;
                slot.label = format!("IA {}", slot.faction.0);
            }
        }
        self.phase = LobbyPhase::Locked;
        true
    }

    pub fn mark_playing(&mut self) {
        self.phase = LobbyPhase::Playing;
    }

    pub fn to_match_config(&self) -> MatchConfig {
        let ai_slots: Vec<bool> = self.slots.iter().map(|s| s.kind == SlotKind::Ai).collect();
        MatchConfig {
            mode: self.mode,
            ai_slots,
            local_faction: self.local_faction,
        }
    }
}

pub fn mode_faction_count(mode: MatchMode) -> usize {
    match mode {
        MatchMode::Duel => 2,
        MatchMode::TwoVTwo => 4,
        MatchMode::FreeForAll { faction_count } => faction_count as usize,
    }
}

pub fn team_for_mode(mode: MatchMode, faction: FactionId) -> TeamId {
    match mode {
        MatchMode::Duel => TeamId(faction.0),
        MatchMode::TwoVTwo => TeamId(faction.0 / 2),
        MatchMode::FreeForAll { .. } => TeamId(faction.0),
    }
}

pub fn random_code() -> String {
    const ALPH: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(42);
    let mut n = t;
    let mut out = String::with_capacity(4);
    for _ in 0..4 {
        out.push(ALPH[(n % ALPH.len() as u128) as usize] as char);
        n /= ALPH.len() as u128;
        n ^= n << 7;
    }
    out
}

pub fn normalize_code(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .take(8)
        .collect()
}
