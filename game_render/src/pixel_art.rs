//! Procedural nearest-neighbor pixel sprites (32px tile).

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use game_core::{BuildingKind, UnitType, TILE_SIZE};

#[derive(Resource)]
pub struct PixelAtlas {
    pub soldier: Handle<Image>,
    pub archer: Handle<Image>,
    pub mage: Handle<Image>,
    pub priest: Handle<Image>,
    pub barracks: Handle<Image>,
    pub archery: Handle<Image>,
    pub mage_tower: Handle<Image>,
    pub temple: Handle<Image>,
    pub castle: Handle<Image>,
}

impl PixelAtlas {
    pub fn unit(&self, ut: UnitType) -> Handle<Image> {
        match ut {
            UnitType::Soldier => self.soldier.clone(),
            UnitType::Archer => self.archer.clone(),
            UnitType::Mage => self.mage.clone(),
            UnitType::Priest => self.priest.clone(),
        }
    }

    pub fn building(&self, kind: BuildingKind) -> Handle<Image> {
        match kind {
            BuildingKind::Barracks => self.barracks.clone(),
            BuildingKind::Archery => self.archery.clone(),
            BuildingKind::MageTower => self.mage_tower.clone(),
            BuildingKind::Temple => self.temple.clone(),
        }
    }
}

pub fn build_atlas(images: &mut Assets<Image>) -> PixelAtlas {
    PixelAtlas {
        soldier: images.add(draw_unit_soldier()),
        archer: images.add(draw_unit_archer()),
        mage: images.add(draw_unit_mage()),
        priest: images.add(draw_unit_priest()),
        barracks: images.add(draw_building_barracks()),
        archery: images.add(draw_building_archery()),
        mage_tower: images.add(draw_building_mage()),
        temple: images.add(draw_building_temple()),
        castle: images.add(draw_building_castle()),
    }
}

fn finish(w: u32, h: u32, data: Vec<u8>) -> Image {
    Image::new(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

fn put(data: &mut [u8], w: u32, x: i32, y: i32, rgba: [u8; 4]) {
    if x < 0 || y < 0 || x >= w as i32 {
        return;
    }
    let h = (data.len() as u32) / (w * 4);
    if y >= h as i32 {
        return;
    }
    let i = ((y as u32 * w + x as u32) * 4) as usize;
    data[i..i + 4].copy_from_slice(&rgba);
}

fn fill_rect(data: &mut [u8], w: u32, x: i32, y: i32, rw: i32, rh: i32, rgba: [u8; 4]) {
    for yy in y..y + rh {
        for xx in x..x + rw {
            put(data, w, xx, yy, rgba);
        }
    }
}

fn outline_rect(data: &mut [u8], w: u32, x: i32, y: i32, rw: i32, rh: i32, rgba: [u8; 4]) {
    for xx in x..x + rw {
        put(data, w, xx, y, rgba);
        put(data, w, xx, y + rh - 1, rgba);
    }
    for yy in y..y + rh {
        put(data, w, x, yy, rgba);
        put(data, w, x + rw - 1, yy, rgba);
    }
}

const INK: [u8; 4] = [20, 18, 22, 255];
const SKIN: [u8; 4] = [220, 180, 140, 255];
const STEEL: [u8; 4] = [160, 170, 185, 255];
const WOOD: [u8; 4] = [110, 75, 45, 255];
const STONE: [u8; 4] = [90, 95, 105, 255];
const ROOF: [u8; 4] = [140, 55, 45, 255];
const GOLD: [u8; 4] = [220, 185, 60, 255];
const CLOTH_B: [u8; 4] = [55, 90, 160, 255];
const CLOTH_G: [u8; 4] = [50, 120, 70, 255];
const CLOTH_P: [u8; 4] = [120, 70, 160, 255];
const CLOTH_W: [u8; 4] = [230, 230, 235, 255];
const MAGIC: [u8; 4] = [120, 200, 255, 255];

fn draw_unit_soldier() -> Image {
    let w = 32u32;
    let mut data = vec![0u8; (w * w * 4) as usize];
    fill_rect(&mut data, w, 10, 8, 12, 10, CLOTH_B);
    fill_rect(&mut data, w, 12, 4, 8, 6, SKIN);
    fill_rect(&mut data, w, 11, 3, 10, 3, STEEL); // helm
    fill_rect(&mut data, w, 8, 12, 4, 10, STEEL); // shield
    fill_rect(&mut data, w, 22, 10, 3, 14, WOOD); // spear
    fill_rect(&mut data, w, 21, 8, 5, 3, STEEL);
    fill_rect(&mut data, w, 12, 18, 4, 8, INK);
    fill_rect(&mut data, w, 17, 18, 4, 8, INK);
    outline_rect(&mut data, w, 10, 8, 12, 10, INK);
    finish(w, w, data)
}

fn draw_unit_archer() -> Image {
    let w = 32u32;
    let mut data = vec![0u8; (w * w * 4) as usize];
    fill_rect(&mut data, w, 11, 9, 10, 10, CLOTH_G);
    fill_rect(&mut data, w, 13, 4, 7, 6, SKIN);
    fill_rect(&mut data, w, 12, 3, 9, 3, WOOD); // hood
    // bow
    for i in 0..10 {
        put(&mut data, w, 22, 8 + i, WOOD);
        put(&mut data, w, 23, 7 + i, WOOD);
    }
    fill_rect(&mut data, w, 8, 14, 4, 2, WOOD); // quiver
    fill_rect(&mut data, w, 12, 19, 3, 7, INK);
    fill_rect(&mut data, w, 17, 19, 3, 7, INK);
    outline_rect(&mut data, w, 11, 9, 10, 10, INK);
    finish(w, w, data)
}

fn draw_unit_mage() -> Image {
    let w = 32u32;
    let mut data = vec![0u8; (w * w * 4) as usize];
    fill_rect(&mut data, w, 10, 10, 12, 12, CLOTH_P);
    fill_rect(&mut data, w, 13, 5, 6, 6, SKIN);
    fill_rect(&mut data, w, 11, 2, 10, 5, CLOTH_P); // hat
    put(&mut data, w, 16, 1, GOLD);
    fill_rect(&mut data, w, 22, 12, 3, 12, WOOD); // staff
    fill_rect(&mut data, w, 21, 9, 5, 4, MAGIC);
    fill_rect(&mut data, w, 12, 22, 4, 6, INK);
    fill_rect(&mut data, w, 17, 22, 4, 6, INK);
    outline_rect(&mut data, w, 10, 10, 12, 12, INK);
    finish(w, w, data)
}

fn draw_unit_priest() -> Image {
    let w = 32u32;
    let mut data = vec![0u8; (w * w * 4) as usize];
    fill_rect(&mut data, w, 10, 10, 12, 12, CLOTH_W);
    fill_rect(&mut data, w, 13, 5, 6, 6, SKIN);
    fill_rect(&mut data, w, 12, 3, 8, 3, GOLD);
    // cross
    fill_rect(&mut data, w, 15, 12, 2, 8, GOLD);
    fill_rect(&mut data, w, 13, 14, 6, 2, GOLD);
    fill_rect(&mut data, w, 12, 22, 4, 6, INK);
    fill_rect(&mut data, w, 17, 22, 4, 6, INK);
    outline_rect(&mut data, w, 10, 10, 12, 12, INK);
    finish(w, w, data)
}

fn draw_building_base(data: &mut [u8], w: u32, roof: [u8; 4]) {
    fill_rect(data, w, 4, 20, 56, 40, STONE);
    fill_rect(data, w, 2, 8, 60, 16, roof);
    // windows
    fill_rect(data, w, 14, 32, 8, 10, MAGIC);
    fill_rect(data, w, 42, 32, 8, 10, MAGIC);
    fill_rect(data, w, 26, 40, 12, 18, WOOD); // door
    outline_rect(data, w, 4, 20, 56, 40, INK);
    outline_rect(data, w, 2, 8, 60, 16, INK);
}

fn draw_building_barracks() -> Image {
    let w = 64u32;
    let mut data = vec![0u8; (w * w * 4) as usize];
    draw_building_base(&mut data, w, ROOF);
    // banner
    fill_rect(&mut data, w, 28, 2, 8, 14, CLOTH_B);
    fill_rect(&mut data, w, 30, 4, 4, 6, STEEL);
    finish(w, w, data)
}

fn draw_building_archery() -> Image {
    let w = 64u32;
    let mut data = vec![0u8; (w * w * 4) as usize];
    draw_building_base(&mut data, w, [60, 100, 55, 255]);
    fill_rect(&mut data, w, 8, 24, 4, 28, WOOD);
    fill_rect(&mut data, w, 52, 24, 4, 28, WOOD);
    finish(w, w, data)
}

fn draw_building_mage() -> Image {
    let w = 64u32;
    let mut data = vec![0u8; (w * w * 4) as usize];
    draw_building_base(&mut data, w, CLOTH_P);
    fill_rect(&mut data, w, 28, 0, 8, 12, STONE); // spire
    fill_rect(&mut data, w, 26, 0, 12, 4, MAGIC);
    finish(w, w, data)
}

fn draw_building_temple() -> Image {
    let w = 64u32;
    let mut data = vec![0u8; (w * w * 4) as usize];
    draw_building_base(&mut data, w, CLOTH_W);
    fill_rect(&mut data, w, 30, 4, 4, 12, GOLD);
    fill_rect(&mut data, w, 26, 8, 12, 4, GOLD);
    finish(w, w, data)
}

fn draw_building_castle() -> Image {
    let w = 64u32;
    let mut data = vec![0u8; (w * w * 4) as usize];
    fill_rect(&mut data, w, 4, 16, 56, 44, STONE);
    // battlements
    for i in 0..7 {
        fill_rect(&mut data, w, 4 + i * 8, 8, 6, 10, STONE);
    }
    fill_rect(&mut data, w, 24, 36, 16, 24, WOOD);
    fill_rect(&mut data, w, 12, 28, 8, 10, MAGIC);
    fill_rect(&mut data, w, 44, 28, 8, 10, MAGIC);
    outline_rect(&mut data, w, 4, 16, 56, 44, INK);
    finish(w, w, data)
}

pub fn unit_display_size() -> Vec2 {
    Vec2::splat(TILE_SIZE)
}

pub fn building_display_size(kind: BuildingKind) -> Vec2 {
    let (fw, fh) = kind.footprint();
    Vec2::new(fw as f32 * TILE_SIZE, fh as f32 * TILE_SIZE)
}

pub fn castle_display_size() -> Vec2 {
    Vec2::splat(TILE_SIZE * 2.0)
}
