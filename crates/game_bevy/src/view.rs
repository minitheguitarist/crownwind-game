//! Camera: a fixed 480×270 world view, integer-scaled and letterboxed, with
//! a smoothed follow.
//!
//! The camera center is simulated smoothly (in `game_core::CameraFollow`),
//! but the transform is snapped to the 1-world-pixel grid so the level never
//! shimmers. The player is snapped to the same grid, so the two move in
//! lockstep pixel increments.
//!
//! The view is always exactly 480×270 world pixels (the size of the Windwood
//! backdrop plates). Extra window space becomes letterbox bars instead of
//! showing more world, so the plates always cover the whole view.

use bevy::camera::{ScalingMode, Viewport};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_core as gc;

use crate::world::snap;
use crate::{GameSim, RENDER_HEIGHT, RENDER_WIDTH, VIEW_HEIGHT, VIEW_WIDTH};

#[derive(Component)]
pub struct MainCamera;

#[derive(Resource)]
pub struct FollowCam {
    pub rules: gc::CameraFollow,
    pub center: gc::Vec2,
    pub seen_teleports: u32,
    /// Current integer pixel scale (screen pixels per world pixel).
    pub scale: u32,
    /// Smoothed horizontal look-ahead.
    pub look_offset: f32,
}

/// Centered viewport of `scale` × 480×270 inside a window, or `None` when
/// the window is smaller than one 480×270 view (then the view fills it).
pub fn letterbox(physical: UVec2, scale: u32) -> Option<(UVec2, UVec2)> {
    let size = UVec2::new(RENDER_WIDTH * scale, RENDER_HEIGHT * scale);
    if size.x > physical.x || size.y > physical.y {
        return None;
    }
    Some(((physical - size) / 2, size))
}

/// Largest whole-number scale at which the 480×270 view fits the window.
pub fn integer_scale(physical: UVec2) -> u32 {
    let sx = physical.x as f32 / RENDER_WIDTH as f32;
    let sy = physical.y as f32 / RENDER_HEIGHT as f32;
    (sx.min(sy).floor() as u32).max(1)
}

pub fn camera_bounds(level: &gc::Level) -> gc::Rect {
    let b = level.bounds();
    // Never look below the bottom of the terrain columns (the backdrop would
    // show under the ground); allow some sky above the tallest walls.
    gc::Rect::new(b.x, b.y, b.w, b.h + 64.0)
}

fn camera_target(sim: &gc::Simulation, look_offset: f32) -> gc::CameraTarget {
    gc::CameraTarget {
        feet: sim.interpolated_position(),
        grounded: sim.player.grounded,
        look_offset,
    }
}

/// Follow settings for the M2 view (320×180 units, character 27% of the
/// screen height). Zones are tightened versus the 480×270 defaults so the
/// larger character keeps similar *relative* framing:
/// - feet sit ~55 units (31%) above the bottom edge when grounded;
/// - look-ahead 28 units (~9% of the width) so upcoming jumps stay visible;
/// - a normal jump (~59 units) stays inside the airborne zone, so the camera
///   does not bob vertically on ordinary jumps.
pub fn m2_camera_rules() -> gc::CameraFollow {
    gc::CameraFollow {
        view_width: VIEW_WIDTH,
        view_height: VIEW_HEIGHT,
        dead_zone_x: 12.0,
        dead_zone_y: 4.0,
        air_zone_up: 62.0,
        air_zone_down: 30.0,
        look_ahead: 28.0,
        height_offset: 35.0,
        ..gc::CameraFollow::default()
    }
}

pub fn spawn_camera(mut commands: Commands, sim: Res<GameSim>) {
    let rules = m2_camera_rules();
    let look_offset = f32::from(sim.0.player.facing) * rules.look_ahead;
    let center = rules.snap(
        camera_target(&sim.0, look_offset),
        camera_bounds(&sim.0.level),
    );
    let c = snap(Vec2::new(center.x, center.y));
    commands.insert_resource(FollowCam {
        rules,
        center,
        seen_teleports: sim.0.teleports,
        scale: 2,
        look_offset,
    });
    commands.spawn((
        Camera2d,
        MainCamera,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::Fixed {
                width: VIEW_WIDTH,
                height: VIEW_HEIGHT,
            },
            ..OrthographicProjection::default_2d()
        }),
        Transform::from_xyz(c.x, c.y, 0.0),
    ));
}

/// Keep one world pixel = an integer number of screen pixels by rendering
/// into a centered viewport of exactly `n` × 640×360 physical pixels.
/// HUD text (not pixel art) is scaled to keep its 540-px-tall proportions.
pub fn fit_integer_scale(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut cam: ResMut<FollowCam>,
    mut camera: Query<&mut Camera, With<MainCamera>>,
    mut ui_scale: ResMut<UiScale>,
    mut last: Local<UVec2>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let physical = window.physical_size();
    if physical.x == 0 || physical.y == 0 || physical == *last {
        return;
    }
    *last = physical;
    let n = integer_scale(physical);
    cam.scale = n;
    let view_h = letterbox(physical, n).map_or(physical.y, |(_, s)| s.y) as f32;
    ui_scale.0 = (view_h / window.scale_factor()) / 540.0;
    if let Ok(mut c) = camera.single_mut() {
        c.viewport = letterbox(physical, n).map(|(pos, size)| Viewport {
            physical_position: pos,
            physical_size: size,
            ..default()
        });
    }
}

pub fn follow_camera(
    sim: Res<GameSim>,
    time: Res<Time>,
    mut cam: ResMut<FollowCam>,
    mut camera: Query<&mut Transform, With<MainCamera>>,
) {
    let bounds = camera_bounds(&sim.0.level);
    let rules = cam.rules;
    let dt = if sim.0.is_paused() {
        0.0
    } else {
        time.delta_secs()
    };
    let p = &sim.0.player;
    if cam.seen_teleports != sim.0.teleports {
        cam.look_offset = f32::from(p.facing) * rules.look_ahead;
    } else {
        cam.look_offset = rules.next_look_offset(
            cam.look_offset,
            p.grounded,
            p.velocity.x,
            f32::from(p.facing),
            dt,
        );
    }
    let target = camera_target(&sim.0, cam.look_offset);
    cam.center = if cam.seen_teleports != sim.0.teleports {
        cam.seen_teleports = sim.0.teleports;
        rules.snap(target, bounds)
    } else {
        rules.update(cam.center, target, bounds, dt)
    };
    let c = snap(Vec2::new(cam.center.x, cam.center.y));
    if let Ok(mut t) = camera.single_mut() {
        t.translation.x = c.x;
        t.translation.y = c.y;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_scale_picks_largest_fit() {
        assert_eq!(integer_scale(UVec2::new(1280, 720)), 2);
        assert_eq!(integer_scale(UVec2::new(1920, 1080)), 3);
        assert_eq!(integer_scale(UVec2::new(2560, 1440)), 4);
        assert_eq!(integer_scale(UVec2::new(960, 540)), 1);
        assert_eq!(integer_scale(UVec2::new(300, 200)), 1);
    }

    #[test]
    fn letterbox_centers_an_exact_multiple() {
        assert_eq!(
            letterbox(UVec2::new(1280, 720), 2),
            Some((UVec2::ZERO, UVec2::new(1280, 720)))
        );
        // 1366×768 laptop at 2×: 43 px side bars, 24 px top/bottom.
        assert_eq!(
            letterbox(UVec2::new(1366, 768), 2),
            Some((UVec2::new(43, 24), UVec2::new(1280, 720)))
        );
        // 960×540 at 1×: 160 px side bars, 90 px top/bottom.
        assert_eq!(
            letterbox(UVec2::new(960, 540), 1),
            Some((UVec2::new(160, 90), UVec2::new(640, 360)))
        );
        assert_eq!(letterbox(UVec2::new(300, 200), 1), None);
    }

    #[test]
    fn view_is_640x360_render_pixels() {
        assert_eq!(VIEW_WIDTH * crate::PIXELS_PER_UNIT, 640.0);
        assert_eq!(VIEW_HEIGHT * crate::PIXELS_PER_UNIT, 360.0);
    }

    #[test]
    fn m2_framing_keeps_character_and_jumps_on_screen() {
        let r = m2_camera_rules();
        let cfg = gc::MovementConfig::default();
        // Grounded: feet above the bottom edge, head well below the top.
        let feet_from_bottom = VIEW_HEIGHT * 0.5 - r.height_offset;
        assert!(feet_from_bottom > 40.0);
        assert!(feet_from_bottom + crate::CHARACTER_CELL_UNITS.1 < VIEW_HEIGHT * 0.6);
        // A full jump (v^2 / 2g) stays inside the airborne zone above the
        // grounded camera target: no vertical camera bob on normal jumps.
        let jump = cfg.jump_velocity * cfg.jump_velocity / (2.0 * cfg.gravity);
        assert!(jump < r.air_zone_up + r.dead_zone_y, "jump {jump}");
        // The head at the top of a full jump is still on screen.
        assert!(feet_from_bottom + jump + crate::CHARACTER_CELL_UNITS.1 < VIEW_HEIGHT);
        // Look-ahead leaves at least 4 tiles visible ahead of the character.
        let ahead = VIEW_WIDTH * 0.5 + r.look_ahead - r.dead_zone_x;
        assert!(ahead > 4.0 * gc::TILE_SIZE * 2.0);
    }
}
