//! Frame-rate independent camera follow math (pure Rust, no engine types).
//!
//! Behaviour:
//! - Horizontal: dead zone around a point slightly *ahead* of the player in
//!   the facing direction, so more of the upcoming course is visible.
//! - Vertical: while grounded the camera settles on the ground height
//!   ("platform snap"). While airborne it ignores normal jump arcs and only
//!   follows when the player leaves a taller zone (big falls, wall climbs).
//! - Both axes use exponential smoothing, so the result does not depend on
//!   how frame time is sliced.

use crate::geometry::{Rect, Vec2};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraFollow {
    /// Visible area in world pixels (virtual viewport).
    pub view_width: f32,
    pub view_height: f32,
    /// Horizontal half-width of the zone in which the target can move
    /// without the camera reacting.
    pub dead_zone_x: f32,
    /// Vertical half-height of the zone while grounded.
    pub dead_zone_y: f32,
    /// Distance above / below the camera center the player may travel while
    /// airborne before the camera follows.
    pub air_zone_up: f32,
    pub air_zone_down: f32,
    /// How far ahead of the player (in the facing direction) to look.
    pub look_ahead: f32,
    /// Exponential smoothing rates (1/s). Higher = snappier.
    pub smoothing_x: f32,
    pub smoothing_y: f32,
    /// The camera centers this far above the player's feet.
    pub height_offset: f32,
}

impl Default for CameraFollow {
    fn default() -> Self {
        Self {
            view_width: 480.0,
            view_height: 270.0,
            dead_zone_x: 16.0,
            dead_zone_y: 6.0,
            air_zone_up: 56.0,
            air_zone_down: 40.0,
            look_ahead: 32.0,
            smoothing_x: 5.0,
            smoothing_y: 4.0,
            height_offset: 40.0,
        }
    }
}

/// What the camera is following this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraTarget {
    /// Player feet position.
    pub feet: Vec2,
    pub grounded: bool,
    /// -1 or 1.
    pub facing: f32,
}

fn follow_axis(cur: f32, tgt: f32, below: f32, above: f32) -> f32 {
    if tgt > cur + above {
        tgt - above
    } else if tgt < cur - below {
        tgt + below
    } else {
        cur
    }
}

impl CameraFollow {
    /// Where the camera wants to be given its current center and the target.
    pub fn desired(&self, current: Vec2, target: CameraTarget) -> Vec2 {
        let tx = target.feet.x + target.facing.signum() * self.look_ahead;
        let ty = target.feet.y + self.height_offset;
        let x = follow_axis(current.x, tx, self.dead_zone_x, self.dead_zone_x);
        let y = if target.grounded {
            follow_axis(current.y, ty, self.dead_zone_y, self.dead_zone_y)
        } else {
            follow_axis(current.y, ty, self.air_zone_down, self.air_zone_up)
        };
        Vec2::new(x, y)
    }

    /// Keep the view inside `bounds` (a world rect). If the bounds are smaller
    /// than the view on an axis, the view is centered on that axis.
    pub fn clamp(&self, center: Vec2, bounds: Rect) -> Vec2 {
        let clamp_axis = |c: f32, lo: f32, hi: f32, half: f32| {
            if hi - lo <= half * 2.0 {
                (lo + hi) * 0.5
            } else {
                c.clamp(lo + half, hi - half)
            }
        };
        Vec2::new(
            clamp_axis(
                center.x,
                bounds.left(),
                bounds.right(),
                self.view_width * 0.5,
            ),
            clamp_axis(
                center.y,
                bounds.bottom(),
                bounds.top(),
                self.view_height * 0.5,
            ),
        )
    }

    /// One frame of smoothed follow. `dt` is the real frame time.
    pub fn update(&self, current: Vec2, target: CameraTarget, bounds: Rect, dt: f32) -> Vec2 {
        let goal = self.clamp(self.desired(current, target), bounds);
        let dt = dt.max(0.0);
        let kx = 1.0 - (-self.smoothing_x * dt).exp();
        let ky = 1.0 - (-self.smoothing_y * dt).exp();
        Vec2::new(
            current.x + (goal.x - current.x) * kx,
            current.y + (goal.y - current.y) * ky,
        )
    }

    /// Immediate placement (used at spawn / respawn).
    pub fn snap(&self, target: CameraTarget, bounds: Rect) -> Vec2 {
        let t = Vec2::new(
            target.feet.x + target.facing.signum() * self.look_ahead,
            target.feet.y + self.height_offset,
        );
        self.clamp(t, bounds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn big_bounds() -> Rect {
        Rect::new(-10_000.0, -10_000.0, 20_000.0, 20_000.0)
    }

    fn on_ground(x: f32, y: f32) -> CameraTarget {
        CameraTarget {
            feet: Vec2::new(x, y),
            grounded: true,
            facing: 1.0,
        }
    }

    fn airborne(x: f32, y: f32) -> CameraTarget {
        CameraTarget {
            grounded: false,
            ..on_ground(x, y)
        }
    }

    #[test]
    fn small_moves_inside_dead_zone_do_not_move_camera() {
        let cam = CameraFollow::default();
        let current = cam.snap(on_ground(0.0, 0.0), big_bounds());
        let next = cam.update(current, on_ground(10.0, 3.0), big_bounds(), 1.0 / 60.0);
        assert_eq!(next, current);
    }

    #[test]
    fn camera_converges_without_overshoot() {
        let cam = CameraFollow::default();
        let mut c = cam.snap(on_ground(0.0, 0.0), big_bounds());
        let target = on_ground(300.0, 0.0);
        let goal_x = 300.0 + cam.look_ahead - cam.dead_zone_x;
        let mut last_x = c.x;
        for _ in 0..600 {
            c = cam.update(c, target, big_bounds(), 1.0 / 60.0);
            assert!(c.x >= last_x, "camera moved backwards");
            assert!(c.x <= goal_x + 1e-3, "overshoot");
            last_x = c.x;
        }
        assert!((c.x - goal_x).abs() < 0.01);
    }

    #[test]
    fn normal_jump_arc_does_not_move_camera_vertically() {
        let cam = CameraFollow::default();
        let start = cam.snap(on_ground(0.0, 0.0), big_bounds());
        let mut c = start;
        // A 50 px high jump.
        for i in 0..40 {
            let t = i as f32 / 40.0;
            let h = 200.0 * t * (1.0 - t);
            c = cam.update(c, airborne(0.0, h), big_bounds(), 1.0 / 60.0);
        }
        assert_eq!(c.y, start.y);
    }

    #[test]
    fn camera_settles_on_new_ground_height() {
        let cam = CameraFollow::default();
        let mut c = cam.snap(on_ground(0.0, 0.0), big_bounds());
        for _ in 0..600 {
            c = cam.update(c, on_ground(0.0, 64.0), big_bounds(), 1.0 / 60.0);
        }
        let goal = 64.0 + cam.height_offset - cam.dead_zone_y;
        assert!((c.y - goal).abs() < 0.01, "{}", c.y);
    }

    #[test]
    fn long_fall_is_followed() {
        let cam = CameraFollow::default();
        let mut c = cam.snap(on_ground(0.0, 200.0), big_bounds());
        for i in 0..60 {
            let y = 200.0 - 5.0 * i as f32;
            c = cam.update(c, airborne(0.0, y), big_bounds(), 1.0 / 60.0);
        }
        assert!(c.y < 200.0, "camera did not follow the fall: {}", c.y);
    }

    #[test]
    fn look_ahead_follows_facing() {
        let cam = CameraFollow::default();
        let r = cam.snap(on_ground(0.0, 0.0), big_bounds());
        let l = cam.snap(
            CameraTarget {
                facing: -1.0,
                ..on_ground(0.0, 0.0)
            },
            big_bounds(),
        );
        assert_eq!(r.x - l.x, 2.0 * cam.look_ahead);
    }

    #[test]
    fn smoothing_is_frame_rate_independent() {
        let cam = CameraFollow::default();
        let target = on_ground(500.0, 0.0);
        let run = |steps: usize| {
            let mut c = Vec2::new(0.0, cam.height_offset);
            for _ in 0..steps {
                c = cam.update(c, target, big_bounds(), 1.0 / steps as f32);
            }
            c
        };
        let a = run(30);
        let b = run(240);
        assert!((a.x - b.x).abs() < 1.0, "{} vs {}", a.x, b.x);
    }

    #[test]
    fn view_is_clamped_to_bounds() {
        let cam = CameraFollow::default();
        let bounds = Rect::new(0.0, 0.0, 1000.0, 400.0);
        let c = cam.snap(on_ground(-500.0, -500.0), bounds);
        assert_eq!(c, Vec2::new(240.0, 135.0));
        let narrow = Rect::new(0.0, 0.0, 100.0, 100.0);
        assert_eq!(cam.snap(on_ground(0.0, 0.0), narrow), Vec2::new(50.0, 50.0));
    }
}
