//! Engine-independent rules for the Crownwind prototype.
//!
//! Conventions:
//! - World units are pixels; +x right, +y up.
//! - The placeholder level uses a 16 px tile grid ([`level::TILE_SIZE`]).
//! - Gameplay advances only in fixed steps of [`movement::FIXED_DT`].
//!
//! This crate must not depend on Bevy.

pub mod camera;
pub mod geometry;
pub mod level;
pub mod movement;
pub mod route;
pub mod simulation;
pub mod tutorial;

pub use camera::{CameraFollow, CameraTarget};
pub use geometry::{Rect, Vec2};
pub use level::{Checkpoint, Level, TILE_SIZE, test_course};
pub use movement::{FIXED_DT, MovementConfig, PlayerInput, PlayerState, step};
pub use simulation::{DeathCause, GameEvent, Phase, Simulation};
pub use tutorial::tutorial_level;

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
