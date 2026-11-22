//! Procedural placeholder level. Uses an explicit 16 px tile grid; this is a
//! temporary course until data-driven levels (`docs/contracts/level.schema.json`)
//! are integrated in a later shift.

use crate::geometry::{Rect, Vec2};

pub const TILE_SIZE: f32 = 16.0;

#[derive(Debug, Clone, PartialEq)]
pub struct Level {
    pub solids: Vec<Rect>,
    /// Player feet position on (re)spawn.
    pub spawn: Vec2,
    /// Falling below this y resets the player.
    pub kill_y: f32,
    /// Reaching a hitbox overlap with this rect counts as finishing.
    pub goal: Rect,
}

impl Level {
    /// Horizontal extent of all solids, useful for camera limits.
    pub fn bounds(&self) -> Rect {
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        for s in &self.solids {
            min_x = min_x.min(s.left());
            min_y = min_y.min(s.bottom());
            max_x = max_x.max(s.right());
            max_y = max_y.max(s.top());
        }
        Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }
}

/// A short left-to-right obstacle course in tile units: flat start, a step,
/// a wall that needs a jump, gaps, floating platforms and a final ledge.
pub fn test_course() -> Level {
    let t = |x, y, w, h| Rect::from_tiles(x, y, w, h, TILE_SIZE);
    let solids = vec![
        // Left boundary wall.
        t(-2, 0, 2, 12),
        // Starting ground.
        t(0, 0, 20, 2),
        // Small step up (1 tile).
        t(14, 2, 6, 1),
        // Wall needing a full jump (2 tiles above the step).
        t(20, 0, 3, 5),
        // Ground after the wall, then a gap.
        t(23, 0, 10, 2),
        // Gap of 3 tiles, then a pit-side pillar.
        t(36, 0, 6, 3),
        // Floating stepping stones over a long pit.
        t(45, 4, 3, 1),
        t(51, 6, 3, 1),
        t(57, 4, 3, 1),
        // Landing ground with a low ceiling tunnel.
        t(62, 0, 16, 2),
        t(66, 5, 8, 1),
        // Stair up to the goal ledge.
        t(80, 2, 3, 1),
        t(85, 4, 3, 1),
        t(90, 0, 12, 6),
        // Right boundary wall.
        t(102, 0, 2, 14),
    ];
    Level {
        solids,
        spawn: Vec2::new(3.0 * TILE_SIZE, 2.0 * TILE_SIZE),
        kill_y: -8.0 * TILE_SIZE,
        goal: t(98, 6, 2, 3),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movement::{MovementConfig, PlayerState};

    #[test]
    fn spawn_is_not_inside_a_solid() {
        let level = test_course();
        let cfg = MovementConfig::default();
        let hb = PlayerState::at(level.spawn).hitbox(&cfg);
        assert!(level.solids.iter().all(|s| !s.overlaps(&hb)));
    }

    #[test]
    fn spawn_and_goal_are_above_kill_plane() {
        let level = test_course();
        assert!(level.spawn.y > level.kill_y);
        assert!(level.goal.bottom() > level.kill_y);
    }

    #[test]
    fn bounds_cover_all_solids() {
        let level = test_course();
        let b = level.bounds();
        for s in &level.solids {
            assert!(s.left() >= b.left() && s.right() <= b.right());
        }
    }
}
