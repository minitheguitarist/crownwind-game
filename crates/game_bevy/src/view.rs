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
use crate::{GameSim, VIRTUAL_HEIGHT, VIRTUAL_WIDTH};

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
    let size = UVec2::new(VIRTUAL_WIDTH as u32 * scale, VIRTUAL_HEIGHT as u32 * scale);
    if size.x > physical.x || size.y > physical.y {
        return None;
    }
    Some(((physical - size) / 2, size))
}

/// Largest whole-number scale at which the 480×270 view fits the window.
pub fn integer_scale(physical: UVec2) -> u32 {
    let sx = physical.x as f32 / VIRTUAL_WIDTH;
    let sy = physical.y as f32 / VIRTUAL_HEIGHT;
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

pub fn spawn_camera(mut commands: Commands, sim: Res<GameSim>) {
    let rules = gc::CameraFollow {
        view_width: VIRTUAL_WIDTH,
        view_height: VIRTUAL_HEIGHT,
        ..default()
    };
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
                width: VIRTUAL_WIDTH,
                height: VIRTUAL_HEIGHT,
            },
            ..OrthographicProjection::default_2d()
        }),
        Transform::from_xyz(c.x, c.y, 0.0),
    ));
}

/// Keep one world pixel = an integer number of screen pixels by rendering
/// into a centered viewport of exactly `n` × 480×270 physical pixels.
pub fn fit_integer_scale(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut cam: ResMut<FollowCam>,
    mut camera: Query<&mut Camera, With<MainCamera>>,
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
        assert_eq!(integer_scale(UVec2::new(960, 540)), 2);
        assert_eq!(integer_scale(UVec2::new(1920, 1080)), 4);
        assert_eq!(integer_scale(UVec2::new(1280, 720)), 2);
        assert_eq!(integer_scale(UVec2::new(300, 200)), 1);
    }

    #[test]
    fn letterbox_centers_an_exact_multiple() {
        assert_eq!(
            letterbox(UVec2::new(960, 540), 2),
            Some((UVec2::ZERO, UVec2::new(960, 540)))
        );
        // 1000×600 window at 2×: 20 px bars left/right, 30 px top/bottom.
        assert_eq!(
            letterbox(UVec2::new(1000, 600), 2),
            Some((UVec2::new(20, 30), UVec2::new(960, 540)))
        );
        // Ultra-wide 2560×1080 at 4×: side bars only.
        assert_eq!(
            letterbox(UVec2::new(2560, 1080), 4),
            Some((UVec2::new(320, 0), UVec2::new(1920, 1080)))
        );
        assert_eq!(letterbox(UVec2::new(300, 200), 1), None);
    }
}
