//! Procedural placeholder levels. Uses an explicit 16 px tile grid; these are
//! temporary courses until data-driven levels (`docs/contracts/level.schema.json`)
//! are integrated in a later shift.

use crate::geometry::{Rect, Vec2};

pub const TILE_SIZE: f32 = 16.0;

/// Hazard hit-boxes are shrunk by this many pixels on every side so that
/// grazing the visible edge of spikes is forgiven.
pub const HAZARD_FORGIVENESS: f32 = 2.0;

#[derive(Debug, Clone, PartialEq)]
pub struct Checkpoint {
    /// Touching this area activates the checkpoint.
    pub area: Rect,
    /// Player feet position used when respawning here.
    pub spawn: Vec2,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Level {
    pub name: &'static str,
    pub solids: Vec<Rect>,
    /// Touching any hazard kills the player.
    pub hazards: Vec<Rect>,
    /// Ordered by progression: a checkpoint only activates if it is further
    /// along than the currently active one.
    pub checkpoints: Vec<Checkpoint>,
    /// Player feet position at level start.
    pub spawn: Vec2,
    /// Falling below this y kills the player.
    pub kill_y: f32,
    /// Overlapping this rect completes the level.
    pub goal: Rect,
}

impl Level {
    /// Extent of all solids, useful for camera limits.
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

    /// True if `hitbox` touches a hazard (using the forgiving inset).
    pub fn touches_hazard(&self, hitbox: &Rect) -> bool {
        self.hazards
            .iter()
            .any(|h| h.inset(HAZARD_FORGIVENESS).overlaps(hitbox))
    }
}

/// Helper: checkpoint standing on a floor whose top is at tile row `floor_ty`.
pub fn checkpoint_at(tx: i32, floor_ty: i32) -> Checkpoint {
    let t = TILE_SIZE;
    let x = tx as f32 * t;
    let y = floor_ty as f32 * t;
    Checkpoint {
        area: Rect::new(x, y, t, 3.0 * t),
        spawn: Vec2::new(x + t * 0.5, y),
    }
}

/// The original first-shift course, extended with a few hazards and
/// checkpoints. Kept small and used by tests.
pub fn test_course() -> Level {
    let t = |x, y, w, h| Rect::from_tiles(x, y, w, h, TILE_SIZE);
    let solids = vec![
        t(-2, 0, 2, 12),
        t(0, 0, 20, 2),
        t(14, 2, 6, 1),
        t(20, 0, 3, 5),
        t(23, 0, 10, 2),
        t(36, 0, 6, 3),
        t(45, 4, 3, 1),
        t(51, 6, 3, 1),
        t(57, 4, 3, 1),
        t(62, 0, 16, 2),
        t(66, 5, 8, 1),
        t(80, 2, 3, 1),
        t(85, 4, 3, 1),
        t(90, 0, 12, 6),
        t(102, 0, 2, 14),
    ];
    let spikes = |x, y, w| {
        Rect::new(
            x as f32 * TILE_SIZE,
            y as f32 * TILE_SIZE,
            w as f32 * TILE_SIZE,
            8.0,
        )
    };
    Level {
        name: "test_course",
        solids,
        hazards: vec![spikes(27, 2, 2), spikes(70, 2, 2)],
        checkpoints: vec![checkpoint_at(24, 2), checkpoint_at(63, 2)],
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

    #[test]
    fn hazard_edges_are_forgiving() {
        let level = Level {
            hazards: vec![Rect::new(0.0, 0.0, 32.0, 8.0)],
            ..test_course()
        };
        // Grazing the top 1 px of the spike strip is forgiven.
        assert!(!level.touches_hazard(&Rect::new(0.0, 7.0, 12.0, 24.0)));
        // Standing in it is not.
        assert!(level.touches_hazard(&Rect::new(10.0, 4.0, 12.0, 24.0)));
    }
}
