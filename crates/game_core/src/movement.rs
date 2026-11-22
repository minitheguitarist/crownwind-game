//! Deterministic, fixed-step platformer movement and AABB collision.
//!
//! Everything here is plain arithmetic on `f32` with no hidden state, so the
//! same sequence of inputs and steps always produces the same result.
//!
//! Collision is resolved one axis at a time (X, then Y). Each axis move is
//! split into sub-steps of at most [`MAX_SUBSTEP`] pixels so fast motion can
//! never skip through thin geometry.

use crate::geometry::{Rect, Vec2};

/// Fixed simulation rate. Renderers must call [`step`] with this dt (the
/// [`crate::Simulation`] wrapper does that via an accumulator).
pub const FIXED_DT: f32 = 1.0 / 60.0;

/// Largest distance (px) moved per collision sub-step. Must be smaller than
/// the thinnest solid and the player's smallest dimension.
pub const MAX_SUBSTEP: f32 = 4.0;

/// Distance (px) used to probe for walls on either side of the player.
const WALL_PROBE: f32 = 1.0;

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
    /// Maximum downward speed while sliding on a wall (pressing into it).
    pub wall_slide_speed: f32,
    /// Horizontal speed away from the wall given by a wall jump.
    pub wall_jump_speed_x: f32,
    /// Upward speed given by a wall jump.
    pub wall_jump_velocity: f32,
    /// Seconds after a wall jump during which horizontal input is ignored,
    /// so the push-off is always identical.
    pub wall_jump_lock_time: f32,
    /// Seconds after leaving a wall during which a wall jump is still allowed.
    pub wall_coyote_time: f32,
    /// Max horizontal nudge (px) applied when the head clips a ceiling corner.
    pub corner_correction: f32,
}

impl Default for MovementConfig {
    fn default() -> Self {
        Self {
            // Matches the prince art (32×48 cells, pivot (16,44)): legs and
            // torso span pivot −6..+7 px, the head top is 40 px above the
            // ground line (hair). 38 px leaves ≤2 px of hair over ceilings.
            player_width: 14.0,
            player_height: 38.0,
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
            wall_slide_speed: 70.0,
            wall_jump_speed_x: 170.0,
            wall_jump_velocity: 330.0,
            wall_jump_lock_time: 0.14,
            wall_coyote_time: 0.08,
            corner_correction: 4.0,
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
    /// Wall touched while airborne: -1 = on the left, 1 = on the right, 0 = none.
    pub wall_dir: i8,
    /// True when sliding down a wall this step (for visuals).
    pub wall_sliding: bool,
    /// Remaining wall-coyote time and the wall side it refers to.
    pub wall_coyote_timer: f32,
    pub wall_coyote_dir: i8,
    /// Remaining time during which horizontal input is ignored.
    pub wall_jump_lock_timer: f32,
    /// Side of the last wall jumped from since last touching the ground.
    /// A second wall jump from the **same** side is refused; this prevents
    /// climbing a single wall forever. Alternating walls (chimneys) work.
    pub last_wall_jump_dir: i8,
    /// Last horizontal facing: -1 left, 1 right.
    pub facing: i8,
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
            wall_dir: 0,
            wall_sliding: false,
            wall_coyote_timer: 0.0,
            wall_coyote_dir: 0,
            wall_jump_lock_timer: 0.0,
            last_wall_jump_dir: 0,
            facing: 1,
        }
    }

    pub fn hitbox(&self, cfg: &MovementConfig) -> Rect {
        hitbox_at(self.position, cfg)
    }
}

pub fn hitbox_at(feet: Vec2, cfg: &MovementConfig) -> Rect {
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

fn hits_any(hb: &Rect, solids: &[Rect]) -> bool {
    solids.iter().any(|s| hb.overlaps(s))
}

/// Which side wall (if any) is within [`WALL_PROBE`] px of the hitbox.
fn wall_contact(feet: Vec2, cfg: &MovementConfig, solids: &[Rect]) -> i8 {
    let hb = hitbox_at(feet, cfg);
    let shifted = |dx: f32| Rect::new(hb.x + dx, hb.y, hb.w, hb.h);
    if hits_any(&shifted(WALL_PROBE), solids) {
        1
    } else if hits_any(&shifted(-WALL_PROBE), solids) {
        -1
    } else {
        0
    }
}

fn substeps(distance: f32) -> u32 {
    ((distance.abs() / MAX_SUBSTEP).ceil() as u32).max(1)
}

/// Move horizontally by `dx`, stopping flush against the first solid hit.
/// Returns true if a solid was hit.
fn move_x(s: &mut PlayerState, dx: f32, cfg: &MovementConfig, solids: &[Rect]) -> bool {
    if dx == 0.0 {
        return false;
    }
    let n = substeps(dx);
    let piece = dx / n as f32;
    for _ in 0..n {
        s.position.x += piece;
        let hb = hitbox_at(s.position, cfg);
        let mut hit = false;
        for solid in solids {
            if hb.overlaps(solid) {
                hit = true;
                s.position.x = if piece > 0.0 {
                    s.position.x.min(solid.left() - cfg.player_width * 0.5)
                } else {
                    s.position.x.max(solid.right() + cfg.player_width * 0.5)
                };
            }
        }
        if hit {
            return true;
        }
    }
    false
}

/// Sideways offset that frees a head which barely clipped a ceiling corner.
fn corner_nudge(feet: Vec2, cfg: &MovementConfig, solids: &[Rect], blocker: &Rect) -> Option<f32> {
    let hb = hitbox_at(feet, cfg);
    let push_left = hb.right() - blocker.left();
    let push_right = blocker.right() - hb.left();
    for (dx, amount) in [(-push_left, push_left), (push_right, push_right)] {
        if amount > 0.0 && amount <= cfg.corner_correction {
            let moved = Vec2::new(feet.x + dx, feet.y);
            if !hits_any(&hitbox_at(moved, cfg), solids) {
                return Some(dx);
            }
        }
    }
    None
}

/// Move vertically by `dy`. Returns (hit_floor, hit_ceiling).
fn move_y(s: &mut PlayerState, dy: f32, cfg: &MovementConfig, solids: &[Rect]) -> (bool, bool) {
    if dy == 0.0 {
        return (false, false);
    }
    let n = substeps(dy);
    let piece = dy / n as f32;
    for _ in 0..n {
        s.position.y += piece;
        let hb = hitbox_at(s.position, cfg);
        let Some(blocker) = solids.iter().find(|r| hb.overlaps(r)).copied() else {
            continue;
        };
        if piece > 0.0 {
            // Head bump: first try a small sideways nudge around the corner.
            if let Some(dx) = corner_nudge(s.position, cfg, solids, &blocker) {
                s.position.x += dx;
                continue;
            }
            let mut y = s.position.y;
            for solid in solids.iter().filter(|r| hb.overlaps(r)) {
                y = y.min(solid.bottom() - cfg.player_height);
            }
            s.position.y = y;
            return (false, true);
        }
        let mut y = s.position.y;
        for solid in solids.iter().filter(|r| hb.overlaps(r)) {
            y = y.max(solid.top());
        }
        s.position.y = y;
        return (true, false);
    }
    (false, false)
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
    let move_in = input.move_x.clamp(-1.0, 1.0);

    // --- Timers -----------------------------------------------------------
    if s.grounded {
        s.coyote_timer = cfg.coyote_time;
        s.last_wall_jump_dir = 0;
    } else {
        s.coyote_timer = (s.coyote_timer - dt).max(0.0);
    }
    if !s.grounded && s.wall_dir != 0 {
        s.wall_coyote_timer = cfg.wall_coyote_time;
        s.wall_coyote_dir = s.wall_dir;
    } else {
        s.wall_coyote_timer = (s.wall_coyote_timer - dt).max(0.0);
    }
    if input.jump_pressed {
        s.jump_buffer_timer = cfg.jump_buffer_time;
    } else {
        s.jump_buffer_timer = (s.jump_buffer_timer - dt).max(0.0);
    }
    let locked = s.wall_jump_lock_timer > 0.0;
    s.wall_jump_lock_timer = (s.wall_jump_lock_timer - dt).max(0.0);

    // --- Horizontal velocity ---------------------------------------------
    if !locked {
        if move_in > 0.0 {
            s.facing = 1;
        } else if move_in < 0.0 {
            s.facing = -1;
        }
        let target = move_in * cfg.max_run_speed;
        let rate = match (s.grounded, move_in != 0.0) {
            (true, true) => cfg.ground_accel,
            (true, false) => cfg.ground_friction,
            (false, true) => cfg.air_accel,
            (false, false) => cfg.air_friction,
        };
        s.velocity.x = approach(s.velocity.x, target, rate * dt);
    }

    // --- Jumps (ground/coyote first, then wall) ----------------------------
    if s.jump_buffer_timer > 0.0 {
        if s.grounded || s.coyote_timer > 0.0 {
            s.velocity.y = cfg.jump_velocity;
            s.jump_buffer_timer = 0.0;
            s.coyote_timer = 0.0;
            s.wall_coyote_timer = 0.0;
            s.grounded = false;
            s.jumping = true;
        } else if s.wall_coyote_timer > 0.0
            && s.wall_coyote_dir != 0
            && s.wall_coyote_dir != s.last_wall_jump_dir
        {
            let d = s.wall_coyote_dir;
            s.velocity.x = -f32::from(d) * cfg.wall_jump_speed_x;
            s.velocity.y = cfg.wall_jump_velocity;
            s.facing = -d;
            s.jump_buffer_timer = 0.0;
            s.wall_coyote_timer = 0.0;
            s.wall_jump_lock_timer = cfg.wall_jump_lock_time;
            s.last_wall_jump_dir = d;
            s.jumping = true;
        }
    }
    if s.jumping && !input.jump_held && s.velocity.y > 0.0 {
        s.velocity.y *= cfg.jump_cut_multiplier;
        s.jumping = false;
    }
    if s.velocity.y <= 0.0 {
        s.jumping = false;
    }

    // --- Gravity & wall slide ----------------------------------------------
    s.velocity.y = (s.velocity.y - cfg.gravity * dt).max(-cfg.max_fall_speed);
    let pressing_into_wall = s.wall_dir != 0 && move_in * f32::from(s.wall_dir) > 0.0;
    s.wall_sliding = !s.grounded && pressing_into_wall && s.velocity.y < 0.0;
    if s.wall_sliding {
        s.velocity.y = s.velocity.y.max(-cfg.wall_slide_speed);
    }

    // --- Move & collide, one axis at a time --------------------------------
    let dx = s.velocity.x * dt;
    if move_x(&mut s, dx, cfg, solids) {
        s.velocity.x = 0.0;
    }
    let dy = s.velocity.y * dt;
    let (_floor, ceiling) = move_y(&mut s, dy, cfg, solids);
    if ceiling {
        s.velocity.y = 0.0;
        s.jumping = false;
    }
    // Grounded = something directly below the feet (half-pixel probe), so
    // walking along a flat floor never flickers between states.
    let below = hitbox_at(Vec2::new(s.position.x, s.position.y - 0.5), cfg);
    s.grounded = s.velocity.y <= 0.0 && hits_any(&below, solids);
    if s.grounded {
        s.velocity.y = 0.0;
        s.wall_dir = 0;
        s.wall_sliding = false;
    } else {
        s.wall_dir = wall_contact(s.position, cfg, solids);
    }

    s
}

#[cfg(test)]
mod tests;
