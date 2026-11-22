//! Fixed-timestep driver. Renderers feed it variable frame times; it runs
//! whole [`FIXED_DT`] steps and exposes an interpolation factor so drawing is
//! decoupled from the simulation rate.

use crate::geometry::Vec2;
use crate::level::Level;
use crate::movement::{FIXED_DT, MovementConfig, PlayerInput, PlayerState, step};

/// Upper bound on steps per `advance` call so a long hitch cannot cause a
/// "spiral of death"; excess time is dropped.
pub const MAX_STEPS_PER_ADVANCE: u32 = 8;

#[derive(Debug, Clone)]
pub struct Simulation {
    pub config: MovementConfig,
    pub level: Level,
    pub player: PlayerState,
    /// Player state before the most recent step (for render interpolation).
    pub previous_player: PlayerState,
    accumulator: f32,
    /// Jump press waiting for the next fixed step.
    pending_jump: bool,
    pub resets: u32,
    pub reached_goal: bool,
    pub steps: u64,
}

impl Simulation {
    pub fn new(level: Level, config: MovementConfig) -> Self {
        let player = PlayerState::at(level.spawn);
        Self {
            config,
            level,
            player,
            previous_player: player,
            accumulator: 0.0,
            pending_jump: false,
            resets: 0,
            reached_goal: false,
            steps: 0,
        }
    }

    /// Put the player back at spawn with no velocity.
    pub fn reset(&mut self) {
        self.player = PlayerState::at(self.level.spawn);
        self.previous_player = self.player;
        self.accumulator = 0.0;
        self.pending_jump = false;
        self.resets += 1;
    }

    /// Run exactly one fixed step with the given input.
    pub fn fixed_step(&mut self, input: PlayerInput) {
        self.previous_player = self.player;
        self.player = step(
            &self.player,
            input,
            &self.config,
            &self.level.solids,
            FIXED_DT,
        );
        self.steps += 1;
        if self.player.hitbox(&self.config).overlaps(&self.level.goal) {
            self.reached_goal = true;
        }
        if self.player.position.y < self.level.kill_y {
            self.reset();
        }
    }

    /// Accumulate `frame_dt` seconds and run as many fixed steps as fit.
    /// A `jump_pressed` edge is latched until a step consumes it, so a press
    /// on a frame with zero steps is not lost. Returns the number of steps.
    pub fn advance(&mut self, frame_dt: f32, input: PlayerInput) -> u32 {
        self.pending_jump |= input.jump_pressed;
        self.accumulator += frame_dt.max(0.0);
        let mut n = 0;
        while self.accumulator >= FIXED_DT && n < MAX_STEPS_PER_ADVANCE {
            self.accumulator -= FIXED_DT;
            let step_input = PlayerInput {
                jump_pressed: self.pending_jump,
                ..input
            };
            self.pending_jump = false;
            self.fixed_step(step_input);
            n += 1;
        }
        if n == MAX_STEPS_PER_ADVANCE {
            self.accumulator = self.accumulator.min(FIXED_DT);
        }
        n
    }

    /// Fraction of a step accumulated but not yet simulated, in `[0, 1)`.
    pub fn alpha(&self) -> f32 {
        (self.accumulator / FIXED_DT).clamp(0.0, 1.0)
    }

    /// Player feet position blended between the last two steps.
    pub fn interpolated_position(&self) -> Vec2 {
        self.previous_player
            .position
            .lerp(self.player.position, self.alpha())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::test_course;

    fn sim() -> Simulation {
        Simulation::new(test_course(), MovementConfig::default())
    }

    #[test]
    fn result_is_independent_of_frame_rate() {
        // Same 2 seconds of held-right input at 30, 60 and 144 "FPS".
        let input = PlayerInput {
            move_x: 1.0,
            ..Default::default()
        };
        let run_at = |fps: u32| {
            let mut s = sim();
            // Use whole fixed steps of total time to avoid float drift
            // in the accumulator: feed 120 steps worth of time.
            let frames = fps * 2;
            let dt = 2.0 / frames as f32;
            for _ in 0..frames {
                s.advance(dt, input);
            }
            // Flush any remainder deterministically.
            while s.steps < 120 {
                s.fixed_step(input);
            }
            (s.steps, s.player)
        };
        let (steps60, p60) = run_at(60);
        assert_eq!(steps60, 120);
        for fps in [30, 144] {
            let (steps, p) = run_at(fps);
            assert!(steps == 120, "fps {fps} ran {steps} steps");
            assert_eq!(p, p60, "fps {fps}");
        }
    }

    #[test]
    fn jump_press_is_latched_until_a_step_runs() {
        let mut s = sim();
        // Settle on the ground.
        for _ in 0..10 {
            s.fixed_step(PlayerInput::default());
        }
        assert!(s.player.grounded);
        // Press arrives on a very short frame that runs no step.
        let press = PlayerInput {
            jump_pressed: true,
            jump_held: true,
            ..Default::default()
        };
        assert_eq!(s.advance(0.001, press), 0);
        let held = PlayerInput {
            jump_held: true,
            ..Default::default()
        };
        assert_eq!(s.advance(FIXED_DT, held), 1);
        assert!(s.player.velocity.y > 0.0);
    }

    #[test]
    fn falling_out_of_world_resets_to_spawn() {
        let mut s = sim();
        s.player.position = Vec2::new(40.0 * 16.0, -100.0);
        for _ in 0..120 {
            s.fixed_step(PlayerInput::default());
        }
        assert_eq!(s.resets, 1);
        assert!(s.player.grounded);
        assert_eq!(s.player.position, s.level.spawn);
    }

    #[test]
    fn long_hitch_is_capped() {
        let mut s = sim();
        let n = s.advance(5.0, PlayerInput::default());
        assert_eq!(n, MAX_STEPS_PER_ADVANCE);
        assert!(s.alpha() <= 1.0);
    }
}
