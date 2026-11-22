use super::*;
use crate::geometry::Rect;
use crate::level::{Checkpoint, test_course};

/// Flat strip with spikes, two checkpoints, a pit and a goal.
///
/// x:  0 ... 150-182 spikes ... 220 CP0 ... 260 CP1 ... 300-400 pit ... 700 goal
fn lab() -> Level {
    Level {
        name: "lab",
        solids: vec![
            Rect::new(-16.0, 0.0, 16.0, 200.0),
            Rect::new(0.0, -16.0, 300.0, 16.0),
            Rect::new(400.0, -16.0, 400.0, 16.0),
        ],
        hazards: vec![Rect::new(150.0, 0.0, 32.0, 8.0)],
        checkpoints: vec![
            Checkpoint {
                area: Rect::new(220.0, 0.0, 16.0, 48.0),
                spawn: Vec2::new(228.0, 0.0),
            },
            Checkpoint {
                area: Rect::new(260.0, 0.0, 16.0, 48.0),
                spawn: Vec2::new(268.0, 0.0),
            },
        ],
        spawn: Vec2::new(40.0, 0.0),
        kill_y: -200.0,
        goal: Rect::new(700.0, 0.0, 32.0, 48.0),
    }
}

fn sim() -> Simulation {
    Simulation::new(lab(), MovementConfig::default())
}

fn idle() -> PlayerInput {
    PlayerInput::default()
}

fn right() -> PlayerInput {
    PlayerInput {
        move_x: 1.0,
        ..Default::default()
    }
}

fn put(s: &mut Simulation, x: f32, y: f32) {
    s.player = PlayerState::at(Vec2::new(x, y));
    s.previous_player = s.player;
}

fn steps_for(seconds: f32) -> usize {
    (seconds / FIXED_DT).ceil() as usize + 1
}

// --- fixed-step driver -------------------------------------------------------

#[test]
fn result_is_independent_of_frame_rate() {
    let run_at = |fps: u32| {
        let mut s = Simulation::new(test_course(), MovementConfig::default());
        let frames = fps * 2;
        let dt = 2.0 / frames as f32;
        for _ in 0..frames {
            s.advance(dt, right());
        }
        while s.steps < 120 {
            s.fixed_step(right());
        }
        (s.steps, s.player)
    };
    let (steps60, p60) = run_at(60);
    assert_eq!(steps60, 120);
    for fps in [30, 144] {
        let (steps, p) = run_at(fps);
        assert_eq!(steps, 120, "fps {fps}");
        assert_eq!(p, p60, "fps {fps}");
    }
}

#[test]
fn jump_press_is_latched_until_a_step_runs() {
    let mut s = sim();
    for _ in 0..10 {
        s.fixed_step(idle());
    }
    assert!(s.player.grounded);
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
fn long_hitch_is_capped() {
    let mut s = sim();
    let n = s.advance(5.0, idle());
    assert_eq!(n, MAX_STEPS_PER_ADVANCE);
    assert!(s.alpha() <= 1.0);
}

#[test]
fn pause_freezes_everything_and_drops_time() {
    let mut s = sim();
    s.fixed_step(idle());
    let before = s.player;
    s.set_paused(true);
    assert!(s.is_paused());
    assert_eq!(s.advance(1.0, right()), 0);
    assert_eq!(s.player, before);
    assert_eq!(s.elapsed, FIXED_DT);
    s.set_paused(false);
    // Paused time was not banked: one frame runs one step.
    assert_eq!(s.advance(FIXED_DT, right()), 1);
}

#[test]
fn jump_pressed_while_paused_is_ignored_after_resume() {
    let mut s = sim();
    for _ in 0..5 {
        s.fixed_step(idle());
    }
    s.set_paused(true);
    let press = PlayerInput {
        jump_pressed: true,
        jump_held: true,
        ..Default::default()
    };
    s.advance(FIXED_DT, press);
    s.set_paused(false);
    s.advance(FIXED_DT, idle());
    assert!(s.player.grounded);
}

// --- hazards & death ---------------------------------------------------------

#[test]
fn touching_hazard_kills_exactly_once() {
    let mut s = sim();
    put(&mut s, 166.0, 0.0); // standing in the spikes
    s.fixed_step(idle());
    assert!(matches!(s.phase, Phase::Dying { .. }));
    assert_eq!(s.deaths, 1);
    // Staying in contact during the death animation does not kill again.
    for _ in 0..steps_for(DEATH_DURATION) - 2 {
        s.fixed_step(idle());
        assert_eq!(s.deaths, 1);
    }
    let events = s.take_events();
    let died = events
        .iter()
        .filter(|e| matches!(e, GameEvent::Died { .. }))
        .count();
    assert_eq!(died, 1);
    assert!(matches!(
        events[0],
        GameEvent::Died {
            cause: DeathCause::Hazard,
            ..
        }
    ));
}

#[test]
fn player_is_frozen_while_dying() {
    let mut s = sim();
    put(&mut s, 166.0, 0.0);
    s.fixed_step(right());
    let at = s.player.position;
    for _ in 0..10 {
        s.fixed_step(right());
        assert_eq!(s.player.position, at);
        assert_eq!(s.interpolated_position(), at);
    }
}

#[test]
fn respawn_without_checkpoint_uses_level_spawn() {
    let mut s = sim();
    put(&mut s, 166.0, 0.0);
    for _ in 0..steps_for(DEATH_DURATION) {
        s.fixed_step(idle());
    }
    assert_eq!(s.phase, Phase::Playing);
    assert_eq!(s.player.position, lab().spawn);
    assert_eq!(s.player.velocity, Vec2::ZERO);
    assert!(
        s.take_events()
            .contains(&GameEvent::Respawned { at: lab().spawn })
    );
}

#[test]
fn falling_into_pit_kills_and_respawns_at_checkpoint() {
    let mut s = sim();
    put(&mut s, 228.0, 0.0);
    s.fixed_step(idle());
    assert_eq!(s.active_checkpoint, Some(0));
    put(&mut s, 350.0, 0.0); // over the pit
    let mut died = false;
    for _ in 0..200 {
        s.fixed_step(idle());
        if let Phase::Dying { .. } = s.phase {
            died = true;
            break;
        }
    }
    assert!(died);
    assert!(s.player.position.y < lab().kill_y);
    assert_eq!(s.deaths, 1);
    for _ in 0..steps_for(DEATH_DURATION) {
        s.fixed_step(idle());
    }
    assert_eq!(s.player.position, Vec2::new(228.0, 0.0));
    assert!(s.take_events().iter().any(|e| matches!(
        e,
        GameEvent::Died {
            cause: DeathCause::Fell,
            ..
        }
    )));
}

#[test]
fn respawn_bumps_teleport_counter() {
    let mut s = sim();
    let t0 = s.teleports;
    put(&mut s, 166.0, 0.0);
    for _ in 0..steps_for(DEATH_DURATION) {
        s.fixed_step(idle());
    }
    assert_eq!(s.teleports, t0 + 1);
}

// --- checkpoints -------------------------------------------------------------

#[test]
fn checkpoint_activates_once_while_standing_in_it() {
    let mut s = sim();
    put(&mut s, 228.0, 0.0);
    for _ in 0..30 {
        s.fixed_step(idle());
    }
    assert_eq!(s.active_checkpoint, Some(0));
    let activations = s
        .take_events()
        .into_iter()
        .filter(|e| matches!(e, GameEvent::CheckpointActivated { .. }))
        .count();
    assert_eq!(activations, 1);
}

#[test]
fn running_through_checkpoints_activates_in_order() {
    let mut s = sim();
    put(&mut s, 200.0, 0.0);
    for _ in 0..60 {
        s.fixed_step(right());
    }
    assert_eq!(s.active_checkpoint, Some(1));
    let ev = s.take_events();
    assert_eq!(
        ev.iter()
            .filter(|e| matches!(e, GameEvent::CheckpointActivated { .. }))
            .copied()
            .collect::<Vec<_>>(),
        vec![
            GameEvent::CheckpointActivated { index: 0 },
            GameEvent::CheckpointActivated { index: 1 }
        ]
    );
    assert_eq!(s.respawn_point(), Vec2::new(268.0, 0.0));
}

#[test]
fn going_back_to_earlier_checkpoint_does_not_regress() {
    let mut s = sim();
    put(&mut s, 268.0, 0.0);
    s.fixed_step(idle());
    assert_eq!(s.active_checkpoint, Some(1));
    s.take_events();
    put(&mut s, 228.0, 0.0);
    for _ in 0..10 {
        s.fixed_step(idle());
    }
    assert_eq!(s.active_checkpoint, Some(1));
    assert!(s.take_events().is_empty());
}

#[test]
fn dead_player_does_not_activate_checkpoints() {
    let mut s = sim();
    put(&mut s, 166.0, 0.0);
    s.fixed_step(idle());
    // Teleport onto a checkpoint while dying: must not activate.
    s.player.position = Vec2::new(228.0, 0.0);
    for _ in 0..5 {
        s.fixed_step(idle());
    }
    assert_eq!(s.active_checkpoint, None);
}

// --- goal & restart ----------------------------------------------------------

#[test]
fn reaching_goal_completes_once_and_stops_timer() {
    let mut s = sim();
    put(&mut s, 690.0, 0.0);
    for _ in 0..30 {
        s.fixed_step(right());
    }
    assert!(s.reached_goal());
    let t = s.elapsed;
    for _ in 0..30 {
        s.fixed_step(right());
    }
    assert_eq!(s.elapsed, t);
    let completions = s
        .take_events()
        .into_iter()
        .filter(|e| *e == GameEvent::LevelCompleted)
        .count();
    assert_eq!(completions, 1);
}

#[test]
fn completed_player_cannot_die() {
    let mut s = sim();
    put(&mut s, 690.0, 0.0);
    for _ in 0..30 {
        s.fixed_step(right());
    }
    assert!(s.reached_goal());
    s.player.position = Vec2::new(166.0, 0.0);
    s.fixed_step(idle());
    assert_eq!(s.deaths, 0);
}

#[test]
fn restart_level_clears_progress() {
    let mut s = sim();
    put(&mut s, 268.0, 0.0);
    s.fixed_step(idle());
    put(&mut s, 166.0, 0.0);
    s.fixed_step(idle());
    assert_eq!(s.deaths, 1);
    assert_eq!(s.active_checkpoint, Some(1));
    s.restart_level();
    assert_eq!(s.phase, Phase::Playing);
    assert_eq!(s.active_checkpoint, None);
    assert_eq!(s.deaths, 0);
    assert_eq!(s.elapsed, 0.0);
    assert_eq!(s.restarts, 1);
    assert_eq!(s.player.position, lab().spawn);
    assert!(s.take_events().contains(&GameEvent::LevelRestarted));
    // After a restart, the old checkpoint can be activated again.
    put(&mut s, 228.0, 0.0);
    s.fixed_step(idle());
    assert_eq!(s.active_checkpoint, Some(0));
}

#[test]
fn restart_after_completion_allows_another_run() {
    let mut s = sim();
    put(&mut s, 690.0, 0.0);
    for _ in 0..30 {
        s.fixed_step(right());
    }
    assert!(s.reached_goal());
    s.restart_level();
    assert!(!s.reached_goal());
    for _ in 0..10 {
        s.fixed_step(right());
    }
    assert!(s.elapsed > 0.0);
}

#[test]
fn lifecycle_is_deterministic() {
    let go = || {
        let mut s = sim();
        for i in 0..2000usize {
            let input = PlayerInput {
                move_x: if (i / 90) % 3 == 2 { -1.0 } else { 1.0 },
                jump_pressed: i % 40 == 0,
                jump_held: i % 40 < 15,
            };
            s.fixed_step(input);
        }
        (s.player, s.deaths, s.active_checkpoint, s.phase, s.steps)
    };
    assert_eq!(go(), go());
}

#[test]
fn start_at_checkpoint_places_player_and_marks_progress() {
    let mut s = sim();
    s.start_at_checkpoint(1);
    assert_eq!(s.active_checkpoint, Some(1));
    assert_eq!(s.player.position, Vec2::new(268.0, 0.0));
    s.start_at_checkpoint(99);
    assert_eq!(s.active_checkpoint, Some(1));
}
