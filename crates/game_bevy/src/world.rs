//! Level and player visuals: Windwood tiles, the prince sprite, and
//! placeholder checkpoint flags / exit door (no art for those yet).
//!
//! Draw order (z): sky -100, forest -90, exit door -1, terrain 0,
//! roots 0.1, plants 0.2, thorns 0.4, checkpoints 0.6, prince 2,
//! particles 5. Everything is placed on the 1-world-pixel grid.

use bevy::prelude::*;
use game_core as gc;

use crate::animation::{PrinceManifest, PrinceSprite};
use crate::manifest::{PRINCE_ATLAS_COLUMNS, PRINCE_ATLAS_PATH};
use crate::tiles::{self, Layer, TILE, TILE_COLUMNS, TILES_PATH};
use crate::{ArtHandles, GameSim};

/// Letterbox bars and anything outside the 480×270 view.
pub const BACKGROUND: Color = Color::srgb(0.06, 0.05, 0.08);

// Placeholder props, picked from the Windwood / prince palette.
const POLE_COLOR: Color = Color::srgb_u8(0x29, 0x23, 0x34);
const POLE_CAP: Color = Color::srgb_u8(0xD8, 0xAC, 0x66);
pub const FLAG_IDLE: Color = Color::srgb_u8(0x68, 0x86, 0x9B);
pub const FLAG_ACTIVE: Color = Color::srgb_u8(0xD8, 0xAC, 0x66);
const DOOR_FRAME: Color = Color::srgb_u8(0xD8, 0xAC, 0x66);
const DOOR_SHADOW: Color = Color::srgb_u8(0x29, 0x23, 0x34);
const DOOR_GLOW: Color = Color::srgb_u8(0xA4, 0xC9, 0xD2);
const DOOR_LIGHT: Color = Color::srgb_u8(0xF6, 0xE8, 0xCF);

const Z_GOAL: f32 = -1.0;
const Z_TERRAIN: f32 = 0.0;
const Z_ROOT: f32 = 0.1;
const Z_PLANT: f32 = 0.2;
const Z_THORNS: f32 = 0.4;
const Z_CHECKPOINT: f32 = 0.6;
const Z_PLAYER: f32 = 2.0;

#[derive(Component)]
pub struct PlayerRoot;

#[derive(Component)]
pub struct CheckpointFlag(pub usize);

/// Round to the 1-world-pixel grid used by the 480×270 virtual viewport.
pub fn snap(v: Vec2) -> Vec2 {
    v.round()
}

fn rect_sprite(commands: &mut Commands, rect: gc::Rect, color: Color, z: f32) {
    let c = rect.center();
    commands.spawn((
        Sprite::from_color(color, Vec2::new(rect.w, rect.h)),
        Transform::from_xyz(c.x, c.y, z),
    ));
}

pub fn spawn_level(
    mut commands: Commands,
    sim: Res<GameSim>,
    manifest: Res<PrinceManifest>,
    assets: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut art: ResMut<ArtHandles>,
) {
    let level = &sim.0.level;

    // --- Terrain, plants, thorns from the Windwood atlas --------------------
    let tiles_image: Handle<Image> = assets.load(TILES_PATH);
    art.0.push(tiles_image.clone().untyped());
    let tiles_layout = layouts.add(TextureAtlasLayout::from_grid(
        UVec2::splat(TILE as u32),
        TILE_COLUMNS,
        1,
        None,
        None,
    ));
    let placements = tiles::terrain(level)
        .into_iter()
        .chain(tiles::decorations(level))
        .chain(tiles::thorns(level));
    for p in placements {
        let z = match (p.layer, p.index) {
            (Layer::Terrain, _) => Z_TERRAIN,
            (Layer::Decor, tiles::ROOT) => Z_ROOT,
            (Layer::Decor, _) => Z_PLANT,
            (Layer::Hazard, _) => Z_THORNS,
        };
        commands.spawn((
            Sprite::from_atlas_image(
                tiles_image.clone(),
                TextureAtlas {
                    layout: tiles_layout.clone(),
                    index: p.index,
                },
            ),
            // 16 px cell: center = corner + 8 keeps edges on whole pixels.
            Transform::from_xyz(p.x + TILE * 0.5, p.y + TILE * 0.5, z),
        ));
    }

    // --- Checkpoints: pole + pennant (placeholder) ---------------------------
    // The pole stands 14 px behind (left of) the respawn point so the prince
    // never hides it, with the pennant pointing back toward the start.
    for (i, cp) in level.checkpoints.iter().enumerate() {
        let base = cp.spawn.x.round() - 14.0;
        let floor = cp.spawn.y;
        rect_sprite(
            &mut commands,
            gc::Rect::new(base - 1.0, floor, 2.0, 36.0),
            POLE_COLOR,
            Z_CHECKPOINT,
        );
        rect_sprite(
            &mut commands,
            gc::Rect::new(base - 2.0, floor + 36.0, 4.0, 2.0),
            POLE_CAP,
            Z_CHECKPOINT + 0.01,
        );
        commands.spawn((
            CheckpointFlag(i),
            Sprite::from_color(FLAG_IDLE, Vec2::new(12.0, 8.0)),
            Transform::from_xyz(base - 7.0, floor + 31.0, Z_CHECKPOINT + 0.01),
        ));
    }

    // --- Exit door (placeholder, same footprint as the goal trigger) --------
    let g = level.goal;
    rect_sprite(&mut commands, g.inset(-2.0), DOOR_FRAME, Z_GOAL);
    rect_sprite(&mut commands, g, DOOR_SHADOW, Z_GOAL + 0.01);
    // Pale light spilling through the doorway, brightest at the threshold.
    rect_sprite(
        &mut commands,
        gc::Rect::new(g.x + 6.0, g.y, g.w - 12.0, g.h - 10.0),
        DOOR_GLOW,
        Z_GOAL + 0.02,
    );
    rect_sprite(
        &mut commands,
        gc::Rect::new(g.x + 10.0, g.y, g.w - 20.0, g.h - 18.0),
        DOOR_LIGHT,
        Z_GOAL + 0.03,
    );

    // --- Prince ----------------------------------------------------------------
    // The root sits at the simulation's feet point; the atlas sprite is a
    // child offset so the manifest pivot lands on it (see `animation`).
    let m = &manifest.0;
    let prince_image: Handle<Image> = assets.load(PRINCE_ATLAS_PATH);
    art.0.push(prince_image.clone().untyped());
    let prince_layout = layouts.add(TextureAtlasLayout::from_grid(
        UVec2::new(m.frame_width, m.frame_height),
        PRINCE_ATLAS_COLUMNS,
        1,
        None,
        None,
    ));
    let first = m.clip("idle").map_or(0, |c| c.frames[0]);
    let pos = snap(Vec2::new(level.spawn.x, level.spawn.y));
    commands
        .spawn((
            PlayerRoot,
            Transform::from_xyz(pos.x, pos.y, Z_PLAYER),
            Visibility::default(),
        ))
        .with_children(|p| {
            p.spawn((
                PrinceSprite::default(),
                Sprite::from_atlas_image(
                    prince_image,
                    TextureAtlas {
                        layout: prince_layout,
                        index: first,
                    },
                ),
                Transform::from_translation(crate::animation::sprite_offset(m).extend(0.0)),
            ));
        });
}

pub fn sync_player(
    sim: Res<GameSim>,
    mut root: Query<(&mut Transform, &mut Visibility), With<PlayerRoot>>,
) {
    let pos = sim.0.interpolated_position();
    let pos = snap(Vec2::new(pos.x, pos.y));
    if let Ok((mut t, mut vis)) = root.single_mut() {
        t.translation.x = pos.x;
        t.translation.y = pos.y;
        let want = if sim.0.death_progress().is_some() {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        if *vis != want {
            *vis = want;
        }
    }
}

pub fn sync_checkpoints(sim: Res<GameSim>, mut flags: Query<(&CheckpointFlag, &mut Sprite)>) {
    let active = sim.0.active_checkpoint;
    for (flag, mut sprite) in &mut flags {
        let reached = active.is_some_and(|a| flag.0 <= a);
        let want = if reached { FLAG_ACTIVE } else { FLAG_IDLE };
        if sprite.color != want {
            sprite.color = want;
        }
    }
}
