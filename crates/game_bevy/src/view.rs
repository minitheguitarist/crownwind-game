//! Camera: integer-scaled pixel-art view with a smoothed follow.
//!
//! The camera center is simulated smoothly (in `game_core::CameraFollow`),
//! but the transform is snapped to the 1-world-pixel grid so the level never
//! shimmers. The player is snapped to the same grid, so the two move in
//! lockstep pixel increments.

use bevy::camera::ScalingMode;
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
}

/// Largest whole-number scale at which the 480×270 view fits the window.
pub fn integer_scale(physical: UVec2) -> u32 {
    let sx = physical.x as f32 / VIRTUAL_WIDTH;
    let sy = physical.y as f32 / VIRTUAL_HEIGHT;
    (sx.min(sy).floor() as u32).max(1)
}

pub fn camera_bounds(level: &gc::Level) -> gc::Rect {
    let b = level.bounds();
    // Allow looking a little below the lowest ground and well above the top.
    gc::Rect::new(b.x, b.y - 48.0, b.w, b.h + 48.0 + 64.0)
}

fn camera_target(sim: &gc::Simulation) -> gc::CameraTarget {
    gc::CameraTarget {
        feet: sim.interpolated_position(),
        grounded: sim.player.grounded,
        facing: f32::from(sim.player.facing),
    }
}

pub fn spawn_camera(mut commands: Commands, sim: Res<GameSim>) {
    let rules = gc::CameraFollow {
        view_width: VIRTUAL_WIDTH,
        view_height: VIRTUAL_HEIGHT,
        ..default()
    };
    let center = rules.snap(camera_target(&sim.0), camera_bounds(&sim.0.level));
    let c = snap(Vec2::new(center.x, center.y));
    commands.insert_resource(FollowCam {
        rules,
        center,
        seen_teleports: sim.0.teleports,
        scale: 2,
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

/// Keep one world pixel = an integer number of screen pixels. A larger
/// window shows a little more of the world rather than stretching pixels.
pub fn fit_integer_scale(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut cam: ResMut<FollowCam>,
    mut projection: Query<&mut Projection, With<MainCamera>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let physical = window.physical_size();
    if physical.x == 0 || physical.y == 0 {
        return;
    }
    let n = integer_scale(physical);
    // Round the visible area to whole world pixels (even, so the center
    // stays on the grid).
    let w = ((physical.x / n) & !1) as f32;
    let h = ((physical.y / n) & !1) as f32;
    if cam.scale == n && cam.rules.view_width == w && cam.rules.view_height == h {
        return;
    }
    cam.scale = n;
    cam.rules.view_width = w;
    cam.rules.view_height = h;
    if let Ok(mut p) = projection.single_mut()
        && let Projection::Orthographic(o) = p.as_mut()
    {
        o.scaling_mode = ScalingMode::Fixed {
            width: w,
            height: h,
        };
    }
}

pub fn follow_camera(
    sim: Res<GameSim>,
    time: Res<Time>,
    mut cam: ResMut<FollowCam>,
    mut camera: Query<&mut Transform, With<MainCamera>>,
) {
    let bounds = camera_bounds(&sim.0.level);
    let target = camera_target(&sim.0);
    let rules = cam.rules;
    cam.center = if cam.seen_teleports != sim.0.teleports {
        cam.seen_teleports = sim.0.teleports;
        rules.snap(target, bounds)
    } else if sim.0.is_paused() {
        cam.center
    } else {
        rules.update(cam.center, target, bounds, time.delta_secs())
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
}
