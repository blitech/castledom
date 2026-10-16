//! Browser multiplayer via Matchbox (WebRTC) + lockstep + lobby sync.

use bevy::prelude::*;
use bevy_matchbox::prelude::*;
use game_core::{
    FactionId, GameInput, LobbyPhase, LobbyState, MatchMode, SlotKind, UnitType, MAX_FACTIONS,
};
use serde::{Deserialize, Serialize};

pub const DEFAULT_SIGNALING_HOST: &str = "ws://127.0.0.1:3536";
const CHANNEL_ID: usize = 0;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NetMessage {
    LobbySync(LobbyState),
    JoinRequest { name: String },
    SlotClaim { faction: u8, label: String },
    Ready { faction: u8 },
    TickInput {
        tick: u32,
        faction: u8,
        produce: Option<u8>,
    },
    StartMatch { seed: u32 },
}

impl NetMessage {
    pub fn encode(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        serde_json::from_slice(bytes).ok()
    }
}

pub fn encode_produce(p: Option<UnitType>) -> Option<u8> {
    p.map(|u| match u {
        UnitType::Melee => 0,
        UnitType::Ranged => 1,
        UnitType::Tank => 2,
    })
}

pub fn decode_produce(p: Option<u8>) -> Option<UnitType> {
    match p {
        Some(0) => Some(UnitType::Melee),
        Some(1) => Some(UnitType::Ranged),
        Some(2) => Some(UnitType::Tank),
        _ => None,
    }
}

#[derive(Resource, Clone, Debug)]
pub struct NetConfig {
    pub signaling_host: String,
    pub room_code: String,
}

impl Default for NetConfig {
    fn default() -> Self {
        Self {
            signaling_host: DEFAULT_SIGNALING_HOST.into(),
            room_code: "LOBBY".into(),
        }
    }
}

impl NetConfig {
    pub fn room_url(&self) -> String {
        let host = self.signaling_host.trim_end_matches('/');
        format!("{}/castledom_{}", host, self.room_code.to_ascii_lowercase())
    }
}

#[derive(Resource, Default)]
pub struct LockstepBuffer {
    pub pending: std::collections::BTreeMap<u32, [Option<GameInput>; MAX_FACTIONS]>,
    pub highest_confirmed: u32,
}

impl LockstepBuffer {
    pub fn insert(&mut self, tick: u32, faction: FactionId, input: GameInput) {
        let entry = self
            .pending
            .entry(tick)
            .or_insert_with(|| [None; MAX_FACTIONS]);
        if faction.index() < MAX_FACTIONS {
            entry[faction.index()] = Some(input);
        }
    }

    pub fn take_if_ready(
        &mut self,
        tick: u32,
        factions: &[FactionId],
    ) -> Option<[GameInput; MAX_FACTIONS]> {
        let entry = self.pending.get(&tick)?;
        for f in factions {
            if entry.get(f.index()).and_then(|x| *x).is_none() {
                return None;
            }
        }
        let mut out = [GameInput::idle(); MAX_FACTIONS];
        if let Some(entry) = self.pending.remove(&tick) {
            for (i, slot) in entry.iter().enumerate() {
                if let Some(inp) = slot {
                    out[i] = *inp;
                }
            }
            self.highest_confirmed = tick;
            Some(out)
        } else {
            None
        }
    }
}

#[derive(Resource, Default)]
pub struct PeerRegistry {
    pub peers: Vec<PeerId>,
    pub connected: usize,
}

#[derive(Message, Clone, Debug)]
pub struct RemoteLobbySync(pub LobbyState);

#[derive(Message, Clone, Debug)]
pub struct RemoteStartMatch {
    pub seed: u32,
}

#[derive(Message, Clone, Debug)]
pub struct RemoteJoinRequest {
    pub name: String,
    pub from: PeerId,
}

#[derive(Message, Clone, Debug)]
pub struct RemoteSlotClaim {
    pub faction: FactionId,
    pub label: String,
}

pub struct GameNetPlugin;

impl Plugin for GameNetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LockstepBuffer>()
            .init_resource::<PeerRegistry>()
            .add_message::<RemoteLobbySync>()
            .add_message::<RemoteStartMatch>()
            .add_message::<RemoteJoinRequest>()
            .add_message::<RemoteSlotClaim>()
            .add_systems(Update, (receive_net_messages, track_peers));
    }
}

pub fn open_socket(commands: &mut Commands, config: &NetConfig) {
    let url = config.room_url();
    info!("Connecting Matchbox room: {url}");
    let socket = MatchboxSocket::new_reliable(url);
    commands.insert_resource(socket);
}

fn track_peers(mut socket: Option<ResMut<MatchboxSocket>>, mut registry: ResMut<PeerRegistry>) {
    let Some(socket) = socket.as_mut() else {
        return;
    };
    for (peer, state) in socket.update_peers() {
        match state {
            PeerState::Connected => {
                info!("Peer connected: {peer}");
                if !registry.peers.contains(&peer) {
                    registry.peers.push(peer);
                }
                registry.connected = registry.peers.len();
            }
            PeerState::Disconnected => {
                info!("Peer disconnected: {peer}");
                registry.peers.retain(|p| *p != peer);
                registry.connected = registry.peers.len();
            }
        }
    }
}

fn receive_net_messages(
    mut socket: Option<ResMut<MatchboxSocket>>,
    mut buffer: ResMut<LockstepBuffer>,
    mut lobby_sync: MessageWriter<RemoteLobbySync>,
    mut start_match: MessageWriter<RemoteStartMatch>,
    mut join_req: MessageWriter<RemoteJoinRequest>,
    mut slot_claim: MessageWriter<RemoteSlotClaim>,
) {
    let Some(socket) = socket.as_mut() else {
        return;
    };
    for (peer, packet) in socket.channel_mut(CHANNEL_ID).receive() {
        let Some(msg) = NetMessage::decode(&packet) else {
            continue;
        };
        match msg {
            NetMessage::TickInput {
                tick,
                faction,
                produce,
            } => {
                buffer.insert(
                    tick,
                    FactionId(faction),
                    GameInput {
                        produce: decode_produce(produce),
                    },
                );
            }
            NetMessage::LobbySync(state) => {
                lobby_sync.write(RemoteLobbySync(state));
            }
            NetMessage::StartMatch { seed } => {
                start_match.write(RemoteStartMatch { seed });
            }
            NetMessage::JoinRequest { name } => {
                join_req.write(RemoteJoinRequest { name, from: peer });
            }
            NetMessage::SlotClaim { faction, label } => {
                slot_claim.write(RemoteSlotClaim {
                    faction: FactionId(faction),
                    label,
                });
            }
            NetMessage::Ready { .. } => {}
        }
    }
}

pub fn broadcast(socket: &mut MatchboxSocket, msg: &NetMessage) {
    let bytes = msg.encode();
    let peers: Vec<_> = socket.connected_peers().collect();
    for peer in peers {
        socket
            .channel_mut(CHANNEL_ID)
            .send(bytes.clone().into(), peer);
    }
}

pub fn send_tick_input(
    socket: &mut MatchboxSocket,
    tick: u32,
    faction: FactionId,
    input: GameInput,
) {
    let msg = NetMessage::TickInput {
        tick,
        faction: faction.0,
        produce: encode_produce(input.produce),
    };
    broadcast(socket, &msg);
}

pub fn mode_to_u8(mode: MatchMode) -> u8 {
    match mode {
        MatchMode::Duel => 0,
        MatchMode::TwoVTwo => 1,
        MatchMode::FreeForAll { .. } => 2,
    }
}

pub fn describe_slot(kind: SlotKind) -> &'static str {
    match kind {
        SlotKind::Empty => "Libre",
        SlotKind::Human => "Humain",
        SlotKind::Ai => "IA",
    }
}

pub fn phase_label(phase: LobbyPhase) -> &'static str {
    match phase {
        LobbyPhase::Waiting => "En attente",
        LobbyPhase::Locked => "Verrouillé",
        LobbyPhase::Playing => "En jeu",
    }
}
