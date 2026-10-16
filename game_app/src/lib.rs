//! Castledom — home → Cémantix lobby → match (solo or Matchbox).

mod lobby_ui;

use bevy::prelude::*;
use bevy_matchbox::prelude::MatchboxSocket;
use game_core::{
    normalize_code, random_code, AiController, FactionId, GameInput, InputBus, LobbyPhase,
    LobbyState, MatchMode, SimWorld, SlotKind, TickResult, VictoryState, TICK_DT,
};
use game_net::{
    broadcast, open_socket, send_tick_input, GameNetPlugin, LockstepBuffer, NetConfig, NetMessage,
    PeerRegistry, RemoteJoinRequest, RemoteLobbySync, RemoteSlotClaim, RemoteStartMatch,
};
use game_render::{
    spawn_match_visuals, CastleVisual, GameRenderPlugin, GameSim, HudRoot, LocalPlayer,
    LocalProductionIntent, MainCamera, MapBackground, ObstacleVisual, PointVisual, UnitVisual,
    VictoryHud,
};
use lobby_ui::{
    setup_home, setup_lobby_screen, HomeAction, JoinCodeBuffer, LobbyAction, UiRoot,
};

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default]
    Home,
    Lobby,
    Playing,
}

#[derive(Resource, Default)]
pub struct SimClock {
    pub accumulator: f32,
}

#[derive(Resource)]
pub struct AiSet {
    pub controllers: Vec<AiController>,
}

#[derive(Resource)]
pub struct ActiveLobby(pub LobbyState);

#[derive(Resource, Default)]
pub struct SimInputs(pub InputBus);

pub fn run() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    let mut join_buf = JoinCodeBuffer::default();
    if let Some(code) = read_room_from_url() {
        join_buf.text = code;
    }

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Castledom".into(),
                        resolution: (960, 640).into(),
                        canvas: Some("#game-canvas".into()),
                        fit_canvas_to_parent: true,
                        prevent_default_event_handling: true,
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .init_state::<AppState>()
        .init_resource::<SimClock>()
        .insert_resource(join_buf)
        .insert_resource(ClearColor(Color::srgb(0.08, 0.09, 0.11)))
        .add_plugins(GameRenderPlugin)
        .add_plugins(GameNetPlugin)
        .add_systems(Startup, |mut commands: Commands| {
            setup_home(&mut commands);
        })
        .add_systems(
            Update,
            (home_buttons, join_code_typing).run_if(in_state(AppState::Home)),
        )
        .add_systems(OnEnter(AppState::Lobby), enter_lobby)
        .add_systems(
            Update,
            (
                lobby_buttons,
                handle_remote_lobby,
                handle_join_requests,
                handle_slot_claims,
                handle_remote_start,
                host_broadcast_lobby,
            )
                .run_if(in_state(AppState::Lobby)),
        )
        .add_systems(OnExit(AppState::Lobby), cleanup_lobby_ui)
        .add_systems(OnEnter(AppState::Playing), start_playing)
        .add_systems(
            Update,
            (drive_simulation, playing_escape).run_if(in_state(AppState::Playing)),
        )
        .add_systems(OnExit(AppState::Playing), cleanup_playing)
        .run();
}

fn read_room_from_url() -> Option<String> {
    std::env::var("ROOM")
        .ok()
        .map(|c| normalize_code(&c))
        .filter(|c| !c.is_empty())
}

fn home_buttons(
    mut commands: Commands,
    mut next: ResMut<NextState<AppState>>,
    join: Res<JoinCodeBuffer>,
    mut interaction: Query<
        (&Interaction, &HomeAction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, action, mut bg) in interaction.iter_mut() {
        match *interaction {
            Interaction::Pressed => match *action {
                HomeAction::CreateDuel => {
                    let code = random_code();
                    commands.insert_resource(ActiveLobby(LobbyState::new_host(
                        MatchMode::Duel,
                        code,
                        false,
                    )));
                    next.set(AppState::Lobby);
                }
                HomeAction::CreateTwoVTwo => {
                    let code = random_code();
                    commands.insert_resource(ActiveLobby(LobbyState::new_host(
                        MatchMode::TwoVTwo,
                        code,
                        false,
                    )));
                    next.set(AppState::Lobby);
                }
                HomeAction::SoloDuel => {
                    commands.insert_resource(ActiveLobby(LobbyState::new_local(MatchMode::Duel)));
                    next.set(AppState::Lobby);
                }
                HomeAction::SoloTwoVTwo => {
                    commands
                        .insert_resource(ActiveLobby(LobbyState::new_local(MatchMode::TwoVTwo)));
                    next.set(AppState::Lobby);
                }
                HomeAction::Join => {
                    let code = normalize_code(&join.text);
                    if code.len() >= 4 {
                        commands.insert_resource(ActiveLobby(LobbyState::new_joiner(
                            MatchMode::Duel,
                            code,
                        )));
                        next.set(AppState::Lobby);
                    }
                }
            },
            Interaction::Hovered => *bg = BackgroundColor(Color::srgb(0.25, 0.45, 0.75)),
            Interaction::None => *bg = BackgroundColor(Color::srgb(0.18, 0.32, 0.55)),
        }
    }
}

fn join_code_typing(
    keys: Res<ButtonInput<KeyCode>>,
    mut join: ResMut<JoinCodeBuffer>,
    mut labels: Query<&mut Text, With<lobby_ui::JoinCodeLabel>>,
) {
    for key in keys.get_just_pressed() {
        let ch = match *key {
            KeyCode::KeyA => Some('A'),
            KeyCode::KeyB => Some('B'),
            KeyCode::KeyC => Some('C'),
            KeyCode::KeyD => Some('D'),
            KeyCode::KeyE => Some('E'),
            KeyCode::KeyF => Some('F'),
            KeyCode::KeyG => Some('G'),
            KeyCode::KeyH => Some('H'),
            KeyCode::KeyJ => Some('J'),
            KeyCode::KeyK => Some('K'),
            KeyCode::KeyL => Some('L'),
            KeyCode::KeyM => Some('M'),
            KeyCode::KeyN => Some('N'),
            KeyCode::KeyP => Some('P'),
            KeyCode::KeyQ => Some('Q'),
            KeyCode::KeyR => Some('R'),
            KeyCode::KeyS => Some('S'),
            KeyCode::KeyT => Some('T'),
            KeyCode::KeyU => Some('U'),
            KeyCode::KeyV => Some('V'),
            KeyCode::KeyW => Some('W'),
            KeyCode::KeyX => Some('X'),
            KeyCode::KeyY => Some('Y'),
            KeyCode::KeyZ => Some('Z'),
            KeyCode::Digit2 => Some('2'),
            KeyCode::Digit3 => Some('3'),
            KeyCode::Digit4 => Some('4'),
            KeyCode::Digit5 => Some('5'),
            KeyCode::Digit6 => Some('6'),
            KeyCode::Digit7 => Some('7'),
            KeyCode::Digit8 => Some('8'),
            KeyCode::Digit9 => Some('9'),
            KeyCode::Backspace => {
                join.text.pop();
                None
            }
            _ => None,
        };
        if let Some(c) = ch {
            if join.text.len() < 8 {
                join.text.push(c);
            }
        }
    }
    if let Ok(mut text) = labels.single_mut() {
        *text = Text::new(format!(
            "Code à rejoindre: {} (tape le code puis Rejoindre)",
            if join.text.is_empty() {
                "____"
            } else {
                &join.text
            }
        ));
    }
}

fn enter_lobby(
    mut commands: Commands,
    lobby: Option<Res<ActiveLobby>>,
    roots: Query<Entity, With<UiRoot>>,
) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
    let Some(lobby) = lobby else {
        return;
    };
    setup_lobby_screen(&mut commands, &lobby.0);

    if !lobby.0.local_only {
        let net = NetConfig {
            room_code: lobby.0.code.clone(),
            ..default()
        };
        commands.insert_resource(net.clone());
        open_socket(&mut commands, &net);

        if !lobby.0.is_host {
            commands.insert_resource(PendingJoinAnnounce);
        }
    }
}

#[derive(Resource)]
struct PendingJoinAnnounce;

fn cleanup_lobby_ui(mut commands: Commands, roots: Query<Entity, With<UiRoot>>) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

fn lobby_buttons(
    mut commands: Commands,
    mut lobby: ResMut<ActiveLobby>,
    mut next: ResMut<NextState<AppState>>,
    mut socket: Option<ResMut<MatchboxSocket>>,
    roots: Query<Entity, With<UiRoot>>,
    mut interaction: Query<
        (&Interaction, &LobbyAction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, action, mut bg) in interaction.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                match *action {
                    LobbyAction::ModeDuel => {
                        lobby.0.rebuild_for_mode(MatchMode::Duel);
                        rebuild_lobby_ui(&mut commands, &roots, &lobby.0);
                    }
                    LobbyAction::ModeTwoVTwo => {
                        lobby.0.rebuild_for_mode(MatchMode::TwoVTwo);
                        rebuild_lobby_ui(&mut commands, &roots, &lobby.0);
                    }
                    LobbyAction::ToggleAi { index } => {
                        lobby.0.toggle_slot_ai(index);
                        rebuild_lobby_ui(&mut commands, &roots, &lobby.0);
                    }
                    LobbyAction::Start => {
                        if lobby.0.local_only {
                            if lobby.0.lock_and_prepare() {
                                lobby.0.mark_playing();
                                next.set(AppState::Playing);
                            }
                        } else if lobby.0.is_host && lobby.0.lock_and_prepare() {
                            if let Some(socket) = socket.as_mut() {
                                broadcast(socket, &NetMessage::LobbySync(lobby.0.clone()));
                                broadcast(
                                    socket,
                                    &NetMessage::StartMatch {
                                        seed: lobby.0.code.len() as u32,
                                    },
                                );
                            }
                            lobby.0.mark_playing();
                            next.set(AppState::Playing);
                        }
                    }
                    LobbyAction::Back => {
                        commands.remove_resource::<ActiveLobby>();
                        commands.remove_resource::<MatchboxSocket>();
                        next.set(AppState::Home);
                        for e in roots.iter() {
                            commands.entity(e).despawn();
                        }
                        setup_home(&mut commands);
                    }
                }
            }
            Interaction::Hovered => *bg = BackgroundColor(Color::srgb(0.25, 0.45, 0.75)),
            Interaction::None => *bg = BackgroundColor(Color::srgb(0.18, 0.32, 0.55)),
        }
    }
}

fn rebuild_lobby_ui(
    commands: &mut Commands,
    roots: &Query<Entity, With<UiRoot>>,
    lobby: &LobbyState,
) {
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
    setup_lobby_screen(commands, lobby);
}

fn host_broadcast_lobby(
    lobby: Res<ActiveLobby>,
    mut socket: Option<ResMut<MatchboxSocket>>,
    peers: Option<Res<PeerRegistry>>,
    pending: Option<Res<PendingJoinAnnounce>>,
    mut commands: Commands,
    mut last_count: Local<usize>,
) {
    let Some(socket) = socket.as_mut() else {
        return;
    };
    let connected = peers.map(|p| p.connected).unwrap_or(0);

    if pending.is_some() && connected > 0 {
        broadcast(
            socket,
            &NetMessage::JoinRequest {
                name: "Joueur".into(),
            },
        );
        commands.remove_resource::<PendingJoinAnnounce>();
    }

    if lobby.0.is_host && !lobby.0.local_only && connected != *last_count {
        *last_count = connected;
        broadcast(socket, &NetMessage::LobbySync(lobby.0.clone()));
    }
}

fn handle_remote_lobby(
    mut commands: Commands,
    mut events: MessageReader<RemoteLobbySync>,
    mut lobby: Option<ResMut<ActiveLobby>>,
    roots: Query<Entity, With<UiRoot>>,
) {
    for ev in events.read() {
        let local_faction = lobby
            .as_ref()
            .map(|l| l.0.local_faction)
            .unwrap_or(FactionId(0));
        let is_host = lobby.as_ref().map(|l| l.0.is_host).unwrap_or(false);
        let mut state = ev.0.clone();
        state.local_faction = local_faction;
        state.is_host = is_host;
        state.local_only = false;
        if let Some(ref mut lobby) = lobby {
            lobby.0 = state.clone();
        } else {
            commands.insert_resource(ActiveLobby(state.clone()));
        }
        for e in roots.iter() {
            commands.entity(e).despawn();
        }
        setup_lobby_screen(&mut commands, &state);
    }
}

fn handle_join_requests(
    mut lobby: ResMut<ActiveLobby>,
    mut events: MessageReader<RemoteJoinRequest>,
    mut socket: Option<ResMut<MatchboxSocket>>,
    mut commands: Commands,
    roots: Query<Entity, With<UiRoot>>,
) {
    if !lobby.0.is_host {
        return;
    }
    for ev in events.read() {
        if let Some(faction) = lobby.0.claim_next_empty(&ev.name) {
            if let Some(socket) = socket.as_mut() {
                broadcast(
                    socket,
                    &NetMessage::SlotClaim {
                        faction: faction.0,
                        label: ev.name.clone(),
                    },
                );
                broadcast(socket, &NetMessage::LobbySync(lobby.0.clone()));
            }
            for e in roots.iter() {
                commands.entity(e).despawn();
            }
            setup_lobby_screen(&mut commands, &lobby.0);
        }
    }
}

fn handle_slot_claims(
    mut lobby: ResMut<ActiveLobby>,
    mut events: MessageReader<RemoteSlotClaim>,
    mut commands: Commands,
    roots: Query<Entity, With<UiRoot>>,
) {
    let mut changed = false;
    for ev in events.read() {
        if let Some(slot) = lobby.0.slots.iter_mut().find(|s| s.faction == ev.faction) {
            slot.kind = SlotKind::Human;
            slot.label = ev.label.clone();
            changed = true;
        }
        if !lobby.0.is_host {
            lobby.0.local_faction = ev.faction;
        }
    }
    if changed {
        for e in roots.iter() {
            commands.entity(e).despawn();
        }
        setup_lobby_screen(&mut commands, &lobby.0);
    }
}

fn handle_remote_start(
    mut events: MessageReader<RemoteStartMatch>,
    mut lobby: ResMut<ActiveLobby>,
    mut next: ResMut<NextState<AppState>>,
) {
    for _ev in events.read() {
        if !lobby.0.is_host {
            lobby.0.phase = LobbyPhase::Playing;
            next.set(AppState::Playing);
        }
    }
}

fn start_playing(mut commands: Commands, lobby: Res<ActiveLobby>) {
    let config = lobby.0.to_match_config();
    let n = config.faction_count();
    commands.insert_resource(LocalPlayer {
        faction: config.local_faction,
    });
    commands.insert_resource(AiSet {
        controllers: (0..n).map(|_| AiController::new()).collect(),
    });
    commands.insert_resource(VictoryHud(VictoryState::Ongoing));
    commands.insert_resource(GameSim(SimWorld::new(config)));
    commands.insert_resource(SimClock::default());
    commands.insert_resource(SimInputs(InputBus::default()));
    spawn_match_visuals(&mut commands);
}

fn drive_simulation(
    time: Res<Time>,
    mut clock: ResMut<SimClock>,
    mut sim: ResMut<GameSim>,
    mut bus: ResMut<SimInputs>,
    mut ai: ResMut<AiSet>,
    mut intent: ResMut<LocalProductionIntent>,
    mut victory: ResMut<VictoryHud>,
    local: Res<LocalPlayer>,
    lobby: Option<Res<ActiveLobby>>,
    mut socket: Option<ResMut<MatchboxSocket>>,
    mut buffer: ResMut<LockstepBuffer>,
) {
    if matches!(victory.0, VictoryState::TeamWon(_)) {
        return;
    }

    let local_only = lobby.as_ref().map(|l| l.0.local_only).unwrap_or(true);
    let is_host = lobby.as_ref().map(|l| l.0.is_host).unwrap_or(true);
    let world = &mut sim.0;

    clock.accumulator += time.delta_secs();
    let mut steps = 0;
    while clock.accumulator >= TICK_DT && steps < 4 {
        clock.accumulator -= TICK_DT;
        steps += 1;
        bus.0.clear();

        let local_input = GameInput {
            produce: intent.produce.take(),
        };

        if local_only {
            bus.0.set(local.faction, local_input);
            for i in 0..world.config.faction_count() {
                let f = FactionId(i as u8);
                if world.config.is_ai(f) {
                    let decision = ai.controllers[i].decide(world, f);
                    bus.0.set(f, decision);
                }
            }
        } else {
            let tick = world.tick + 1;
            buffer.insert(tick, local.faction, local_input);
            if let Some(socket) = socket.as_mut() {
                send_tick_input(socket, tick, local.faction, local_input);
            }

            if is_host {
                for i in 0..world.config.faction_count() {
                    let f = FactionId(i as u8);
                    if world.config.is_ai(f) {
                        let decision = ai.controllers[i].decide(world, f);
                        buffer.insert(tick, f, decision);
                        if let Some(socket) = socket.as_mut() {
                            send_tick_input(socket, tick, f, decision);
                        }
                    }
                }
            }

            let all: Vec<FactionId> = (0..world.config.faction_count())
                .map(|i| FactionId(i as u8))
                .collect();

            if let Some(inputs) = buffer.take_if_ready(tick, &all) {
                for (i, inp) in inputs.iter().enumerate() {
                    bus.0.set(FactionId(i as u8), *inp);
                }
            } else {
                clock.accumulator += TICK_DT;
                break;
            }
        }

        let TickResult { victory: v, .. } = world.tick(&bus.0);
        victory.0 = v;
    }
}

fn playing_escape(
    keys: Res<ButtonInput<KeyCode>>,
    mut next: ResMut<NextState<AppState>>,
    mut commands: Commands,
) {
    if keys.just_pressed(KeyCode::Escape) {
        commands.remove_resource::<ActiveLobby>();
        commands.remove_resource::<MatchboxSocket>();
        next.set(AppState::Home);
        setup_home(&mut commands);
    }
}

fn cleanup_playing(
    mut commands: Commands,
    q: Query<
        Entity,
        Or<(
            With<MainCamera>,
            With<MapBackground>,
            With<HudRoot>,
            With<UnitVisual>,
            With<CastleVisual>,
            With<PointVisual>,
            With<ObstacleVisual>,
        )>,
    >,
) {
    for e in q.iter() {
        commands.entity(e).despawn();
    }
    commands.remove_resource::<GameSim>();
    commands.remove_resource::<SimInputs>();
    commands.remove_resource::<AiSet>();
}
