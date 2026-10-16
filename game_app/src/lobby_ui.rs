//! Home + Cémantix-style lobby UI.

use bevy::prelude::*;
use game_core::{LobbyState, MatchMode, SlotKind};

#[derive(Component)]
pub struct UiRoot;

/// Caméra 2D du menu / lobby (Bevy UI ne s’affiche pas sans caméra).
#[derive(Component)]
pub struct MenuCamera;

#[derive(Component)]
pub struct JoinCodeLabel;

#[derive(Resource, Default, Clone)]
pub struct JoinCodeBuffer {
    pub text: String,
}

#[derive(Component, Clone, Copy)]
pub enum HomeAction {
    SoloDuel,
    SoloTwoVTwo,
    CreateDuel,
    CreateTwoVTwo,
    Join,
}

#[derive(Component, Clone, Copy)]
pub enum LobbyAction {
    ModeDuel,
    ModeTwoVTwo,
    ToggleAi { index: usize },
    Start,
    Back,
}

pub fn spawn_menu_camera(commands: &mut Commands) {
    commands.spawn((Camera2d, MenuCamera));
}

pub fn setup_home(commands: &mut Commands) {
    commands
        .spawn((
            UiRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.06, 0.08, 0.94)),
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("Castledom"),
                TextFont {
                    font_size: 32.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            p.spawn((
                Text::new("Castle Wars × Domination Wars — lobby type Cémantix"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.75, 0.78, 0.82)),
            ));

            spawn_btn(p, HomeAction::SoloDuel, "Solo 1v1 (lobby local + IA)");
            spawn_btn(p, HomeAction::SoloTwoVTwo, "Solo 2v2 (lobby local + IA)");
            spawn_btn(p, HomeAction::CreateDuel, "Créer salon 1v1 (code)");
            spawn_btn(p, HomeAction::CreateTwoVTwo, "Créer salon 2v2 (code)");
            spawn_btn(p, HomeAction::Join, "Rejoindre salon");

            p.spawn((
                JoinCodeLabel,
                Text::new("Code à rejoindre: ____ (tape le code puis Rejoindre)"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.85, 0.4)),
            ));

            p.spawn((
                Text::new("Réseau: cargo run -p signaling  |  ?room=CODE dans l’URL"),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.6, 0.65, 0.7)),
            ));
        });
}

pub fn setup_lobby_screen(commands: &mut Commands, lobby: &LobbyState) {
    let mode_label = match lobby.mode {
        MatchMode::Duel => "1v1",
        MatchMode::TwoVTwo => "2v2",
        MatchMode::FreeForAll { .. } => "FFA",
    };
    let title = if lobby.local_only {
        format!("Lobby local — {mode_label}")
    } else {
        format!("Salon {} — {mode_label}", lobby.code)
    };

    commands
        .spawn((
            UiRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.06, 0.08, 0.94)),
        ))
        .with_children(|p| {
            p.spawn((
                Text::new(title),
                TextFont {
                    font_size: 26.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));

            if !lobby.local_only {
                p.spawn((
                    Text::new(format!(
                        "Partage le code {}  (ou ?room={})",
                        lobby.code, lobby.code
                    )),
                    TextFont {
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.95, 0.85, 0.35)),
                ));
            }

            if lobby.is_host {
                spawn_lobby_btn(p, LobbyAction::ModeDuel, "Mode 1v1");
                spawn_lobby_btn(p, LobbyAction::ModeTwoVTwo, "Mode 2v2");
            }

            for (i, slot) in lobby.slots.iter().enumerate() {
                let kind = match slot.kind {
                    SlotKind::Empty => "Libre",
                    SlotKind::Human => "Humain",
                    SlotKind::Ai => "IA",
                };
                let line = format!(
                    "Slot {} | Équipe {} | {} — {}",
                    slot.faction.0, slot.team.0, kind, slot.label
                );
                p.spawn((
                    Text::new(line),
                    TextFont {
                        font_size: 15.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.85, 0.88, 0.92)),
                ));
                if lobby.is_host && i > 0 && lobby.phase == game_core::LobbyPhase::Waiting {
                    spawn_lobby_btn(
                        p,
                        LobbyAction::ToggleAi { index: i },
                        &format!("Toggle IA slot {i}"),
                    );
                }
            }

            if lobby.is_host {
                spawn_lobby_btn(p, LobbyAction::Start, "Verrouiller & Lancer");
            } else {
                p.spawn((
                    Text::new("En attente que le host lance…"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.7, 0.75, 0.8)),
                ));
            }

            spawn_lobby_btn(p, LobbyAction::Back, "Retour");
        });
}

fn spawn_btn(parent: &mut ChildSpawnerCommands, action: HomeAction, label: &str) {
    parent
        .spawn((
            Button,
            action,
            Node {
                width: Val::Px(380.0),
                height: Val::Px(38.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.18, 0.32, 0.55)),
        ))
        .with_children(|b| {
            b.spawn((
                Text::new(label),
                TextFont {
                    font_size: 15.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

fn spawn_lobby_btn(parent: &mut ChildSpawnerCommands, action: LobbyAction, label: &str) {
    parent
        .spawn((
            Button,
            action,
            Node {
                width: Val::Px(320.0),
                height: Val::Px(34.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.18, 0.32, 0.55)),
        ))
        .with_children(|b| {
            b.spawn((
                Text::new(label),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}
