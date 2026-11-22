//! Minimal 2D geometry used by the rules. World units are **pixels**, with
//! +x to the right and +y **up** (matching Bevy's 2D world convention, so the
//! integration layer does not need to flip anything).

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Linear interpolation, used by renderers to blend fixed-step states.
    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
        }
    }
}

/// Axis-aligned rectangle stored as its bottom-left corner plus size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    /// Rectangle in tile coordinates (bottom-left tile + size in tiles).
    pub fn from_tiles(tx: i32, ty: i32, tw: i32, th: i32, tile: f32) -> Self {
        Self::new(
            tx as f32 * tile,
            ty as f32 * tile,
            tw as f32 * tile,
            th as f32 * tile,
        )
    }

    pub fn left(&self) -> f32 {
        self.x
    }
    pub fn right(&self) -> f32 {
        self.x + self.w
    }
    pub fn bottom(&self) -> f32 {
        self.y
    }
    pub fn top(&self) -> f32 {
        self.y + self.h
    }
    pub fn center(&self) -> Vec2 {
        Vec2::new(self.x + self.w * 0.5, self.y + self.h * 0.5)
    }

    /// Rectangle shrunk by `m` on every side (grown if `m` is negative).
    /// Never produces negative sizes.
    pub fn inset(&self, m: f32) -> Rect {
        let mx = m.min(self.w * 0.5);
        let my = m.min(self.h * 0.5);
        Rect::new(
            self.x + mx,
            self.y + my,
            self.w - 2.0 * mx,
            self.h - 2.0 * my,
        )
    }

    /// Strict overlap: rectangles that merely touch along an edge do **not**
    /// overlap. This keeps a player resting exactly on a floor stable.
    pub fn overlaps(&self, other: &Rect) -> bool {
        self.left() < other.right()
            && self.right() > other.left()
            && self.bottom() < other.top()
            && self.top() > other.bottom()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn touching_edges_do_not_overlap() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(10.0, 0.0, 10.0, 10.0);
        let c = Rect::new(0.0, 10.0, 10.0, 10.0);
        assert!(!a.overlaps(&b));
        assert!(!a.overlaps(&c));
        assert!(a.overlaps(&Rect::new(9.5, 9.5, 2.0, 2.0)));
    }

    #[test]
    fn inset_shrinks_and_clamps() {
        let r = Rect::new(0.0, 0.0, 16.0, 8.0);
        assert_eq!(r.inset(2.0), Rect::new(2.0, 2.0, 12.0, 4.0));
        assert_eq!(r.inset(10.0), Rect::new(8.0, 4.0, 0.0, 0.0));
    }

    #[test]
    fn tile_rect_uses_tile_size() {
        let r = Rect::from_tiles(2, 1, 3, 1, 16.0);
        assert_eq!(r, Rect::new(32.0, 16.0, 48.0, 16.0));
    }
}
