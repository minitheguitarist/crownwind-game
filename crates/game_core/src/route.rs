//! Automated traversal checker ("route finder") for procedural levels.
//!
//! A best-first search over coarse input decisions: every [`DECISION_STEPS`]
//! fixed steps the virtual player chooses one of six inputs
//! ({left, none, right} × {jump held, released}). A press edge is generated
//! when jump goes from released to held. States that touch a hazard or fall
//! below the kill plane are discarded.
//!
//! This is a development / test tool. It proves a course is *physically*
//! beatable with the real movement code; it says nothing about how hard the
//! required inputs are for a human. Coarse decisions (100 ms) keep it from
//! relying on single-frame tricks.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};

use crate::geometry::Rect;
use crate::level::Level;
use crate::movement::{FIXED_DT, MovementConfig, PlayerInput, PlayerState, step};

/// Fixed steps per decision (6 steps = 100 ms at 60 Hz).
pub const DECISION_STEPS: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Action {
    move_x: f32,
    jump: bool,
}

const ACTIONS: [Action; 6] = [
    Action {
        move_x: 1.0,
        jump: false,
    },
    Action {
        move_x: 1.0,
        jump: true,
    },
    Action {
        move_x: 0.0,
        jump: false,
    },
    Action {
        move_x: 0.0,
        jump: true,
    },
    Action {
        move_x: -1.0,
        jump: false,
    },
    Action {
        move_x: -1.0,
        jump: true,
    },
];

struct Node {
    state: PlayerState,
    jump_held: bool,
    parent: Option<usize>,
    action: usize,
    depth: u32,
}

#[derive(PartialEq)]
struct Open {
    priority: f32,
    index: usize,
}

impl Eq for Open {}

impl PartialOrd for Open {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Open {
    fn cmp(&self, other: &Self) -> Ordering {
        // Min-heap on priority; ties broken by insertion order (deterministic).
        other
            .priority
            .total_cmp(&self.priority)
            .then_with(|| other.index.cmp(&self.index))
    }
}

/// Result of a successful search: per-step inputs and the final state.
#[derive(Debug, Clone)]
pub struct Route {
    pub inputs: Vec<PlayerInput>,
    pub end: PlayerState,
}

fn key(s: &PlayerState, held: bool) -> (i32, i32, i32, i32, u8) {
    let flags = (s.grounded as u8)
        | ((s.wall_dir + 1) as u8) << 1
        | ((s.last_wall_jump_dir + 1) as u8) << 3
        | (held as u8) << 5;
    (
        (s.position.x / 2.0).round() as i32,
        (s.position.y / 2.0).round() as i32,
        (s.velocity.x / 20.0).round() as i32,
        (s.velocity.y / 40.0).round() as i32,
        flags,
    )
}

fn expand(
    from: &PlayerState,
    held_before: bool,
    action: Action,
    cfg: &MovementConfig,
    level: &Level,
    solids: &[Rect],
    out: &mut Vec<PlayerInput>,
) -> Option<PlayerState> {
    let mut s = *from;
    out.clear();
    for i in 0..DECISION_STEPS {
        let input = PlayerInput {
            move_x: action.move_x,
            jump_pressed: action.jump && !held_before && i == 0,
            jump_held: action.jump,
        };
        out.push(input);
        s = step(&s, input, cfg, solids, FIXED_DT);
        if level.touches_hazard(&s.hitbox(cfg)) || s.position.y < level.kill_y {
            return None;
        }
    }
    Some(s)
}

/// Search for inputs that bring the player from `start` to overlap `target`
/// without dying. `max_nodes` bounds the work.
pub fn find_route(
    level: &Level,
    cfg: &MovementConfig,
    start: PlayerState,
    target: Rect,
    max_nodes: usize,
) -> Option<Route> {
    // Only collide with geometry near the segment (speeds up the search).
    let c0 = start.position;
    let c1 = target.center();
    let margin = 400.0;
    let window = Rect::new(
        c0.x.min(c1.x) - margin,
        c0.y.min(c1.y) - margin,
        (c0.x - c1.x).abs() + 2.0 * margin,
        (c0.y - c1.y).abs() + 2.0 * margin,
    );
    let solids: Vec<Rect> = level
        .solids
        .iter()
        .filter(|r| r.overlaps(&window))
        .copied()
        .collect();

    let heuristic = |s: &PlayerState| {
        let c = s.hitbox(cfg).center();
        let dx = (c.x - c1.x).abs();
        let dy = (c.y - c1.y).abs();
        // Decisions needed at roughly run speed / climb speed.
        dx / (cfg.max_run_speed * FIXED_DT * DECISION_STEPS as f32)
            + dy / (40.0 * FIXED_DT * DECISION_STEPS as f32)
    };

    let mut nodes = vec![Node {
        state: start,
        jump_held: false,
        parent: None,
        action: 0,
        depth: 0,
    }];
    let mut open = BinaryHeap::new();
    open.push(Open {
        priority: heuristic(&start),
        index: 0,
    });
    let mut seen = HashSet::new();
    seen.insert(key(&start, false));
    let mut buf = Vec::with_capacity(DECISION_STEPS);

    while let Some(Open { index, .. }) = open.pop() {
        if nodes.len() > max_nodes {
            return None;
        }
        let (state, held, depth) = {
            let n = &nodes[index];
            (n.state, n.jump_held, n.depth)
        };
        for (ai, action) in ACTIONS.iter().enumerate() {
            let Some(next) = expand(&state, held, *action, cfg, level, &solids, &mut buf) else {
                continue;
            };
            if !seen.insert(key(&next, action.jump)) {
                continue;
            }
            nodes.push(Node {
                state: next,
                jump_held: action.jump,
                parent: Some(index),
                action: ai,
                depth: depth + 1,
            });
            let ni = nodes.len() - 1;
            if next.hitbox(cfg).overlaps(&target) {
                return Some(reconstruct(&nodes, ni, level, cfg, &solids));
            }
            open.push(Open {
                priority: (depth + 1) as f32 * 0.5 + heuristic(&next),
                index: ni,
            });
        }
    }
    None
}

/// Plan a complete run: every checkpoint in order, then the goal.
/// Returns the per-step inputs from the level spawn, or `None` if some
/// segment was not found within `max_nodes_per_segment`.
pub fn plan_level_route(
    level: &Level,
    cfg: &MovementConfig,
    max_nodes_per_segment: usize,
) -> Option<Vec<PlayerInput>> {
    let mut targets: Vec<Rect> = level.checkpoints.iter().map(|c| c.area).collect();
    targets.push(level.goal);
    let mut state = PlayerState::at(level.spawn);
    let mut inputs = Vec::new();
    for target in targets {
        let route = find_route(level, cfg, state, target, max_nodes_per_segment)?;
        inputs.extend(route.inputs);
        state = route.end;
    }
    Some(inputs)
}

fn reconstruct(
    nodes: &[Node],
    last: usize,
    level: &Level,
    cfg: &MovementConfig,
    solids: &[Rect],
) -> Route {
    let mut chain = Vec::new();
    let mut i = Some(last);
    while let Some(idx) = i {
        if nodes[idx].parent.is_some() {
            chain.push(idx);
        }
        i = nodes[idx].parent;
    }
    chain.reverse();
    let mut inputs = Vec::new();
    let mut buf = Vec::new();
    let mut held = false;
    let mut s = nodes[0].state;
    for idx in chain {
        let action = ACTIONS[nodes[idx].action];
        s = expand(&s, held, action, cfg, level, solids, &mut buf)
            .expect("replay of a found route cannot fail");
        held = action.jump;
        inputs.extend_from_slice(&buf);
    }
    Route { inputs, end: s }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Vec2;
    use crate::level::Level;

    fn flat(extra: Vec<Rect>, hazards: Vec<Rect>) -> Level {
        let mut solids = vec![
            Rect::new(-16.0, -16.0, 16.0, 600.0),
            Rect::new(0.0, -16.0, 600.0, 16.0),
            Rect::new(600.0, -16.0, 16.0, 600.0),
        ];
        solids.extend(extra);
        Level {
            name: "route_lab",
            solids,
            hazards,
            checkpoints: vec![],
            spawn: Vec2::new(40.0, 0.0),
            kill_y: -200.0,
            goal: Rect::new(560.0, 0.0, 16.0, 32.0),
        }
    }

    fn cfg() -> MovementConfig {
        MovementConfig::default()
    }

    #[test]
    fn finds_trivial_route() {
        let level = flat(vec![], vec![]);
        let r = find_route(
            &level,
            &cfg(),
            PlayerState::at(level.spawn),
            level.goal,
            50_000,
        );
        assert!(r.is_some());
    }

    #[test]
    fn refuses_target_above_jump_height() {
        // A ledge 5 tiles up with no walls nearby is out of reach.
        let ledge = Rect::new(200.0, 72.0, 64.0, 8.0);
        let level = flat(vec![ledge], vec![]);
        let target = Rect::new(220.0, 80.0, 16.0, 16.0);
        let r = find_route(&level, &cfg(), PlayerState::at(level.spawn), target, 60_000);
        assert!(r.is_none(), "route finder reached an impossible ledge");
    }

    #[test]
    fn reaches_ledge_within_jump_height() {
        let ledge = Rect::new(200.0, 40.0, 64.0, 8.0);
        let level = flat(vec![ledge], vec![]);
        let target = Rect::new(220.0, 48.0, 16.0, 16.0);
        let r = find_route(&level, &cfg(), PlayerState::at(level.spawn), target, 60_000);
        assert!(r.is_some());
    }

    #[test]
    fn refuses_to_cross_unjumpable_spikes() {
        // 12 tiles of spikes: far beyond a running jump.
        let spikes = Rect::new(200.0, 0.0, 192.0, 8.0);
        let level = flat(vec![], vec![spikes]);
        let r = find_route(
            &level,
            &cfg(),
            PlayerState::at(level.spawn),
            level.goal,
            60_000,
        );
        assert!(r.is_none());
    }

    #[test]
    fn single_wall_does_not_reach_high_ledge() {
        // High ledge reachable only by climbing a single wall: refused.
        let wall = Rect::new(300.0, 0.0, 16.0, 160.0);
        let level = flat(vec![wall], vec![]);
        let target = Rect::new(300.0, 160.0, 16.0, 16.0);
        let r = find_route(&level, &cfg(), PlayerState::at(level.spawn), target, 80_000);
        assert!(r.is_none());
    }
}
