//! Prince animation: pure state selection + time-based frame stepping, and
//! the Bevy system that applies it to the sprite.
//!
//! Animation reads gameplay state; it never feeds back into the simulation.
//! Frames advance with real elapsed time (`Time::delta_secs`), so playback
//! speed does not depend on the display refresh rate.

use bevy::prelude::*;
use game_core as gc;

use crate::GameSim;
use crate::manifest::{AnimationManifest, Clip};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimState {
    Idle,
    Run,
    Jump,
    Fall,
}

impl AnimState {
    pub fn clip_name(self) -> &'static str {
        match self {
            AnimState::Idle => "idle",
            AnimState::Run => "run",
            AnimState::Jump => "jump",
            AnimState::Fall => "fall",
        }
    }
}

/// Horizontal speed (px/s) above which the prince starts running...
const RUN_ENTER_SPEED: f32 = 20.0;
/// ...and below which a running prince returns to idle (hysteresis).
const RUN_EXIT_SPEED: f32 = 8.0;
/// Small downward speeds right after leaving the ground (stepping off a
/// ledge or down a stair) keep the ground pose for a moment: no flicker.
const LEDGE_GRACE_FALL_SPEED: f32 = -80.0;

/// True when the player stands on a solid, even before the simulation has
/// taken its first step (a freshly spawned `PlayerState` is not yet marked
/// `grounded`, which would otherwise flash the fall pose at spawn/respawn).
pub fn standing(p: &gc::PlayerState, level: &gc::Level, cfg: &gc::MovementConfig) -> bool {
    if p.grounded {
        return true;
    }
    if p.velocity.y != 0.0 {
        return false;
    }
    let probe = gc::movement::hitbox_at(gc::Vec2::new(p.position.x, p.position.y - 0.5), cfg);
    level.solids.iter().any(|s| s.overlaps(&probe))
}

/// Pick the animation for the current player state. `grounded` should come
/// from [`standing`].
pub fn choose_state(prev: AnimState, p: &gc::PlayerState, grounded: bool) -> AnimState {
    let on_ground_pose = matches!(prev, AnimState::Idle | AnimState::Run);
    let grace = !grounded
        && on_ground_pose
        && p.coyote_timer > 0.0
        && p.velocity.y <= 0.0
        && p.velocity.y > LEDGE_GRACE_FALL_SPEED;
    if grounded || grace {
        let speed = p.velocity.x.abs();
        let running = if prev == AnimState::Run {
            speed > RUN_EXIT_SPEED
        } else {
            speed > RUN_ENTER_SPEED
        };
        if running {
            AnimState::Run
        } else {
            AnimState::Idle
        }
    } else if p.velocity.y > 0.0 {
        AnimState::Jump
    } else {
        AnimState::Fall
    }
}

/// Atlas cell for `clip` after `time` seconds of playback.
pub fn clip_frame(clip: &Clip, time: f32) -> usize {
    let n = (time.max(0.0) * clip.fps).floor() as usize;
    let i = if clip.looping {
        n % clip.frames.len()
    } else {
        n.min(clip.frames.len() - 1)
    };
    clip.frames[i]
}

/// Offset of a center-anchored frame so the manifest pivot lands on the
/// simulation's feet point. The pivot `y` is the sole pixel row (verified in
/// the atlas: grounded frames' lowest opaque row is 44), so that row sits
/// directly on the ground line instead of one pixel inside it.
pub fn sprite_offset(m: &AnimationManifest) -> Vec2 {
    let (px, py) = m.pivot;
    Vec2::new(
        m.frame_width as f32 * 0.5 - px,
        py + 1.0 - m.frame_height as f32 * 0.5,
    )
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Animator {
    pub state: AnimState,
    pub time: f32,
}

impl Default for Animator {
    fn default() -> Self {
        Self {
            state: AnimState::Idle,
            time: 0.0,
        }
    }
}

impl Animator {
    /// Advance by `dt` seconds; switching state restarts the clip.
    pub fn update(&mut self, next: AnimState, dt: f32) {
        if next != self.state {
            self.state = next;
            self.time = 0.0;
        } else {
            self.time += dt.max(0.0);
        }
    }

    pub fn frame(&self, m: &AnimationManifest) -> usize {
        m.clip(self.state.clip_name())
            .map_or(0, |c| clip_frame(c, self.time))
    }
}

/// Bevy resource holding the parsed manifest.
#[derive(Resource)]
pub struct PrinceManifest(pub AnimationManifest);

#[derive(Component, Default)]
pub struct PrinceSprite {
    pub animator: Animator,
}

pub fn animate_prince(
    time: Res<Time>,
    sim: Res<GameSim>,
    manifest: Res<PrinceManifest>,
    mut q: Query<(&mut Sprite, &mut Transform, &mut PrinceSprite)>,
) {
    let m = &manifest.0;
    let dt = if sim.0.is_paused() {
        0.0
    } else {
        time.delta_secs()
    };
    let p = &sim.0.player;
    let offset = sprite_offset(m);
    for (mut sprite, mut t, mut prince) in &mut q {
        let grounded = standing(p, &sim.0.level, &sim.0.config);
        let next = choose_state(prince.animator.state, p, grounded);
        prince.animator.update(next, dt);
        let index = prince.animator.frame(m);
        if let Some(atlas) = sprite.texture_atlas.as_mut()
            && atlas.index != index
        {
            atlas.index = index;
        }
        let flip = p.facing < 0;
        if sprite.flip_x != flip {
            sprite.flip_x = flip;
        }
        // Mirror the pivot offset with the sprite.
        t.translation.x = if flip { -offset.x } else { offset.x };
        t.translation.y = offset.y;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn player() -> gc::PlayerState {
        let mut p = gc::PlayerState::at(gc::Vec2::ZERO);
        p.grounded = true;
        p
    }

    #[test]
    fn grounded_states() {
        let mut p = player();
        assert_eq!(
            choose_state(AnimState::Idle, &p, p.grounded),
            AnimState::Idle
        );
        p.velocity.x = 130.0;
        assert_eq!(
            choose_state(AnimState::Idle, &p, p.grounded),
            AnimState::Run
        );
        // Hysteresis: slowing to 12 px/s keeps running, 5 px/s stops.
        p.velocity.x = 12.0;
        assert_eq!(choose_state(AnimState::Run, &p, p.grounded), AnimState::Run);
        assert_eq!(
            choose_state(AnimState::Idle, &p, p.grounded),
            AnimState::Idle
        );
        p.velocity.x = -5.0;
        assert_eq!(
            choose_state(AnimState::Run, &p, p.grounded),
            AnimState::Idle
        );
    }

    #[test]
    fn airborne_states() {
        let mut p = player();
        p.grounded = false;
        p.velocity.y = 200.0;
        assert_eq!(
            choose_state(AnimState::Run, &p, p.grounded),
            AnimState::Jump
        );
        p.velocity.y = -200.0;
        assert_eq!(
            choose_state(AnimState::Jump, &p, p.grounded),
            AnimState::Fall
        );
    }

    #[test]
    fn stepping_off_a_ledge_keeps_ground_pose_briefly() {
        let mut p = player();
        p.grounded = false;
        p.coyote_timer = 0.08;
        p.velocity.x = 130.0;
        p.velocity.y = -20.0;
        assert_eq!(choose_state(AnimState::Run, &p, p.grounded), AnimState::Run);
        p.velocity.y = -150.0;
        assert_eq!(
            choose_state(AnimState::Run, &p, p.grounded),
            AnimState::Fall
        );
        // No grace once already airborne.
        p.velocity.y = -20.0;
        assert_eq!(
            choose_state(AnimState::Fall, &p, p.grounded),
            AnimState::Fall
        );
    }

    #[test]
    fn fresh_spawn_on_ground_shows_idle_not_fall() {
        let level = gc::tutorial_level();
        let cfg = gc::MovementConfig::default();
        let p = gc::PlayerState::at(level.spawn); // grounded == false
        assert!(!p.grounded);
        assert!(standing(&p, &level, &cfg));
        assert_eq!(
            choose_state(AnimState::Fall, &p, standing(&p, &level, &cfg)),
            AnimState::Idle
        );
        // Same position but falling: not standing.
        let mut q = p;
        q.position.y += 40.0;
        q.velocity.y = -10.0;
        assert!(!standing(&q, &level, &cfg));
    }

    #[test]
    fn frames_follow_manifest_timing() {
        let m = AnimationManifest::prince();
        let run = m.clip("run").unwrap(); // 10 fps, 4..=9, loop
        assert_eq!(clip_frame(run, 0.0), 4);
        assert_eq!(clip_frame(run, 0.099), 4);
        assert_eq!(clip_frame(run, 0.1), 5);
        assert_eq!(clip_frame(run, 0.55), 9);
        assert_eq!(clip_frame(run, 0.6), 4); // wraps
        let jump = m.clip("jump").unwrap(); // non-looping holds last frame
        assert_eq!(clip_frame(jump, 0.0), 10);
        assert_eq!(clip_frame(jump, 5.0), 11);
        let fall = m.clip("fall").unwrap();
        assert_eq!(clip_frame(fall, 5.0), 13);
    }

    #[test]
    fn every_reachable_frame_is_inside_its_clip_and_atlas() {
        let m = AnimationManifest::prince();
        for name in ["idle", "run", "jump", "fall"] {
            let clip = m.clip(name).unwrap();
            for i in 0..200 {
                let f = clip_frame(clip, i as f32 * 0.013);
                assert!(clip.frames.contains(&f));
                assert!(f < crate::manifest::PRINCE_ATLAS_COLUMNS as usize);
            }
        }
    }

    #[test]
    fn playback_speed_is_independent_of_frame_rate() {
        let m = AnimationManifest::prince();
        // 1.05 s keeps the total away from a frame boundary (10 fps clip), so
        // float rounding of the summed frame times cannot flip the result.
        let at = |fps: u32| {
            let mut a = Animator::default();
            a.update(AnimState::Run, 0.0);
            for _ in 0..fps {
                a.update(AnimState::Run, 1.05 / fps as f32);
            }
            a.frame(&m)
        };
        // After the same elapsed time every refresh rate shows the same cell.
        assert_eq!(at(30), at(60));
        assert_eq!(at(60), at(144));
    }

    #[test]
    fn state_change_restarts_clip() {
        let mut a = Animator::default();
        a.update(AnimState::Idle, 0.7);
        a.update(AnimState::Idle, 0.7);
        assert!(a.time > 0.0);
        a.update(AnimState::Jump, 0.016);
        assert_eq!(a.state, AnimState::Jump);
        assert_eq!(a.time, 0.0);
    }

    #[test]
    fn pivot_offset_puts_sole_row_on_feet_with_integer_edges() {
        let m = AnimationManifest::prince();
        let o = sprite_offset(&m);
        assert_eq!(o, Vec2::new(0.0, 21.0));
        // Bottom of pixel row 44 = center - h/2 + (h - 45) = feet (0).
        let bottom_of_sole = o.y - m.frame_height as f32 * 0.5 + (m.frame_height as f32 - 45.0);
        assert_eq!(bottom_of_sole, 0.0);
        // Even frame size + integer offset => sprite edges on the pixel grid.
        assert_eq!(m.frame_width % 2, 0);
        assert_eq!(m.frame_height % 2, 0);
        assert_eq!(o.fract(), Vec2::ZERO);
    }
}
