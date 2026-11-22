//! Engine-independent rules for the Crownwind prototype.
//!
//! Claude's first shift should extend this crate with tested, deterministic
//! player movement and collision rules. It must not import Bevy.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerState {
    pub position: Vec2,
    pub velocity: Vec2,
    pub grounded: bool,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
            grounded: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_is_stationary() {
        let state = PlayerState::default();
        assert_eq!(state.position, Vec2::ZERO);
        assert_eq!(state.velocity, Vec2::ZERO);
        assert!(!state.grounded);
    }
}
