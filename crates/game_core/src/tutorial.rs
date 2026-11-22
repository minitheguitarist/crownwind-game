//! The Milestone 1 tutorial course: a procedural level made of tile-aligned
//! rectangles that teaches mechanics in order.
//!
//! Coordinates below are in 16 px tiles; `ty` values are the *top* of the
//! walkable surface. Player physics envelope (default `MovementConfig`):
//! - full jump: ~59 px (3.7 tiles) high, ~90 px (5.6 tiles) far at run speed;
//! - every gap here is at most 4 tiles and every step up at most 2 tiles,
//!   except inside the wall-jump chimneys, which are climbed by alternating
//!   wall jumps (3 tiles wide).
//!
//! Areas:
//! 1. Opening (x 0–38): flat run, a hop, a small pit, a staircase.
//! 2. Landing practice (x 38–90): pillars and floating ledges over pits,
//!    varied spacing and heights, ending with a 4-tile running jump.
//! 3. Walls (x 90–118): walk under an overhang into a chimney; climb it with
//!    wall slides / wall jumps; exit onto a high ledge.
//! 4. Hazards (x 116–170): spikes on the floor, then a spike pit crossed on
//!    floating ledges.
//! 5. Finale (x 156–212): a taller chimney, then a descending run of ledges
//!    over a long spike pit to the exit door on a plateau.

use crate::geometry::{Rect, Vec2};
use crate::level::{Checkpoint, Level, TILE_SIZE, checkpoint_at};

/// Bottom of all "ground" columns, in tiles (they extend below the view).
const BASE_TY: i32 = -6;

fn tile_rect(x0: i32, y0: i32, x1: i32, y1: i32) -> Rect {
    Rect::from_tiles(x0, y0, x1 - x0, y1 - y0, TILE_SIZE)
}

/// Solid ground column from far below up to surface `top`.
fn ground(x0: i32, x1: i32, top: i32) -> Rect {
    tile_rect(x0, BASE_TY, x1, top)
}

/// One-tile-thick floating ledge whose surface is at `top`.
fn ledge(x0: i32, x1: i32, top: i32) -> Rect {
    tile_rect(x0, top - 1, x1, top)
}

/// Wall spanning tile rows `y0..y1`.
fn wall(x0: i32, x1: i32, y0: i32, y1: i32) -> Rect {
    tile_rect(x0, y0, x1, y1)
}

/// Spike strip lying on a surface at `top` (8 px tall).
fn spikes(x0: i32, x1: i32, top: i32) -> Rect {
    Rect::new(
        x0 as f32 * TILE_SIZE,
        top as f32 * TILE_SIZE,
        (x1 - x0) as f32 * TILE_SIZE,
        8.0,
    )
}

pub fn tutorial_level() -> Level {
    let mut solids = Vec::new();
    let mut hazards = Vec::new();
    let mut checkpoints: Vec<Checkpoint> = Vec::new();

    // Left boundary.
    solids.push(wall(-2, 0, BASE_TY, 20));

    // --- Area 1: opening ---------------------------------------------------
    solids.push(ground(0, 14, 2));
    solids.push(ledge(8, 10, 3)); // a one-tile hop
    // pit 14..16 (2 tiles)
    solids.push(ground(16, 20, 2));
    solids.push(ground(20, 23, 3)); // staircase up
    solids.push(ground(23, 26, 4));
    solids.push(ground(26, 38, 2)); // and down
    checkpoints.push(checkpoint_at(33, 2));

    // --- Area 2: landing practice -----------------------------------------
    // pit 38..41
    solids.push(ground(41, 45, 2));
    solids.push(ground(48, 51, 4)); // gap 3, up 2
    solids.push(ground(54, 56, 5)); // gap 3, up 1, narrow landing
    solids.push(ground(59, 61, 3)); // gap 3, down 2
    solids.push(ground(64, 66, 5)); // gap 3, up 2
    solids.push(ground(69, 73, 2)); // gap 3, down 3
    // 4-tile running jump at the same height
    solids.push(ground(77, 98, 2));
    checkpoints.push(checkpoint_at(82, 2));

    // --- Area 3: wall slide / wall jump chimney ----------------------------
    // Overhang you walk under (head room 3 tiles) into a 3-tile shaft.
    solids.push(wall(98, 100, 5, 20));
    solids.push(ground(98, 103, 2)); // shaft floor
    // Right side of the shaft is the face of a high block (top 10).
    solids.push(ground(103, 116, 10));
    checkpoints.push(checkpoint_at(108, 10));

    // --- Area 4: hazards -----------------------------------------------------
    solids.push(ground(116, 134, 2));
    hazards.push(spikes(121, 123, 2));
    hazards.push(spikes(127, 130, 2));
    // Spike pit, one tile lower, crossed on floating ledges.
    solids.push(ground(134, 156, 1));
    hazards.push(spikes(134, 156, 1));
    solids.push(ledge(136, 138, 4));
    solids.push(ledge(141, 143, 5));
    solids.push(ledge(146, 148, 4));
    solids.push(ledge(151, 153, 5));
    solids.push(ground(156, 171, 2));
    checkpoints.push(checkpoint_at(160, 2));

    // --- Area 5: finale --------------------------------------------------------
    // A taller chimney: overhang 166..168, shaft 168..171, block top 13.
    solids.push(wall(166, 168, 5, 22));
    solids.push(ground(171, 176, 13));
    checkpoints.push(checkpoint_at(173, 13));
    // Descending ledges over a long spike pit.
    solids.push(ground(176, 194, 1));
    hazards.push(spikes(176, 194, 1));
    solids.push(ledge(179, 181, 11));
    solids.push(ledge(184, 186, 9));
    solids.push(ledge(189, 191, 7));
    // Exit plateau with the door.
    solids.push(ground(194, 210, 6));
    // Right boundary.
    solids.push(wall(210, 212, BASE_TY, 24));

    Level {
        name: "tutorial",
        solids,
        hazards,
        checkpoints,
        spawn: Vec2::new(3.0 * TILE_SIZE, 2.0 * TILE_SIZE),
        kill_y: (BASE_TY as f32 + 1.0) * TILE_SIZE,
        goal: Rect::from_tiles(204, 6, 2, 3, TILE_SIZE),
    }
}

#[cfg(test)]
mod tests;
