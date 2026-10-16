//! Bevy 2D visualization + in-game HUD (resources, build drag-and-drop).

mod pixel_art;

pub use pixel_art::PixelAtlas;

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_core::{
    faction_color, footprint_center, world_to_tile, BuildingKind, CapturePoint, FactionId,
    PlaceBuildingCmd, SimWorld, VictoryState, MAP_HEIGHT, MAP_WIDTH, TILE_SIZE,
};
use pixel_art::{building_display_size, build_atlas, castle_display_size, unit_display_size};

pub struct GameRenderPlugin;

impl Plugin for GameRenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraDrag>()
            .init_resource::<LocalPlayer>()
            .init_resource::<LocalPlaceIntent>()
            .init_resource::<BuildDrag>()
            .init_resource::<VictoryHud>()
            .add_systems(Startup, setup_pixel_atlas)
            .add_systems(
                Update,
                (
                    camera_controls.run_if(resource_exists::<GameSim>),
                    handle_build_drag.run_if(resource_exists::<GameSim>),
                    update_ghost.run_if(resource_exists::<GameSim>),
                    sync_world_visuals.run_if(resource_exists::<GameSim>),
                    update_hud.run_if(resource_exists::<GameSim>),
                ),
            );
    }
}

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
pub struct BuildingVisual {
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

#[derive(Component)]
pub struct BuildBarRoot;

#[derive(Component)]
pub struct BuildButton {
    pub kind: BuildingKind,
}

#[derive(Component)]
pub struct GhostPreview;

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
pub struct LocalPlaceIntent {
    pub place: Option<PlaceBuildingCmd>,
}

#[derive(Resource, Default)]
pub struct BuildDrag {
    pub kind: Option<BuildingKind>,
    pub tile_x: i16,
    pub tile_y: i16,
    pub valid: bool,
}

#[derive(Resource, Clone, Debug)]
pub struct VictoryHud(pub VictoryState);

impl Default for VictoryHud {
    fn default() -> Self {
        Self(VictoryState::Ongoing)
    }
}

fn setup_pixel_atlas(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    commands.insert_resource(build_atlas(&mut images));
}

pub fn spawn_match_visuals(commands: &mut Commands, atlas: &PixelAtlas) {
    commands.spawn((
        Camera2d,
        MainCamera,
        Transform::from_xyz(MAP_WIDTH * 0.5, MAP_HEIGHT * 0.5, 1000.0),
    ));

    commands.spawn((
        MapBackground,
        Sprite {
            color: Color::srgb(0.14, 0.18, 0.14),
            custom_size: Some(Vec2::new(MAP_WIDTH, MAP_HEIGHT)),
            ..default()
        },
        Transform::from_xyz(MAP_WIDTH * 0.5, MAP_HEIGHT * 0.5, 0.0),
    ));

    // Subtle tile grid
    for tx in 0..30 {
        for ty in 0..20 {
            if (tx + ty) % 2 == 0 {
                continue;
            }
            let x = (tx as f32 + 0.5) * TILE_SIZE;
            let y = (ty as f32 + 0.5) * TILE_SIZE;
            commands.spawn((
                MapBackground,
                Sprite {
                    color: Color::srgba(0.16, 0.20, 0.16, 0.35),
                    custom_size: Some(Vec2::splat(TILE_SIZE)),
                    ..default()
                },
                Transform::from_xyz(x, y, 0.1),
            ));
        }
    }

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
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
        ))
        .with_children(|parent| {
            parent.spawn((
                HudText,
                Text::new("…"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.95, 0.9, 0.55)),
            ));
        });

    commands
        .spawn((
            BuildBarRoot,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(12.0),
                left: Val::Px(12.0),
                right: Val::Px(12.0),
                height: Val::Px(88.0),
                padding: UiRect::all(Val::Px(8.0)),
                column_gap: Val::Px(10.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.06, 0.08, 0.75)),
        ))
        .with_children(|parent| {
            for kind in BuildingKind::all() {
                spawn_build_button(parent, kind, atlas);
            }
        });

    commands.spawn((
        GhostPreview,
        Sprite {
            image: atlas.barracks.clone(),
            custom_size: Some(building_display_size(BuildingKind::Barracks)),
            color: Color::srgba(1.0, 1.0, 1.0, 0.0),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 50.0),
        Visibility::Hidden,
    ));
}

fn spawn_build_button(parent: &mut ChildSpawnerCommands, kind: BuildingKind, atlas: &PixelAtlas) {
    parent
        .spawn((
            BuildButton { kind },
            Button,
            Node {
                width: Val::Px(72.0),
                height: Val::Px(72.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BorderColor::all(Color::srgb(0.35, 0.4, 0.45)),
            BackgroundColor(Color::srgb(0.12, 0.14, 0.16)),
        ))
        .with_children(|b| {
            b.spawn((
                ImageNode {
                    image: atlas.building(kind),
                    ..default()
                },
                Node {
                    width: Val::Px(40.0),
                    height: Val::Px(40.0),
                    ..default()
                },
            ));
            b.spawn((
                Text::new(format!("{}\n{}", kind.label(), kind.cost() as i32)),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.85)),
                Node {
                    margin: UiRect::top(Val::Px(2.0)),
                    ..default()
                },
            ));
        });
}

fn camera_controls(
    mut drag: ResMut<CameraDrag>,
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut scroll: MessageReader<bevy::input::mouse::MouseWheel>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut cameras: Query<&mut Transform, With<MainCamera>>,
    build_drag: Res<BuildDrag>,
) {
    if build_drag.kind.is_some() {
        return;
    }
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

fn handle_build_drag(
    mut drag: ResMut<BuildDrag>,
    mut intent: ResMut<LocalPlaceIntent>,
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    sim: Res<GameSim>,
    local: Res<LocalPlayer>,
    atlas: Res<PixelAtlas>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut btn_q: Query<(&Interaction, &BuildButton, &mut BackgroundColor), Changed<Interaction>>,
    mut ghost: Query<(&mut Sprite, &mut Transform, &mut Visibility), With<GhostPreview>>,
) {
    for (interaction, btn, mut bg) in btn_q.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                drag.kind = Some(btn.kind);
                *bg = BackgroundColor(Color::srgb(0.3, 0.45, 0.25));
                if let Ok((mut sprite, _, mut vis)) = ghost.single_mut() {
                    sprite.image = atlas.building(btn.kind);
                    sprite.custom_size = Some(building_display_size(btn.kind));
                    *vis = Visibility::Visible;
                }
            }
            Interaction::Hovered => {
                if drag.kind.is_none() {
                    *bg = BackgroundColor(Color::srgb(0.2, 0.24, 0.28));
                }
            }
            Interaction::None => {
                *bg = BackgroundColor(Color::srgb(0.12, 0.14, 0.16));
            }
        }
    }

    if keys.just_pressed(KeyCode::Escape) && drag.kind.is_some() {
        drag.kind = None;
        if let Ok((mut sprite, _, mut vis)) = ghost.single_mut() {
            sprite.color = Color::srgba(1.0, 1.0, 1.0, 0.0);
            *vis = Visibility::Hidden;
        }
        return;
    }

    let Some(kind) = drag.kind else {
        return;
    };

    let Ok(window) = windows.single() else {
        return;
    };
    let Ok((camera, cam_tf)) = camera_q.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok(world) = camera.viewport_to_world_2d(cam_tf, cursor) else {
        return;
    };

    let (fw, fh) = kind.footprint();
    let (tx, ty) = world_to_tile(world.x, world.y);
    // Snap so cursor is near footprint center
    let tile_x = tx - (fw as i16 / 2);
    let tile_y = ty - (fh as i16 / 2);
    drag.tile_x = tile_x;
    drag.tile_y = tile_y;
    drag.valid = sim.0.can_place(local.faction, kind, tile_x, tile_y);

    if buttons.just_released(MouseButton::Left) {
        if drag.valid {
            intent.place = Some(PlaceBuildingCmd {
                kind,
                tile_x,
                tile_y,
            });
        }
        drag.kind = None;
        if let Ok((mut sprite, _, mut vis)) = ghost.single_mut() {
            sprite.color = Color::srgba(1.0, 1.0, 1.0, 0.0);
            *vis = Visibility::Hidden;
        }
    }
}

fn update_ghost(
    drag: Res<BuildDrag>,
    mut ghost: Query<(&mut Sprite, &mut Transform, &mut Visibility), With<GhostPreview>>,
) {
    let Ok((mut sprite, mut tf, mut vis)) = ghost.single_mut() else {
        return;
    };
    let Some(kind) = drag.kind else {
        *vis = Visibility::Hidden;
        return;
    };
    *vis = Visibility::Visible;
    let (fw, fh) = kind.footprint();
    let (cx, cy) = footprint_center(drag.tile_x, drag.tile_y, fw, fh);
    tf.translation.x = cx;
    tf.translation.y = cy;
    tf.translation.z = 50.0;
    sprite.custom_size = Some(building_display_size(kind));
    sprite.color = if drag.valid {
        Color::srgba(0.45, 1.0, 0.5, 0.65)
    } else {
        Color::srgba(1.0, 0.35, 0.3, 0.55)
    };
}

fn sync_world_visuals(
    mut commands: Commands,
    sim: Res<GameSim>,
    atlas: Res<PixelAtlas>,
    // B0001: chaque Query<&mut Sprite> doit être disjoint (Without) — sinon panic WASM / écran noir.
    mut units_q: Query<
        (Entity, &UnitVisual, &mut Transform, &mut Sprite),
        (
            Without<BuildingVisual>,
            Without<CastleVisual>,
            Without<PointVisual>,
        ),
    >,
    mut buildings_q: Query<
        (Entity, &BuildingVisual, &mut Transform, &mut Sprite),
        (Without<UnitVisual>, Without<CastleVisual>, Without<PointVisual>),
    >,
    mut castles_q: Query<
        (&CastleVisual, &mut Sprite),
        (Without<UnitVisual>, Without<BuildingVisual>, Without<PointVisual>),
    >,
    mut points_q: Query<
        (&PointVisual, &mut Sprite),
        (
            Without<UnitVisual>,
            Without<CastleVisual>,
            Without<BuildingVisual>,
        ),
    >,
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
                    image: atlas.castle.clone(),
                    custom_size: Some(castle_display_size()),
                    color: Color::srgba(col[0], col[1], col[2], 1.0),
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
            let scale = 0.75 + 0.25 * (c.hp / c.max_hp);
            sprite.custom_size = Some(castle_display_size() * scale);
        }
    }

    for (vis, mut sprite) in points_q.iter_mut() {
        if let Some(p) = world.points.get(vis.index) {
            sprite.color = point_color(p);
        }
    }

    // Buildings
    let mut seen_b = std::collections::HashSet::new();
    for (entity, vis, mut tf, mut sprite) in buildings_q.iter_mut() {
        if let Some(b) = world.buildings.iter().find(|b| b.id == vis.id) {
            seen_b.insert(b.id);
            let (cx, cy) = b.center();
            tf.translation.x = cx;
            tf.translation.y = cy;
            let col = faction_color(b.faction);
            sprite.color = Color::srgba(col[0], col[1], col[2], 1.0);
            sprite.custom_size = Some(building_display_size(b.kind));
        } else {
            commands.entity(entity).despawn();
        }
    }
    for b in &world.buildings {
        if !seen_b.contains(&b.id) {
            let col = faction_color(b.faction);
            let (cx, cy) = b.center();
            commands.spawn((
                BuildingVisual { id: b.id },
                Sprite {
                    image: atlas.building(b.kind),
                    custom_size: Some(building_display_size(b.kind)),
                    color: Color::srgba(col[0], col[1], col[2], 1.0),
                    ..default()
                },
                Transform::from_xyz(cx, cy, 6.0),
            ));
        }
    }

    // Units
    let mut seen = std::collections::HashSet::new();
    for (entity, vis, mut tf, mut sprite) in units_q.iter_mut() {
        if let Some(u) = world.units.iter().find(|u| u.id == vis.id) {
            seen.insert(u.id);
            tf.translation.x = u.x;
            tf.translation.y = u.y;
            let col = faction_color(u.faction);
            sprite.color = Color::srgba(col[0], col[1], col[2], 1.0);
            sprite.custom_size = Some(unit_display_size());
            sprite.image = atlas.unit(u.unit_type);
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
                    image: atlas.unit(u.unit_type),
                    custom_size: Some(unit_display_size()),
                    color: Color::srgba(col[0], col[1], col[2], 1.0),
                    ..default()
                },
                Transform::from_xyz(u.x, u.y, 10.0),
            ));
        }
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
    drag: Res<BuildDrag>,
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
    let income = world.income_per_sec(local.faction);
    let units = world
        .units
        .iter()
        .filter(|u| u.faction == local.faction)
        .count();
    let buildings = world
        .buildings
        .iter()
        .filter(|b| b.faction == local.faction)
        .count();
    let team = world.team_of(local.faction);

    let mut status = format!(
        "Gold {:.0}  (+{:.0}/s)\nFaction {} | Team {} | Bâtiments {} | Unités {}\nGlisser un bâtiment depuis la barre · Esc annule · WASD / RMB caméra",
        gold, income, local.faction.0, team.0, buildings, units
    );
    if let Some(kind) = drag.kind {
        status.push_str(&format!(
            "\nPose: {} {}",
            kind.label(),
            if drag.valid { "✓" } else { "✗" }
        ));
    }
    if let VictoryState::TeamWon(t) = victory.0 {
        status.push_str(&format!("\n*** TEAM {} WINS ***  (Esc = menu)", t.0));
    }

    *text = Text::new(status);
}
