//! Frame-rate independent camera follow math (pure Rust, no engine types).

use crate::geometry::{Rect, Vec2};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraFollow {
    /// Visible area in world pixels (virtual viewport).
    pub view_width: f32,
    pub view_height: f32,
    /// Horizontal half-width of the zone in which the target can move
    /// without the camera reacting.
    pub dead_zone_x: f32,
    /// Vertical half-height of the dead zone.
    pub dead_zone_y: f32,
    /// Exponential smoothing rate (1/s). Higher = snappier.
    pub smoothing: f32,
    /// Offset added to the target (e.g. look slightly above the feet).
    pub target_offset: Vec2,
}

impl Default for CameraFollow {
    fn default() -> Self {
        Self {
            view_width: 480.0,
            view_height: 270.0,
            dead_zone_x: 24.0,
            dead_zone_y: 32.0,
            smoothing: 6.0,
            target_offset: Vec2::new(0.0, 32.0),
        }
    }
}

impl CameraFollow {
    /// Where the camera wants to be given its current center and the target.
    pub fn desired(&self, current: Vec2, target: Vec2) -> Vec2 {
        let t = Vec2::new(
            target.x + self.target_offset.x,
            target.y + self.target_offset.y,
        );
        let follow = |cur: f32, tgt: f32, zone: f32| {
            if tgt > cur + zone {
                tgt - zone
            } else if tgt < cur - zone {
                tgt + zone
            } else {
                cur
            }
        };
        Vec2::new(
            follow(current.x, t.x, self.dead_zone_x),
            follow(current.y, t.y, self.dead_zone_y),
        )
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

    /// One frame of smoothed follow. `dt` is the real frame time; the result
    /// converges identically regardless of how the time is sliced.
    pub fn update(&self, current: Vec2, target: Vec2, bounds: Rect, dt: f32) -> Vec2 {
        let goal = self.clamp(self.desired(current, target), bounds);
        let k = 1.0 - (-self.smoothing * dt.max(0.0)).exp();
        current.lerp(goal, k)
    }

    /// Immediate placement (used at spawn / reset).
    pub fn snap(&self, target: Vec2, bounds: Rect) -> Vec2 {
        let t = Vec2::new(
            target.x + self.target_offset.x,
            target.y + self.target_offset.y,
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

    #[test]
    fn small_moves_inside_dead_zone_do_not_move_camera() {
        let cam = CameraFollow::default();
        let current = Vec2::new(0.0, 32.0);
        let next = cam.update(current, Vec2::new(10.0, 5.0), big_bounds(), 1.0 / 60.0);
        assert_eq!(next, current);
    }

    #[test]
    fn camera_converges_without_overshoot() {
        let cam = CameraFollow::default();
        let mut c = Vec2::ZERO;
        let target = Vec2::new(300.0, 0.0);
        let mut last_x = c.x;
        for _ in 0..600 {
            c = cam.update(c, target, big_bounds(), 1.0 / 60.0);
            assert!(c.x >= last_x, "camera moved backwards");
            assert!(c.x <= target.x, "overshoot");
            last_x = c.x;
        }
        assert!((c.x - (target.x - cam.dead_zone_x)).abs() < 0.01);
    }

    #[test]
    fn smoothing_is_frame_rate_independent() {
        let cam = CameraFollow::default();
        let target = Vec2::new(500.0, 0.0);
        let run = |steps: usize| {
            let mut c = Vec2::ZERO;
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
        let c = cam.snap(Vec2::new(-500.0, -500.0), bounds);
        assert_eq!(c, Vec2::new(240.0, 135.0));
        let narrow = Rect::new(0.0, 0.0, 100.0, 100.0);
        assert_eq!(cam.snap(Vec2::ZERO, narrow), Vec2::new(50.0, 50.0));
    }
}
