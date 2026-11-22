use super::*;
use crate::movement::{MovementConfig, PlayerState, hitbox_at};
use crate::route::find_route;
use crate::simulation::{GameEvent, Phase, Simulation};

const MAX_NODES: usize = 250_000;

fn standing_ok(level: &Level, cfg: &MovementConfig, feet: Vec2, what: &str) {
    let hb = hitbox_at(feet, cfg);
    assert!(
        level.solids.iter().all(|s| !s.overlaps(&hb)),
        "{what} is inside a solid"
    );
    let below = hitbox_at(Vec2::new(feet.x, feet.y - 0.5), cfg);
    assert!(
        level.solids.iter().any(|s| s.overlaps(&below)),
        "{what} is not standing on ground"
    );
    assert!(!level.touches_hazard(&hb), "{what} touches a hazard");
}

#[test]
fn spawn_and_checkpoints_are_safe_standing_spots() {
    let level = tutorial_level();
    let cfg = MovementConfig::default();
    standing_ok(&level, &cfg, level.spawn, "spawn");
    for (i, cp) in level.checkpoints.iter().enumerate() {
        standing_ok(&level, &cfg, cp.spawn, &format!("checkpoint {i}"));
        assert!(cp.area.overlaps(&hitbox_at(cp.spawn, &cfg)));
    }
}

#[test]
fn checkpoints_are_ordered_left_to_right() {
    let level = tutorial_level();
    assert!(level.checkpoints.len() >= 4);
    assert!(
        level
            .checkpoints
            .windows(2)
            .all(|w| w[0].area.x < w[1].area.x)
    );
    assert!(level.checkpoints.last().unwrap().area.x < level.goal.x);
}

#[test]
fn hazards_sit_on_surfaces_and_goal_is_reachable_height() {
    let level = tutorial_level();
    for h in &level.hazards {
        let below = Rect::new(h.x, h.y - 1.0, h.w, 1.0);
        assert!(
            level.solids.iter().any(|s| s.overlaps(&below)),
            "hazard {h:?} floats"
        );
        assert!(
            level.solids.iter().all(|s| !s.overlaps(h)),
            "hazard {h:?} buried"
        );
    }
    assert!(level.goal.bottom() > level.kill_y);
    assert!(level.solids.iter().all(|s| !s.overlaps(&level.goal)));
}

#[test]
fn level_is_long_enough_and_bounded() {
    let level = tutorial_level();
    let b = level.bounds();
    assert!(b.w > 3000.0, "level is only {} px long", b.w);
    // Boundary walls keep the player in.
    assert!(level.spawn.x > b.left() && level.goal.right() < b.right());
}

/// Waypoints in order: every checkpoint, then the goal.
fn waypoints(level: &Level) -> Vec<Rect> {
    let mut w: Vec<Rect> = level.checkpoints.iter().map(|c| c.area).collect();
    w.push(level.goal);
    w
}

/// Find a full route through every waypoint; returns the per-step inputs.
fn full_route(level: &Level, cfg: &MovementConfig) -> Vec<crate::PlayerInput> {
    let mut state = PlayerState::at(level.spawn);
    let mut inputs = Vec::new();
    for (i, target) in waypoints(level).into_iter().enumerate() {
        let route = find_route(level, cfg, state, target, MAX_NODES)
            .unwrap_or_else(|| panic!("no route to waypoint {i} from {:?}", state.position));
        inputs.extend(route.inputs);
        state = route.end;
    }
    inputs
}

#[test]
fn whole_level_is_beatable_by_deterministic_input_replay() {
    let level = tutorial_level();
    let cfg = MovementConfig::default();
    let inputs = full_route(&level, &cfg);

    // Replay the exact inputs through the real Simulation lifecycle.
    let mut sim = Simulation::new(level.clone(), cfg);
    let mut events = Vec::new();
    for input in &inputs {
        sim.fixed_step(*input);
        events.extend(sim.take_events());
    }
    assert_eq!(sim.phase, Phase::Completed, "replay did not finish");
    assert_eq!(sim.deaths, 0);
    assert_eq!(sim.active_checkpoint, Some(level.checkpoints.len() - 1));
    let order: Vec<usize> = events
        .iter()
        .filter_map(|e| match e {
            GameEvent::CheckpointActivated { index } => Some(*index),
            _ => None,
        })
        .collect();
    assert_eq!(order, (0..level.checkpoints.len()).collect::<Vec<_>>());
    // A sane completion time for a tutorial (seconds of game time).
    assert!(sim.elapsed < 120.0, "route takes {} s", sim.elapsed);

    // Replay twice: identical outcome (determinism over a full level).
    let mut again = Simulation::new(level, cfg);
    for input in &inputs {
        again.fixed_step(*input);
    }
    assert_eq!(again.player, sim.player);
    assert_eq!(again.steps, sim.steps);
}

#[test]
fn every_checkpoint_respawn_can_continue() {
    let level = tutorial_level();
    let cfg = MovementConfig::default();
    let targets = waypoints(&level);
    for (i, cp) in level.checkpoints.iter().enumerate() {
        let start = PlayerState::at(cp.spawn);
        assert!(
            find_route(&level, &cfg, start, targets[i + 1], MAX_NODES).is_some(),
            "stuck after respawning at checkpoint {i}"
        );
    }
}

#[test]
fn level_has_margin_for_weaker_physics() {
    // 5% weaker run and jumps must still finish: no pixel-perfect gaps.
    let level = tutorial_level();
    let d = MovementConfig::default();
    let cfg = MovementConfig {
        max_run_speed: d.max_run_speed * 0.95,
        jump_velocity: d.jump_velocity * 0.95,
        wall_jump_velocity: d.wall_jump_velocity * 0.95,
        wall_jump_speed_x: d.wall_jump_speed_x * 0.95,
        ..d
    };
    let _ = full_route(&level, &cfg);
}

/// Prints per-segment statistics of the found route (run with --ignored --nocapture).
#[test]
#[ignore]
fn print_route_stats() {
    let level = tutorial_level();
    let cfg = MovementConfig::default();
    let mut state = PlayerState::at(level.spawn);
    let mut total = 0usize;
    for (i, target) in waypoints(&level).into_iter().enumerate() {
        let r = find_route(&level, &cfg, state, target, MAX_NODES).unwrap();
        let mut s = state;
        let mut wall_jumps = 0;
        let mut jumps = 0;
        for inp in &r.inputs {
            let before = s;
            s = crate::movement::step(&s, *inp, &cfg, &level.solids, crate::FIXED_DT);
            if s.last_wall_jump_dir != before.last_wall_jump_dir && s.last_wall_jump_dir != 0 {
                wall_jumps += 1;
            }
            if s.velocity.y > 0.0
                && before.velocity.y <= 0.0
                && s.last_wall_jump_dir == before.last_wall_jump_dir
            {
                jumps += 1;
            }
        }
        total += r.inputs.len();
        println!(
            "segment {i}: {:.1}s, jumps {jumps}, wall jumps {wall_jumps}, end {:?}",
            r.inputs.len() as f32 / 60.0,
            r.end.position
        );
        state = r.end;
    }
    println!("total {:.1}s", total as f32 / 60.0);
}

/// Movement with wall jumps switched off (wall-coyote window of zero).
fn no_wall_jumps() -> MovementConfig {
    MovementConfig {
        wall_coyote_time: 0.0,
        ..MovementConfig::default()
    }
}

/// Route from checkpoint `from` (None = level spawn) to waypoint `to`.
fn segment(level: &Level, cfg: &MovementConfig, from: Option<usize>, to: usize) -> bool {
    let start = from.map_or(level.spawn, |i| level.checkpoints[i].spawn);
    find_route(
        level,
        cfg,
        PlayerState::at(start),
        waypoints(level)[to],
        MAX_NODES,
    )
    .is_some()
}

#[test]
fn wall_jumps_can_really_be_disabled() {
    let cfg = no_wall_jumps();
    let solids = [
        Rect::new(-100.0, -16.0, 200.0, 16.0),
        Rect::new(30.0, 0.0, 16.0, 300.0),
    ];
    let mut s = PlayerState::at(Vec2::new(24.0, 100.0));
    s = crate::movement::step(
        &s,
        crate::PlayerInput {
            move_x: 1.0,
            ..Default::default()
        },
        &cfg,
        &solids,
        crate::FIXED_DT,
    );
    assert_eq!(s.wall_dir, 1);
    let press = crate::PlayerInput {
        move_x: 1.0,
        jump_pressed: true,
        jump_held: true,
    };
    s = crate::movement::step(&s, press, &cfg, &solids, crate::FIXED_DT);
    assert!(s.velocity.y <= 0.0);
}

#[test]
fn opening_and_landing_areas_need_no_wall_jumps() {
    let level = tutorial_level();
    let cfg = no_wall_jumps();
    assert!(segment(&level, &cfg, None, 0), "area 1 needs wall jumps");
    assert!(segment(&level, &cfg, Some(0), 1), "area 2 needs wall jumps");
}

#[test]
fn hazard_area_and_finale_descent_need_no_wall_jumps() {
    let level = tutorial_level();
    let cfg = no_wall_jumps();
    assert!(segment(&level, &cfg, Some(2), 3), "area 4 needs wall jumps");
    assert!(
        segment(&level, &cfg, Some(4), 5),
        "final descent needs wall jumps"
    );
}

#[test]
fn chimneys_cannot_be_skipped_without_wall_jumps() {
    let level = tutorial_level();
    let cfg = no_wall_jumps();
    assert!(
        !segment(&level, &cfg, Some(1), 2),
        "first chimney is skippable"
    );
    assert!(
        !segment(&level, &cfg, Some(3), 4),
        "second chimney is skippable"
    );
}
