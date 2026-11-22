//! Deterministic, fixed-step platformer movement and AABB collision.
//!
//! Everything here is plain arithmetic on `f32` with no hidden state, so the
//! same sequence of inputs and steps always produces the same result.

use crate::geometry::{Rect, Vec2};

/// Fixed simulation rate. Renderers must call [`step`] with this dt (the
/// [`crate::Simulation`] wrapper does that via an accumulator).
pub const FIXED_DT: f32 = 1.0 / 60.0;

/// Tuning values. Units: pixels and seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MovementConfig {
    /// Player hitbox size in pixels (feet are at `position.y`).
    pub player_width: f32,
    pub player_height: f32,
    pub max_run_speed: f32,
    pub ground_accel: f32,
    pub ground_friction: f32,
    pub air_accel: f32,
    pub air_friction: f32,
    pub gravity: f32,
    pub max_fall_speed: f32,
    pub jump_velocity: f32,
    /// Upward velocity is multiplied by this when jump is released early.
    pub jump_cut_multiplier: f32,
    /// Seconds after walking off a ledge during which a jump is still allowed.
    pub coyote_time: f32,
    /// Seconds a jump press is remembered while airborne.
    pub jump_buffer_time: f32,
}

impl Default for MovementConfig {
    fn default() -> Self {
        Self {
            player_width: 12.0,
            player_height: 24.0,
            max_run_speed: 130.0,
            ground_accel: 1400.0,
            ground_friction: 1600.0,
            air_accel: 900.0,
            air_friction: 250.0,
            gravity: 1000.0,
            max_fall_speed: 420.0,
            // Peak height = v^2 / 2g ≈ 61 px ≈ 3.8 tiles of 16 px.
            jump_velocity: 350.0,
            jump_cut_multiplier: 0.45,
            coyote_time: 0.1,
            jump_buffer_time: 0.12,
        }
    }
}

/// Per-step input sample.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PlayerInput {
    /// Desired horizontal direction in `[-1, 1]`.
    pub move_x: f32,
    /// True on the step the jump button was pressed (edge, not level).
    pub jump_pressed: bool,
    /// True while the jump button is held (used for variable jump height).
    pub jump_held: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerState {
    /// Bottom-center of the hitbox ("feet"), in world pixels.
    pub position: Vec2,
    pub velocity: Vec2,
    pub grounded: bool,
    /// Remaining coyote time in seconds.
    pub coyote_timer: f32,
    /// Remaining jump-buffer time in seconds.
    pub jump_buffer_timer: f32,
    /// True while rising from a jump that may still be cut short.
    pub jumping: bool,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self::at(Vec2::ZERO)
    }
}

impl PlayerState {
    pub fn at(position: Vec2) -> Self {
        Self {
            position,
            velocity: Vec2::ZERO,
            grounded: false,
            coyote_timer: 0.0,
            jump_buffer_timer: 0.0,
            jumping: false,
        }
    }

    pub fn hitbox(&self, cfg: &MovementConfig) -> Rect {
        hitbox_at(self.position, cfg)
    }
}

fn hitbox_at(feet: Vec2, cfg: &MovementConfig) -> Rect {
    Rect::new(
        feet.x - cfg.player_width * 0.5,
        feet.y,
        cfg.player_width,
        cfg.player_height,
    )
}

fn approach(current: f32, target: f32, max_delta: f32) -> f32 {
    if current < target {
        (current + max_delta).min(target)
    } else {
        (current - max_delta).max(target)
    }
}

/// Advance the player by one fixed step of `dt` seconds against `solids`.
pub fn step(
    state: &PlayerState,
    input: PlayerInput,
    cfg: &MovementConfig,
    solids: &[Rect],
    dt: f32,
) -> PlayerState {
    let mut s = *state;
    let move_x = input.move_x.clamp(-1.0, 1.0);

    // --- Timers -----------------------------------------------------------
    if s.grounded {
        s.coyote_timer = cfg.coyote_time;
    } else {
        s.coyote_timer = (s.coyote_timer - dt).max(0.0);
    }
    if input.jump_pressed {
        s.jump_buffer_timer = cfg.jump_buffer_time;
    } else {
        s.jump_buffer_timer = (s.jump_buffer_timer - dt).max(0.0);
    }

    // --- Horizontal velocity ---------------------------------------------
    let target = move_x * cfg.max_run_speed;
    let rate = match (s.grounded, move_x != 0.0) {
        (true, true) => cfg.ground_accel,
        (true, false) => cfg.ground_friction,
        (false, true) => cfg.air_accel,
        (false, false) => cfg.air_friction,
    };
    s.velocity.x = approach(s.velocity.x, target, rate * dt);

    // --- Jump --------------------------------------------------------------
    if s.jump_buffer_timer > 0.0 && (s.grounded || s.coyote_timer > 0.0) {
        s.velocity.y = cfg.jump_velocity;
        s.jump_buffer_timer = 0.0;
        s.coyote_timer = 0.0;
        s.grounded = false;
        s.jumping = true;
    }
    if s.jumping && !input.jump_held && s.velocity.y > 0.0 {
        s.velocity.y *= cfg.jump_cut_multiplier;
        s.jumping = false;
    }
    if s.velocity.y <= 0.0 {
        s.jumping = false;
    }

    // --- Gravity -----------------------------------------------------------
    s.velocity.y = (s.velocity.y - cfg.gravity * dt).max(-cfg.max_fall_speed);

    // --- Move & collide, one axis at a time --------------------------------
    s.position.x += s.velocity.x * dt;
    let mut hb = hitbox_at(s.position, cfg);
    for solid in solids {
        if hb.overlaps(solid) {
            if s.velocity.x > 0.0 {
                s.position.x = solid.left() - cfg.player_width * 0.5;
            } else if s.velocity.x < 0.0 {
                s.position.x = solid.right() + cfg.player_width * 0.5;
            }
            s.velocity.x = 0.0;
            hb = hitbox_at(s.position, cfg);
        }
    }

    s.position.y += s.velocity.y * dt;
    hb = hitbox_at(s.position, cfg);
    s.grounded = false;
    for solid in solids {
        if hb.overlaps(solid) {
            if s.velocity.y < 0.0 {
                s.position.y = solid.top();
                s.grounded = true;
            } else if s.velocity.y > 0.0 {
                s.position.y = solid.bottom() - cfg.player_height;
                s.jumping = false;
            }
            s.velocity.y = 0.0;
            hb = hitbox_at(s.position, cfg);
        }
    }

    s
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = FIXED_DT;

    fn floor() -> Rect {
        Rect::new(-1000.0, -16.0, 2000.0, 16.0)
    }

    fn run(mut s: PlayerState, input: PlayerInput, solids: &[Rect], steps: usize) -> PlayerState {
        let cfg = MovementConfig::default();
        for _ in 0..steps {
            s = step(&s, input, &cfg, solids, DT);
        }
        s
    }

    fn settled_on_floor() -> PlayerState {
        run(
            PlayerState::at(Vec2::new(0.0, 0.0)),
            PlayerInput::default(),
            &[floor()],
            2,
        )
    }

    #[test]
    fn falling_player_lands_on_floor_top() {
        let s = run(
            PlayerState::at(Vec2::new(0.0, 100.0)),
            PlayerInput::default(),
            &[floor()],
            120,
        );
        assert!(s.grounded);
        assert_eq!(s.position.y, 0.0);
        assert_eq!(s.velocity.y, 0.0);
    }

    #[test]
    fn resting_player_stays_grounded_and_still() {
        let s = run(settled_on_floor(), PlayerInput::default(), &[floor()], 300);
        assert!(s.grounded);
        assert_eq!(s.position, Vec2::new(0.0, 0.0));
    }

    #[test]
    fn running_reaches_but_never_exceeds_max_speed() {
        let cfg = MovementConfig::default();
        let input = PlayerInput {
            move_x: 1.0,
            ..Default::default()
        };
        let s = run(settled_on_floor(), input, &[floor()], 60);
        assert_eq!(s.velocity.x, cfg.max_run_speed);
        // Friction brings it back to rest.
        let s = run(s, PlayerInput::default(), &[floor()], 60);
        assert_eq!(s.velocity.x, 0.0);
    }

    #[test]
    fn wall_blocks_horizontal_motion() {
        let cfg = MovementConfig::default();
        let wall = Rect::new(40.0, 0.0, 16.0, 64.0);
        let input = PlayerInput {
            move_x: 1.0,
            ..Default::default()
        };
        let s = run(settled_on_floor(), input, &[floor(), wall], 180);
        assert_eq!(s.position.x, wall.left() - cfg.player_width * 0.5);
        assert!(s.hitbox(&cfg).right() <= wall.left());
        assert!(s.grounded);
    }

    #[test]
    fn wall_blocks_from_the_right_side_too() {
        let cfg = MovementConfig::default();
        let wall = Rect::new(-56.0, 0.0, 16.0, 64.0);
        let input = PlayerInput {
            move_x: -1.0,
            ..Default::default()
        };
        let s = run(settled_on_floor(), input, &[floor(), wall], 180);
        assert_eq!(s.hitbox(&cfg).left(), wall.right());
    }

    #[test]
    fn ceiling_stops_upward_motion() {
        let cfg = MovementConfig::default();
        let ceiling = Rect::new(-50.0, 40.0, 100.0, 16.0);
        let mut s = settled_on_floor();
        let press = PlayerInput {
            jump_pressed: true,
            jump_held: true,
            ..Default::default()
        };
        let hold = PlayerInput {
            jump_held: true,
            ..Default::default()
        };
        s = step(&s, press, &cfg, &[floor(), ceiling], DT);
        let mut max_top = s.hitbox(&cfg).top();
        for _ in 0..60 {
            s = step(&s, hold, &cfg, &[floor(), ceiling], DT);
            max_top = max_top.max(s.hitbox(&cfg).top());
        }
        assert!(max_top <= ceiling.bottom());
        assert!(s.grounded);
    }

    #[test]
    fn jump_from_ground_rises_and_lands() {
        let cfg = MovementConfig::default();
        let mut s = settled_on_floor();
        s = step(
            &s,
            PlayerInput {
                jump_pressed: true,
                jump_held: true,
                ..Default::default()
            },
            &cfg,
            &[floor()],
            DT,
        );
        assert!(!s.grounded);
        assert!(s.velocity.y > 0.0);
        let held = PlayerInput {
            jump_held: true,
            ..Default::default()
        };
        let mut peak: f32 = 0.0;
        for _ in 0..120 {
            s = step(&s, held, &cfg, &[floor()], DT);
            peak = peak.max(s.position.y);
        }
        assert!(peak > 50.0, "peak {peak}");
        assert!(s.grounded);
    }

    #[test]
    fn releasing_jump_early_gives_lower_jump() {
        let cfg = MovementConfig::default();
        let press = PlayerInput {
            jump_pressed: true,
            jump_held: true,
            ..Default::default()
        };
        let peak_with = |hold_steps: usize| {
            let mut s = step(&settled_on_floor(), press, &cfg, &[floor()], DT);
            let mut peak: f32 = 0.0;
            for i in 0..120 {
                let input = PlayerInput {
                    jump_held: i < hold_steps,
                    ..Default::default()
                };
                s = step(&s, input, &cfg, &[floor()], DT);
                peak = peak.max(s.position.y);
            }
            peak
        };
        assert!(peak_with(3) < peak_with(60) * 0.6);
    }

    #[test]
    fn buffered_jump_triggers_on_landing() {
        let cfg = MovementConfig::default();
        // Fall from a small height, press jump a few steps before landing.
        let mut s = PlayerState::at(Vec2::new(0.0, 6.0));
        s.velocity.y = -200.0;
        // Pressing now: we are airborne with no coyote time, so no jump yet.
        s = step(
            &s,
            PlayerInput {
                jump_pressed: true,
                jump_held: true,
                ..Default::default()
            },
            &cfg,
            &[floor()],
            DT,
        );
        // Landing happens on this or the next step; the buffered press must
        // make the player leave the ground within the buffer window.
        let held = PlayerInput {
            jump_held: true,
            ..Default::default()
        };
        let window = (cfg.jump_buffer_time / DT) as usize;
        let mut jumped = false;
        for _ in 0..window {
            s = step(&s, held, &cfg, &[floor()], DT);
            if s.velocity.y > 0.0 {
                jumped = true;
                break;
            }
        }
        assert!(jumped, "buffered jump did not fire");
    }

    #[test]
    fn stale_jump_buffer_expires() {
        let cfg = MovementConfig::default();
        let mut s = PlayerState::at(Vec2::new(0.0, 200.0));
        s = step(
            &s,
            PlayerInput {
                jump_pressed: true,
                jump_held: true,
                ..Default::default()
            },
            &cfg,
            &[floor()],
            DT,
        );
        // Long fall: buffer expires well before landing.
        s = run(s, PlayerInput::default(), &[floor()], 120);
        assert!(s.grounded);
        assert_eq!(s.velocity.y, 0.0);
        assert_eq!(s.position.y, 0.0);
    }

    fn ledge() -> Rect {
        // Platform ending at x = 0; player walks right off it.
        Rect::new(-200.0, -16.0, 200.0, 16.0)
    }

    /// Walk right off the ledge; returns state on the first airborne step.
    fn walk_off_ledge() -> PlayerState {
        let cfg = MovementConfig::default();
        let mut s = PlayerState::at(Vec2::new(-20.0, 0.0));
        s = step(&s, PlayerInput::default(), &cfg, &[ledge()], DT);
        assert!(s.grounded);
        let right = PlayerInput {
            move_x: 1.0,
            ..Default::default()
        };
        for _ in 0..120 {
            s = step(&s, right, &cfg, &[ledge()], DT);
            if !s.grounded {
                return s;
            }
        }
        panic!("never left the ledge");
    }

    #[test]
    fn coyote_time_allows_late_jump() {
        let cfg = MovementConfig::default();
        let mut s = walk_off_ledge();
        // A couple of steps in the air, still inside the coyote window.
        s = step(&s, PlayerInput::default(), &cfg, &[ledge()], DT);
        assert!(!s.grounded);
        assert!(s.position.y < 0.0);
        s = step(
            &s,
            PlayerInput {
                jump_pressed: true,
                jump_held: true,
                ..Default::default()
            },
            &cfg,
            &[ledge()],
            DT,
        );
        assert!(s.velocity.y > 0.0, "coyote jump should fire");
    }

    #[test]
    fn coyote_time_expires() {
        let cfg = MovementConfig::default();
        let mut s = walk_off_ledge();
        let steps = (cfg.coyote_time / DT) as usize + 2;
        s = run(s, PlayerInput::default(), &[ledge()], steps);
        s = step(
            &s,
            PlayerInput {
                jump_pressed: true,
                jump_held: true,
                ..Default::default()
            },
            &cfg,
            &[ledge()],
            DT,
        );
        assert!(s.velocity.y < 0.0, "late jump must not fire");
    }

    #[test]
    fn cannot_double_jump_in_air() {
        let cfg = MovementConfig::default();
        let press = PlayerInput {
            jump_pressed: true,
            jump_held: true,
            ..Default::default()
        };
        let mut s = step(&settled_on_floor(), press, &cfg, &[floor()], DT);
        s = run(s, PlayerInput::default(), &[floor()], 15);
        let vy_before = s.velocity.y;
        s = step(&s, press, &cfg, &[floor()], DT);
        assert!(s.velocity.y < vy_before);
    }

    #[test]
    fn fall_speed_is_clamped() {
        let cfg = MovementConfig::default();
        let s = run(
            PlayerState::at(Vec2::new(0.0, 10_000.0)),
            PlayerInput::default(),
            &[],
            300,
        );
        assert_eq!(s.velocity.y, -cfg.max_fall_speed);
    }

    #[test]
    fn simulation_is_deterministic() {
        let script = |i: usize| PlayerInput {
            move_x: if i % 50 < 30 { 1.0 } else { -0.5 },
            jump_pressed: i.is_multiple_of(37),
            jump_held: i % 37 < 10,
        };
        let solids = [floor(), Rect::new(80.0, 0.0, 16.0, 32.0)];
        let cfg = MovementConfig::default();
        let go = || {
            let mut s = settled_on_floor();
            for i in 0..600 {
                s = step(&s, script(i), &cfg, &solids, DT);
            }
            s
        };
        assert_eq!(go(), go());
    }
}
