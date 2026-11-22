//! Level and player visuals: Windwood tiles, the prince sprite, and
//! placeholder checkpoint flags / exit door (no art for those yet).
//!
//! Draw order (z): sky -100, forest -90, exit door -1, terrain 0,
//! roots 0.1, plants 0.2, thorns 0.4, checkpoints 0.6, prince 2,
//! particles 5. Everything is placed on the 1-world-pixel grid.

use bevy::prelude::*;
use game_core as gc;

use crate::animation::{PrinceManifest, PrinceSprite, PrincessSprite};
use crate::manifest::{
    AnimationManifest, HERO_ATLAS_COLUMNS, HERO_ATLAS_PATH, PRINCESS_ATLAS_COLUMNS,
    PRINCESS_ATLAS_PATH,
};
use crate::tiles::{self, Layer, TILE, TILE_COLUMNS, TILES_PATH};
use crate::{ArtHandles, GameSim};

/// Letterbox bars and anything outside the 480×270 view.
pub const BACKGROUND: Color = Color::srgb(0.06, 0.05, 0.08);

// Placeholder props, picked from the Windwood / prince palette.
const POLE_COLOR: Color = Color::srgb_u8(0x29, 0x23, 0x34);
const POLE_CAP: Color = Color::srgb_u8(0xD8, 0xAC, 0x66);
pub const FLAG_IDLE: Color = Color::srgb_u8(0x68, 0x86, 0x9B);
pub const FLAG_ACTIVE: Color = Color::srgb_u8(0xD8, 0xAC, 0x66);

/// The princess stands just behind the prince's layer.
const Z_PRINCESS: f32 = 1.5;
/// How far past the goal trigger's centre the princess stands (units).
const PRINCESS_GOAL_OFFSET_X: f32 = 14.0;
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

/// Round a gameplay-unit coordinate to the render-pixel grid
/// (1 / `PIXELS_PER_UNIT` units).
pub fn snap_scalar(v: f32) -> f32 {
    (v * crate::PIXELS_PER_UNIT).round() / crate::PIXELS_PER_UNIT
}

/// Round to the render-pixel grid of the 640×360 internal resolution.
pub fn snap(v: Vec2) -> Vec2 {
    Vec2::new(snap_scalar(v.x), snap_scalar(v.y))
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
            Sprite {
                // One tile always covers one 16-unit cell, whatever the
                // art's texel size (16 legacy, 32 for the M2 standard).
                custom_size: Some(Vec2::splat(TILE)),
                ..Sprite::from_atlas_image(
                    tiles_image.clone(),
                    TextureAtlas {
                        layout: tiles_layout.clone(),
                        index: p.index,
                    },
                )
            },
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

    // --- Princess at the goal -------------------------------------------------
    // The finish is the princess herself: she stands on the goal's floor
    // line just past the trigger (so the arriving prince stops beside her
    // rather than inside her), facing back toward him.
    let g = level.goal;
    let pm = AnimationManifest::princess();
    let princess_image: Handle<Image> = assets.load(PRINCESS_ATLAS_PATH);
    art.0.push(princess_image.clone().untyped());
    let princess_layout = layouts.add(TextureAtlasLayout::from_grid(
        UVec2::new(pm.frame_width, pm.frame_height),
        PRINCESS_ATLAS_COLUMNS,
        1,
        None,
        None,
    ));
    let feet = snap(Vec2::new(g.x + g.w * 0.5 + PRINCESS_GOAL_OFFSET_X, g.y));
    let offset = crate::animation::sprite_offset_units(&pm);
    let idle_first = pm.clip("idle").map_or(0, |c| c.frames[0]);
    commands
        .spawn((
            Transform::from_xyz(feet.x, feet.y, Z_PRINCESS),
            Visibility::default(),
        ))
        .with_children(|p| {
            p.spawn((
                PrincessSprite::default(),
                Sprite {
                    custom_size: Some(crate::animation::frame_size_units(&pm)),
                    flip_x: true,
                    ..Sprite::from_atlas_image(
                        princess_image,
                        TextureAtlas {
                            layout: princess_layout,
                            index: idle_first,
                        },
                    )
                },
                Transform::from_translation(Vec2::new(-offset.x, offset.y).extend(0.0)),
            ));
        });

    // --- Prince ----------------------------------------------------------------
    // The root sits at the simulation's feet point; the atlas sprite is a
    // child offset so the manifest pivot lands on it (see `animation`).
    let m = &manifest.0;
    let prince_image: Handle<Image> = assets.load(HERO_ATLAS_PATH);
    art.0.push(prince_image.clone().untyped());
    let prince_layout = layouts.add(TextureAtlasLayout::from_grid(
        UVec2::new(m.frame_width, m.frame_height),
        HERO_ATLAS_COLUMNS,
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
                Sprite {
                    custom_size: Some(crate::animation::frame_size_units(m)),
                    ..Sprite::from_atlas_image(
                        prince_image,
                        TextureAtlas {
                            layout: prince_layout,
                            index: first,
                        },
                    )
                },
                Transform::from_translation(crate::animation::sprite_offset_units(m).extend(0.0)),
            ));
        });
}

/// Horizontal render position of the prince on the pixel grid.
///
/// The camera center moves smoothly but is drawn at `round(camera_x)`. If
/// the prince were drawn at `round(x)`, both roundings flip on different
/// frames and the prince wobbles by 1 px on screen while the camera eases.
/// Rounding the *offset from the camera* instead keeps the on-screen position
/// stable; the prince may sit up to 1 px off its exact world column, which
/// is invisible horizontally. Vertically the plain `round(y)` is kept so the
/// feet always rest exactly on the tile surface.
pub fn render_x(player_x: f32, camera_x: f32) -> f32 {
    snap_scalar(camera_x) + snap_scalar(player_x - camera_x)
}

pub fn sync_player(
    sim: Res<GameSim>,
    cam: Res<crate::view::FollowCam>,
    mut root: Query<(&mut Transform, &mut Visibility), With<PlayerRoot>>,
) {
    let p = sim.0.interpolated_position();
    let pos = Vec2::new(render_x(p.x, cam.center.x), snap_scalar(p.y));
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Prince and camera moving together at a fractional speed: the prince's
    /// on-screen x (render position minus drawn camera position) must not
    /// wobble, while naive independent rounding does.
    #[test]
    fn camera_relative_rounding_removes_one_pixel_wobble() {
        let speed = 2.1667; // 130 px/s at 60 fps
        let lag = 13.4; // camera trails the prince by a fractional amount
        let mut naive = std::collections::HashSet::new();
        let mut fixed = std::collections::HashSet::new();
        for i in 0..240 {
            let px = 100.0 + speed * i as f32;
            let cx = px - lag;
            naive.insert(((snap_scalar(px) - snap_scalar(cx)) * 2.0) as i32);
            fixed.insert(((render_x(px, cx) - snap_scalar(cx)) * 2.0) as i32);
        }
        assert!(naive.len() > 1, "test setup should show the wobble");
        assert_eq!(fixed.len(), 1, "prince still wobbles: {fixed:?}");
    }

    #[test]
    fn render_x_stays_within_one_pixel_of_world_position() {
        for i in 0..1000 {
            let px = i as f32 * 0.37;
            let cx = px * 0.9 + 3.3;
            assert!((render_x(px, cx) - px).abs() <= 1.0 / crate::PIXELS_PER_UNIT);
            // On the render-pixel grid (0.5 units).
            assert_eq!((render_x(px, cx) * crate::PIXELS_PER_UNIT).fract(), 0.0);
        }
    }
}
