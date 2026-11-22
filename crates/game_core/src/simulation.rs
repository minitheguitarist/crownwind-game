//! Fixed-timestep driver and gameplay lifecycle.
//!
//! Renderers feed variable frame times to [`Simulation::advance`]; it runs
//! whole [`FIXED_DT`] steps and exposes an interpolation factor so drawing is
//! decoupled from the simulation rate.
//!
//! Lifecycle (all transitions happen inside fixed steps):
//!
//! ```text
//!   Playing --hazard / fell--> Dying --timer--> Playing (at active checkpoint)
//!   Playing --goal-----------> Completed
//!   any     --restart_level--> Playing (at level spawn, progress cleared)
//! ```

use crate::geometry::Vec2;
use crate::level::Level;
use crate::movement::{FIXED_DT, MovementConfig, PlayerInput, PlayerState, step};

/// Upper bound on steps per `advance` call so a long hitch cannot cause a
/// "spiral of death"; excess time is dropped.
pub const MAX_STEPS_PER_ADVANCE: u32 = 8;

/// Seconds between death and respawn.
pub const DEATH_DURATION: f32 = 0.6;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Phase {
    Playing,
    Dying { remaining: f32 },
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeathCause {
    Hazard,
    Fell,
}

/// Things that happened during simulation, for presentation (effects/HUD).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GameEvent {
    Died { at: Vec2, cause: DeathCause },
    Respawned { at: Vec2 },
    CheckpointActivated { index: usize },
    LevelCompleted,
    LevelRestarted,
}

#[derive(Debug, Clone)]
pub struct Simulation {
    pub config: MovementConfig,
    pub level: Level,
    pub player: PlayerState,
    /// Player state before the most recent step (for render interpolation).
    pub previous_player: PlayerState,
    pub phase: Phase,
    /// Index of the latest activated checkpoint, if any.
    pub active_checkpoint: Option<usize>,
    pub deaths: u32,
    pub restarts: u32,
    /// Increments on every teleport (respawn/restart); lets the renderer snap
    /// its camera instead of panning across the level.
    pub teleports: u32,
    /// Seconds spent in play since the last restart (stops on completion).
    pub elapsed: f32,
    pub steps: u64,
    paused: bool,
    accumulator: f32,
    /// Jump press waiting for the next fixed step.
    pending_jump: bool,
    events: Vec<GameEvent>,
}

impl Simulation {
    pub fn new(level: Level, config: MovementConfig) -> Self {
        let player = PlayerState::at(level.spawn);
        Self {
            config,
            level,
            player,
            previous_player: player,
            phase: Phase::Playing,
            active_checkpoint: None,
            deaths: 0,
            restarts: 0,
            teleports: 0,
            elapsed: 0.0,
            steps: 0,
            paused: false,
            accumulator: 0.0,
            pending_jump: false,
            events: Vec::new(),
        }
    }

    pub fn reached_goal(&self) -> bool {
        self.phase == Phase::Completed
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Pausing freezes the simulation; frame time is discarded, not banked.
    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
        self.accumulator = 0.0;
        self.pending_jump = false;
        self.previous_player = self.player;
    }

    /// Where the player will (re)spawn right now.
    pub fn respawn_point(&self) -> Vec2 {
        self.active_checkpoint
            .map(|i| self.level.checkpoints[i].spawn)
            .unwrap_or(self.level.spawn)
    }

    /// 0 → 1 progress through the death animation, if dying.
    pub fn death_progress(&self) -> Option<f32> {
        match self.phase {
            Phase::Dying { remaining } => Some(1.0 - remaining / DEATH_DURATION),
            _ => None,
        }
    }

    /// Restart the whole level: spawn point, no checkpoints, counters reset.
    pub fn restart_level(&mut self) {
        self.place_player(self.level.spawn);
        self.phase = Phase::Playing;
        self.active_checkpoint = None;
        self.deaths = 0;
        self.elapsed = 0.0;
        self.restarts += 1;
        self.accumulator = 0.0;
        self.pending_jump = false;
        self.events.push(GameEvent::LevelRestarted);
    }

    /// Jump straight to checkpoint `index` as if it had just been activated
    /// (practice / debugging). Out-of-range indices are ignored.
    pub fn start_at_checkpoint(&mut self, index: usize) {
        if index >= self.level.checkpoints.len() {
            return;
        }
        self.active_checkpoint = Some(index);
        self.phase = Phase::Playing;
        let at = self.level.checkpoints[index].spawn;
        self.place_player(at);
    }

    /// Drain the events produced since the last call.
    pub fn take_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.events)
    }

    fn place_player(&mut self, at: Vec2) {
        self.player = PlayerState::at(at);
        self.previous_player = self.player;
        self.teleports += 1;
    }

    fn die(&mut self, cause: DeathCause) {
        self.deaths += 1;
        self.player.velocity = Vec2::ZERO;
        self.phase = Phase::Dying {
            remaining: DEATH_DURATION,
        };
        self.events.push(GameEvent::Died {
            at: self.player.position,
            cause,
        });
    }

    /// Run exactly one fixed step with the given input.
    pub fn fixed_step(&mut self, input: PlayerInput) {
        self.previous_player = self.player;
        self.steps += 1;
        match self.phase {
            Phase::Playing => self.step_playing(input),
            Phase::Dying { remaining } => {
                let remaining = remaining - FIXED_DT;
                if remaining <= 0.0 {
                    let at = self.respawn_point();
                    self.place_player(at);
                    self.phase = Phase::Playing;
                    self.events.push(GameEvent::Respawned { at });
                } else {
                    self.phase = Phase::Dying { remaining };
                }
            }
            Phase::Completed => {
                // Let the hero come to rest naturally; nothing else happens.
                self.player = step(
                    &self.player,
                    PlayerInput::default(),
                    &self.config,
                    &self.level.solids,
                    FIXED_DT,
                );
            }
        }
    }

    fn step_playing(&mut self, input: PlayerInput) {
        self.elapsed += FIXED_DT;
        self.player = step(
            &self.player,
            input,
            &self.config,
            &self.level.solids,
            FIXED_DT,
        );
        let hb = self.player.hitbox(&self.config);

        if hb.overlaps(&self.level.goal) {
            self.phase = Phase::Completed;
            self.events.push(GameEvent::LevelCompleted);
            return;
        }
        if self.level.touches_hazard(&hb) {
            self.die(DeathCause::Hazard);
            return;
        }
        if self.player.position.y < self.level.kill_y {
            self.die(DeathCause::Fell);
            return;
        }
        for (i, cp) in self.level.checkpoints.iter().enumerate() {
            let further = self.active_checkpoint.is_none_or(|a| i > a);
            if further && hb.overlaps(&cp.area) {
                self.active_checkpoint = Some(i);
                self.events
                    .push(GameEvent::CheckpointActivated { index: i });
            }
        }
    }

    /// Accumulate `frame_dt` seconds and run as many fixed steps as fit.
    /// A `jump_pressed` edge is latched until a step consumes it, so a press
    /// on a frame with zero steps is not lost. Returns the number of steps.
    pub fn advance(&mut self, frame_dt: f32, input: PlayerInput) -> u32 {
        if self.paused {
            return 0;
        }
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

    /// Fraction of a step accumulated but not yet simulated, in `[0, 1]`.
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
mod tests;
