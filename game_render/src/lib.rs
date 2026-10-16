//! Bevy 2D visualization + in-game HUD.

use bevy::prelude::*;
use game_core::{
    faction_color, CapturePoint, FactionId, SimUnit, SimWorld, UnitType, VictoryState, MAP_HEIGHT,
    MAP_WIDTH,
};

pub struct GameRenderPlugin;

impl Plugin for GameRenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraDrag>()
            .init_resource::<LocalPlayer>()
            .init_resource::<LocalProductionIntent>()
            .init_resource::<VictoryHud>()
            .add_systems(
                Update,
                (
                    camera_controls.run_if(resource_exists::<GameSim>),
                    sync_world_visuals.run_if(resource_exists::<GameSim>),
                    update_hud.run_if(resource_exists::<GameSim>),
                    handle_production_keys.run_if(resource_exists::<GameSim>),
                ),
            );
    }
}

/// Bevy resource wrapping the pure simulation world.
#[derive(Resource)]
pub struct GameSim(pub SimWorld);

#[derive(Resource, Default)]
pub struct CameraDrag {
    pub dragging: bool,
    pub last_cursor: Vec2,
}

#[derive(Component)]
pub struct MainCamera;

#[derive(Component)]
pub struct UnitVisual {
    pub id: u32,
}

#[derive(Component)]
pub struct CastleVisual {
    pub faction: FactionId,
}

#[derive(Component)]
pub struct PointVisual {
    pub index: usize,
}

#[derive(Component)]
pub struct ObstacleVisual;

#[derive(Component)]
pub struct MapBackground;

#[derive(Component)]
pub struct HudRoot;

#[derive(Component)]
pub struct HudText;

#[derive(Resource, Clone, Debug)]
pub struct LocalPlayer {
    pub faction: FactionId,
}

impl Default for LocalPlayer {
    fn default() -> Self {
        Self {
            faction: FactionId(0),
        }
    }
}

#[derive(Resource, Default)]
pub struct LocalProductionIntent {
    pub produce: Option<UnitType>,
}

#[derive(Resource, Clone, Debug)]
pub struct VictoryHud(pub VictoryState);

impl Default for VictoryHud {
    fn default() -> Self {
        Self(VictoryState::Ongoing)
    }
}

pub fn spawn_match_visuals(commands: &mut Commands) {
    commands.spawn((
        Camera2d,
        MainCamera,
        Transform::from_xyz(MAP_WIDTH * 0.5, MAP_HEIGHT * 0.5, 1000.0),
    ));

    commands.spawn((
        MapBackground,
        Sprite {
            color: Color::srgb(0.12, 0.14, 0.16),
            custom_size: Some(Vec2::new(MAP_WIDTH, MAP_HEIGHT)),
            ..default()
        },
        Transform::from_xyz(MAP_WIDTH * 0.5, MAP_HEIGHT * 0.5, 0.0),
    ));

    commands
        .spawn((
            HudRoot,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(8.0),
                left: Val::Px(8.0),
                padding: UiRect::all(Val::Px(10.0)),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
        ))
        .with_children(|parent| {
            parent.spawn((
                HudText,
                Text::new("…"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

fn camera_controls(
    mut drag: ResMut<CameraDrag>,
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut scroll: MessageReader<bevy::input::mouse::MouseWheel>,
    windows: Query<&Window>,
    mut cameras: Query<&mut Transform, With<MainCamera>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Ok(mut transform) = cameras.single_mut() else {
        return;
    };

    for ev in scroll.read() {
        let zoom = 1.0 - ev.y * 0.08;
        transform.scale *= zoom;
        transform.scale = transform.scale.clamp(Vec3::splat(0.35), Vec3::splat(2.5));
    }

    if buttons.just_pressed(MouseButton::Right) || buttons.just_pressed(MouseButton::Middle) {
        drag.dragging = true;
        if let Some(pos) = window.cursor_position() {
            drag.last_cursor = pos;
        }
    }
    if buttons.just_released(MouseButton::Right) || buttons.just_released(MouseButton::Middle) {
        drag.dragging = false;
    }
    if drag.dragging {
        if let Some(pos) = window.cursor_position() {
            let delta = (pos - drag.last_cursor) * transform.scale.truncate();
            transform.translation.x -= delta.x;
            transform.translation.y += delta.y;
            drag.last_cursor = pos;
        }
    }

    let speed = 320.0 * transform.scale.x;
    let dt = 1.0 / 60.0;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        transform.translation.y += speed * dt;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        transform.translation.y -= speed * dt;
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        transform.translation.x -= speed * dt;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        transform.translation.x += speed * dt;
    }
}

fn handle_production_keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut intent: ResMut<LocalProductionIntent>,
) {
    if keys.just_pressed(KeyCode::Digit1) || keys.just_pressed(KeyCode::Numpad1) {
        intent.produce = Some(UnitType::Melee);
    }
    if keys.just_pressed(KeyCode::Digit2) || keys.just_pressed(KeyCode::Numpad2) {
        intent.produce = Some(UnitType::Ranged);
    }
    if keys.just_pressed(KeyCode::Digit3) || keys.just_pressed(KeyCode::Numpad3) {
        intent.produce = Some(UnitType::Tank);
    }
}

fn sync_world_visuals(
    mut commands: Commands,
    sim: Res<GameSim>,
    mut units_q: Query<(Entity, &UnitVisual, &mut Transform, &mut Sprite)>,
    mut castles_q: Query<(&CastleVisual, &mut Sprite)>,
    mut points_q: Query<(&PointVisual, &mut Sprite)>,
    mut spawned_static: Local<bool>,
    mut last_tick: Local<u32>,
) {
    let world = &sim.0;
    if world.tick < *last_tick {
        *spawned_static = false;
    }
    *last_tick = world.tick;

    if !*spawned_static {
        for &(x, y, w, h) in &world.obstacles {
            commands.spawn((
                ObstacleVisual,
                Sprite {
                    color: Color::srgb(0.28, 0.30, 0.34),
                    custom_size: Some(Vec2::new(w, h)),
                    ..default()
                },
                Transform::from_xyz(x, y, 1.0),
            ));
        }
        for (i, p) in world.points.iter().enumerate() {
            commands.spawn((
                PointVisual { index: i },
                Sprite {
                    color: Color::srgba(0.7, 0.7, 0.7, 0.35),
                    custom_size: Some(Vec2::splat(p.radius * 2.0)),
                    ..default()
                },
                Transform::from_xyz(p.x, p.y, 2.0),
            ));
        }
        for c in &world.castles {
            let col = faction_color(c.faction);
            commands.spawn((
                CastleVisual {
                    faction: c.faction,
                },
                Sprite {
                    color: Color::srgba(col[0], col[1], col[2], col[3]),
                    custom_size: Some(Vec2::new(36.0, 36.0)),
                    ..default()
                },
                Transform::from_xyz(c.x, c.y, 5.0),
            ));
        }
        *spawned_static = true;
    }

    for (vis, mut sprite) in castles_q.iter_mut() {
        if let Some(c) = world.castles.iter().find(|c| c.faction == vis.faction) {
            let col = faction_color(c.faction);
            let alive = if c.alive() { 1.0 } else { 0.25 };
            sprite.color = Color::srgba(col[0] * alive, col[1] * alive, col[2] * alive, 1.0);
            let scale = 0.7 + 0.3 * (c.hp / c.max_hp);
            sprite.custom_size = Some(Vec2::splat(36.0 * scale));
        }
    }

    for (vis, mut sprite) in points_q.iter_mut() {
        if let Some(p) = world.points.get(vis.index) {
            sprite.color = point_color(p);
        }
    }

    let mut seen = std::collections::HashSet::new();
    for (entity, vis, mut tf, mut sprite) in units_q.iter_mut() {
        if let Some(u) = world.units.iter().find(|u| u.id == vis.id) {
            seen.insert(u.id);
            tf.translation.x = u.x;
            tf.translation.y = u.y;
            let col = faction_color(u.faction);
            sprite.color = Color::srgba(col[0], col[1], col[2], 1.0);
            sprite.custom_size = Some(Vec2::splat(unit_size(u)));
        } else {
            commands.entity(entity).despawn();
        }
    }
    for u in &world.units {
        if !seen.contains(&u.id) {
            let col = faction_color(u.faction);
            commands.spawn((
                UnitVisual { id: u.id },
                Sprite {
                    color: Color::srgba(col[0], col[1], col[2], 1.0),
                    custom_size: Some(Vec2::splat(unit_size(u))),
                    ..default()
                },
                Transform::from_xyz(u.x, u.y, 10.0),
            ));
        }
    }
}

fn unit_size(u: &SimUnit) -> f32 {
    match u.unit_type {
        UnitType::Melee => 10.0,
        UnitType::Ranged => 9.0,
        UnitType::Tank => 14.0,
    }
}

fn point_color(p: &CapturePoint) -> Color {
    if let Some(owner) = p.owner {
        let c = faction_color(owner);
        Color::srgba(c[0], c[1], c[2], 0.45)
    } else if let Some(cap) = p.capturing {
        let c = faction_color(cap);
        let a = 0.25 + 0.35 * (p.progress / 100.0);
        Color::srgba(c[0], c[1], c[2], a)
    } else {
        Color::srgba(0.75, 0.75, 0.75, 0.3)
    }
}

fn update_hud(
    sim: Res<GameSim>,
    local: Res<LocalPlayer>,
    victory: Res<VictoryHud>,
    mut texts: Query<&mut Text, With<HudText>>,
) {
    let Ok(mut text) = texts.single_mut() else {
        return;
    };
    let world = &sim.0;
    let gold = world
        .gold
        .get(local.faction.index())
        .copied()
        .unwrap_or(0.0);
    let units = world
        .units
        .iter()
        .filter(|u| u.faction == local.faction)
        .count();
    let total_units = world.units.len();
    let team = world.team_of(local.faction);

    let mut status = format!(
        "Faction {} | Team {} | Gold {:.0} | Units {}/{} | Tick {}\n[1] Melee  [2] Ranged  [3] Tank  |  WASD pan  RMB drag  scroll zoom",
        local.faction.0, team.0, gold, units, total_units, world.tick
    );

    if let VictoryState::TeamWon(t) = victory.0 {
        status.push_str(&format!("\n*** TEAM {} WINS ***  (Esc = menu)", t.0));
    }

    *text = Text::new(status);
}
