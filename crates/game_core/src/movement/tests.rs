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

// ---------------------------------------------------------------------------
// Wall slide / wall jump
// ---------------------------------------------------------------------------

const PRESS: PlayerInput = PlayerInput {
    move_x: 0.0,
    jump_pressed: true,
    jump_held: true,
};

fn input(move_x: f32, jump_pressed: bool, jump_held: bool) -> PlayerInput {
    PlayerInput {
        move_x,
        jump_pressed,
        jump_held,
    }
}

/// Floor plus two tall walls with a 48 px gap centered on x = 0.
fn chimney() -> Vec<Rect> {
    vec![
        floor(),
        Rect::new(-40.0, 0.0, 16.0, 400.0),
        Rect::new(24.0, 0.0, 16.0, 400.0),
    ]
}

/// Single tall wall on the right of the player, at x = 30.
fn right_wall() -> Vec<Rect> {
    vec![floor(), Rect::new(30.0, 0.0, 16.0, 400.0)]
}

/// Airborne player pressed against the right wall, falling.
fn airborne_on_right_wall(solids: &[Rect]) -> PlayerState {
    let cfg = MovementConfig::default();
    let mut s = PlayerState::at(Vec2::new(30.0 - cfg.player_width * 0.5, 120.0));
    s = step(&s, input(1.0, false, false), &cfg, solids, DT);
    assert_eq!(s.wall_dir, 1);
    assert!(!s.grounded);
    s
}

#[test]
fn wall_contact_is_detected_on_both_sides() {
    let cfg = MovementConfig::default();
    let solids = chimney();
    let mut s = PlayerState::at(Vec2::new(-24.0 + 6.0, 100.0));
    s = step(&s, PlayerInput::default(), &cfg, &solids, DT);
    assert_eq!(s.wall_dir, -1);
    let mut s = PlayerState::at(Vec2::new(24.0 - 6.0, 100.0));
    s = step(&s, PlayerInput::default(), &cfg, &solids, DT);
    assert_eq!(s.wall_dir, 1);
    let mut s = PlayerState::at(Vec2::new(0.0, 100.0));
    s = step(&s, PlayerInput::default(), &cfg, &solids, DT);
    assert_eq!(s.wall_dir, 0);
}

#[test]
fn grounded_player_next_to_wall_is_not_on_wall() {
    let s = run(
        settled_on_floor(),
        input(1.0, false, false),
        &right_wall(),
        120,
    );
    assert!(s.grounded);
    assert_eq!(s.wall_dir, 0);
    assert!(!s.wall_sliding);
}

#[test]
fn wall_slide_caps_fall_speed_only_when_pressing_into_wall() {
    let cfg = MovementConfig::default();
    let solids = right_wall();
    let start = airborne_on_right_wall(&solids);

    let sliding = run(start, input(1.0, false, false), &solids, 40);
    assert!(sliding.wall_sliding);
    assert_eq!(sliding.velocity.y, -cfg.wall_slide_speed);
    assert_eq!(sliding.wall_dir, 1);

    let free = run(start, input(0.0, false, false), &solids, 20);
    assert!(!free.grounded);
    assert!(!free.wall_sliding);
    assert!(free.velocity.y < -cfg.wall_slide_speed);
}

#[test]
fn wall_jump_gives_exact_predictable_impulse() {
    let cfg = MovementConfig::default();
    let solids = right_wall();
    let s0 = run(
        airborne_on_right_wall(&solids),
        input(1.0, false, false),
        &solids,
        10,
    );
    // Still holding toward the wall while pressing jump.
    let s = step(&s0, input(1.0, true, true), &cfg, &solids, DT);
    assert_eq!(s.velocity.x, -cfg.wall_jump_speed_x);
    assert_eq!(s.velocity.y, cfg.wall_jump_velocity - cfg.gravity * DT);
    assert_eq!(s.facing, -1);
    assert_eq!(s.last_wall_jump_dir, 1);

    // During the lock window input cannot cancel the push-off.
    let lock_steps = (cfg.wall_jump_lock_time / DT) as usize;
    let mut t = s;
    for _ in 0..lock_steps {
        t = step(&t, input(1.0, false, true), &cfg, &solids, DT);
        assert_eq!(t.velocity.x, -cfg.wall_jump_speed_x);
    }
    // Identical regardless of the held direction.
    let mut u = s;
    for _ in 0..lock_steps {
        u = step(&u, input(-1.0, false, true), &cfg, &solids, DT);
    }
    assert_eq!(t.position, u.position);
}

#[test]
fn wall_jump_from_same_wall_twice_is_refused() {
    let cfg = MovementConfig::default();
    let solids = right_wall();
    let mut s = airborne_on_right_wall(&solids);
    s.position.y += 200.0; // plenty of room to return before landing
    s = run(s, input(1.0, false, false), &solids, 5);
    s = step(&s, input(1.0, true, true), &cfg, &solids, DT);
    assert!(s.velocity.y > 0.0);
    // Steer back to the same wall and try again.
    for _ in 0..120 {
        s = step(&s, input(1.0, false, true), &cfg, &solids, DT);
        if s.wall_dir == 1 {
            break;
        }
    }
    assert_eq!(s.wall_dir, 1, "should be back on the wall");
    assert!(!s.grounded);
    for _ in 0..10 {
        s = step(&s, input(1.0, true, true), &cfg, &solids, DT);
        assert!(s.velocity.y <= 0.0, "second same-side wall jump fired");
        assert_ne!(s.velocity.x, -cfg.wall_jump_speed_x);
    }
}

#[test]
fn single_wall_cannot_be_climbed_indefinitely() {
    // Mash jump while hugging one wall for 10 seconds.
    let cfg = MovementConfig::default();
    let solids = right_wall();
    let mut s = settled_on_floor();
    let mut peak: f32 = 0.0;
    for i in 0..600 {
        s = step(&s, input(1.0, i % 6 == 0, i % 6 < 4), &cfg, &solids, DT);
        peak = peak.max(s.position.y);
    }
    // One ground jump (~61 px) plus at most one wall jump (~55 px).
    assert!(peak < 130.0, "climbed to {peak}");
}

#[test]
fn alternating_walls_allow_chimney_climb() {
    let cfg = MovementConfig::default();
    let solids = chimney();
    let mut s = settled_on_floor();
    // Jump, drift right, then wall-jump back and forth, buffering presses.
    s = step(&s, input(1.0, true, true), &cfg, &solids, DT);
    let mut wall_jumps = 0;
    let mut dir = 1.0;
    for _ in 0..300 {
        let touching = s.wall_dir != 0;
        let jump = touching && s.velocity.y <= 0.0;
        let before = s.last_wall_jump_dir;
        s = step(&s, input(dir, jump, true), &cfg, &solids, DT);
        if s.last_wall_jump_dir != before && s.last_wall_jump_dir != 0 {
            wall_jumps += 1;
            dir = -dir;
        }
    }
    assert!(wall_jumps >= 4, "only {wall_jumps} wall jumps");
    assert!(s.position.y > 150.0 || wall_jumps >= 4);
}

#[test]
fn chimney_climb_gains_height_each_cycle() {
    let cfg = MovementConfig::default();
    let solids = chimney();
    let mut s = settled_on_floor();
    s = step(&s, input(1.0, true, true), &cfg, &solids, DT);
    let mut dir = 1.0;
    let mut heights = Vec::new();
    for _ in 0..400 {
        let jump = s.wall_dir != 0 && s.velocity.y <= 0.0;
        let before = s.last_wall_jump_dir;
        s = step(&s, input(dir, jump, true), &cfg, &solids, DT);
        if s.last_wall_jump_dir != before && s.last_wall_jump_dir != 0 {
            heights.push(s.position.y);
            dir = -dir;
        }
    }
    assert!(heights.len() >= 4);
    assert!(heights.windows(2).all(|w| w[1] > w[0]), "{heights:?}");
}

#[test]
fn wall_coyote_allows_jump_shortly_after_leaving_wall() {
    let cfg = MovementConfig::default();
    let solids = right_wall();
    let mut s = run(
        airborne_on_right_wall(&solids),
        input(1.0, false, false),
        &solids,
        5,
    );
    // Let go of the wall and press jump on the first step out of contact.
    let mut steps = 0;
    while s.wall_dir != 0 {
        s = step(&s, input(-1.0, false, false), &cfg, &solids, DT);
        steps += 1;
        assert!(steps < 30);
    }
    s = step(&s, input(-1.0, true, true), &cfg, &solids, DT);
    assert!(s.velocity.y > 0.0, "wall coyote jump should fire");
    assert_eq!(s.velocity.x, -cfg.wall_jump_speed_x);
}

#[test]
fn wall_coyote_expires() {
    let cfg = MovementConfig::default();
    let solids = right_wall();
    let mut s = run(
        airborne_on_right_wall(&solids),
        input(1.0, false, false),
        &solids,
        5,
    );
    for _ in 0..10 {
        s = step(&s, input(-1.0, false, false), &cfg, &solids, DT);
    }
    s = step(&s, input(-1.0, true, true), &cfg, &solids, DT);
    assert!(s.velocity.y < 0.0, "expired wall coyote must not jump");
}

#[test]
fn ground_jump_takes_priority_next_to_wall() {
    let cfg = MovementConfig::default();
    let solids = right_wall();
    let s = run(settled_on_floor(), input(1.0, false, false), &solids, 60);
    let s = step(&s, input(1.0, true, true), &cfg, &solids, DT);
    assert_eq!(s.velocity.y, cfg.jump_velocity - cfg.gravity * DT);
    assert_eq!(s.last_wall_jump_dir, 0);
    assert_eq!(s.velocity.x, 0.0);
}

#[test]
fn landing_resets_wall_jump_side() {
    let cfg = MovementConfig::default();
    let solids = right_wall();
    let mut s = run(
        airborne_on_right_wall(&solids),
        input(1.0, false, false),
        &solids,
        5,
    );
    s = step(&s, input(1.0, true, true), &cfg, &solids, DT);
    assert_eq!(s.last_wall_jump_dir, 1);
    s = run(s, PlayerInput::default(), &solids, 200);
    assert!(s.grounded);
    s = step(&s, PlayerInput::default(), &cfg, &solids, DT);
    assert_eq!(s.last_wall_jump_dir, 0);
}

#[test]
fn wall_jump_respects_jump_buffer() {
    // Press jump just before touching the wall; it fires on contact.
    let cfg = MovementConfig::default();
    let solids = right_wall();
    let mut s = PlayerState::at(Vec2::new(30.0 - 6.0 - 1.5, 120.0));
    s.velocity.x = 130.0;
    s = step(&s, input(1.0, true, true), &cfg, &solids, DT);
    assert_eq!(s.wall_dir, 1);
    s = step(&s, input(1.0, false, true), &cfg, &solids, DT);
    assert_eq!(s.velocity.x, -cfg.wall_jump_speed_x);
    assert!(s.velocity.y > 0.0);
}

// ---------------------------------------------------------------------------
// Collision robustness
// ---------------------------------------------------------------------------

#[test]
fn very_fast_fall_does_not_tunnel_through_thin_platform() {
    let cfg = MovementConfig {
        max_fall_speed: 100_000.0,
        ..Default::default()
    };
    let thin = Rect::new(-50.0, 0.0, 100.0, 4.0);
    let mut s = PlayerState::at(Vec2::new(0.0, 40.0));
    s.velocity.y = -6000.0; // 100 px per step
    s = step(&s, PlayerInput::default(), &cfg, &[thin], DT);
    assert_eq!(s.position.y, thin.top());
    assert!(s.grounded);
}

#[test]
fn very_fast_run_does_not_tunnel_through_thin_wall() {
    let cfg = MovementConfig::default();
    let thin = Rect::new(30.0, 0.0, 4.0, 60.0);
    let mut s = settled_on_floor();
    s.velocity.x = 6000.0;
    s.wall_jump_lock_timer = 1.0; // keep velocity unchanged by input
    s = step(&s, PlayerInput::default(), &cfg, &[floor(), thin], DT);
    assert_eq!(s.hitbox(&cfg).right(), thin.left());
}

#[test]
fn landing_on_one_pixel_of_ledge_edge() {
    let cfg = MovementConfig::default();
    let ledge = Rect::new(0.0, 0.0, 48.0, 16.0);
    // Hitbox right edge overlaps the ledge by just 1 px.
    let mut s = PlayerState::at(Vec2::new(-cfg.player_width * 0.5 + 1.0, 40.0));
    s = run(s, PlayerInput::default(), &[ledge], 60);
    assert!(s.grounded);
    assert_eq!(s.position.y, ledge.top());
}

#[test]
fn falling_past_edge_does_not_snag() {
    let cfg = MovementConfig::default();
    let ledge = Rect::new(0.0, 0.0, 48.0, 16.0);
    // Exactly touching the ledge's left side: must fall freely past it.
    let mut s = PlayerState::at(Vec2::new(-cfg.player_width * 0.5, 40.0));
    s = run(s, PlayerInput::default(), &[ledge], 30);
    assert!(s.position.y < 0.0);
    assert_eq!(s.position.x, -cfg.player_width * 0.5);
}

#[test]
fn running_across_seam_between_floor_tiles_does_not_snag() {
    let cfg = MovementConfig::default();
    let a = Rect::new(-200.0, -16.0, 200.0, 16.0);
    let b = Rect::new(0.0, -16.0, 200.0, 16.0);
    let mut s = PlayerState::at(Vec2::new(-100.0, 0.0));
    for _ in 0..90 {
        s = step(&s, input(1.0, false, false), &cfg, &[a, b], DT);
        assert!(s.grounded);
        assert_eq!(s.position.y, 0.0);
    }
    assert!(s.position.x > 50.0);
    assert_eq!(s.velocity.x, cfg.max_run_speed);
}

#[test]
fn head_clipping_ceiling_corner_is_nudged_past() {
    let cfg = MovementConfig::default();
    // Ceiling whose left edge overlaps the hitbox by 3 px.
    let ceiling = Rect::new(3.0, 40.0, 100.0, 16.0);
    let mut s = PlayerState::at(Vec2::new(-6.0 + 0.0, 0.0));
    let solids = [floor(), ceiling];
    s = step(&s, PlayerInput::default(), &cfg, &solids, DT);
    s = step(&s, PRESS, &cfg, &solids, DT);
    let mut peak = s.position.y;
    for _ in 0..30 {
        s = step(&s, input(0.0, false, true), &cfg, &solids, DT);
        peak = peak.max(s.position.y);
    }
    assert!(
        peak + cfg.player_height > ceiling.bottom(),
        "was not nudged"
    );
    assert!(s.hitbox(&cfg).right() <= ceiling.left());
}

#[test]
fn large_ceiling_overlap_still_blocks() {
    let cfg = MovementConfig::default();
    let ceiling = Rect::new(-2.0, 40.0, 100.0, 16.0); // 6 px overlap
    let solids = [floor(), ceiling];
    let mut s = step(&settled_on_floor(), PRESS, &cfg, &solids, DT);
    let mut max_top = s.hitbox(&cfg).top();
    for _ in 0..30 {
        s = step(&s, input(0.0, false, true), &cfg, &solids, DT);
        max_top = max_top.max(s.hitbox(&cfg).top());
    }
    assert!(max_top <= ceiling.bottom());
}

#[test]
fn wall_jumping_is_deterministic() {
    let cfg = MovementConfig::default();
    let solids = chimney();
    let go = || {
        let mut s = settled_on_floor();
        for i in 0..900usize {
            let dir = if (i / 25) % 2 == 0 { 1.0 } else { -1.0 };
            s = step(&s, input(dir, i % 11 == 0, i % 11 < 7), &cfg, &solids, DT);
        }
        s
    };
    assert_eq!(go(), go());
}
