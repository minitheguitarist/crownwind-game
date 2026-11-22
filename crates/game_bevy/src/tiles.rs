//! Windwood tile mapping: turns the level's collision rectangles into tile
//! placements. Pure logic (no Bevy types) so it can be tested.
//!
//! Visuals follow collision exactly: every solid cell gets one opaque-body
//! terrain tile and no terrain tile is placed outside a solid. Decorations
//! (ferns, flowers, roots) are purely cosmetic and never sit on hazards,
//! checkpoints, the exit or the spawn.

use std::collections::HashSet;

use game_core as gc;

/// Atlas `assets/environment/windwood/tiles.png`: 11 cells of 16×16.
pub const TILES_PATH: &str = "environment/windwood/tiles.png";
pub const TILE_COLUMNS: u32 = 11;
pub const TILE: f32 = 16.0;

pub const GRASS: usize = 0;
pub const EARTH_A: usize = 1;
pub const EARTH_B: usize = 2;
pub const STONE: usize = 3;
pub const MOSS_STONE: usize = 4;
pub const EDGE_LEFT: usize = 5;
pub const EDGE_RIGHT: usize = 6;
pub const FERN: usize = 7;
pub const ROOT: usize = 8;
pub const FLOWERS: usize = 9;
pub const THORNS: usize = 10;

/// Transparent rows at the top of the thorn cell (measured in the atlas).
#[cfg(test)]
pub const THORN_EMPTY_TOP_ROWS: f32 = 4.0;
/// The thorn cell is drawn this many pixels below the hazard's surface so its
/// dark root band overlaps the ground and the visible thorns above the
/// surface are exactly as tall as the 8 px hazard rectangle.
pub const THORN_SINK: f32 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    Terrain,
    Decor,
    Hazard,
}

/// One tile sprite; `x`, `y` are the bottom-left corner in world pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub x: f32,
    pub y: f32,
    pub index: usize,
    pub layer: Layer,
}

/// Deterministic per-cell hash in `0..1`.
fn hash01(x: i32, y: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x8DA6_B343) ^ (y as u32).wrapping_mul(0xD816_3841);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5BD1_E995);
    h ^= h >> 15;
    (h & 0xFFFF) as f32 / 65535.0
}

fn tile_range(lo: f32, hi: f32) -> std::ops::Range<i32> {
    (lo / TILE).round() as i32..(hi / TILE).round() as i32
}

/// Cells covered by solids.
pub fn occupancy(level: &gc::Level) -> HashSet<(i32, i32)> {
    let mut occ = HashSet::new();
    for s in &level.solids {
        for tx in tile_range(s.left(), s.right()) {
            for ty in tile_range(s.bottom(), s.top()) {
                occ.insert((tx, ty));
            }
        }
    }
    occ
}

fn cell_rect(tx: i32, ty: i32) -> gc::Rect {
    gc::Rect::new(tx as f32 * TILE, ty as f32 * TILE, TILE, TILE)
}

/// Terrain tiles for every solid cell.
pub fn terrain(level: &gc::Level) -> Vec<Placement> {
    let occ = occupancy(level);
    let filled = |x: i32, y: i32| occ.contains(&(x, y));
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for s in &level.solids {
        let xs = tile_range(s.left(), s.right());
        let ys = tile_range(s.bottom(), s.top());
        let wall = xs.len() <= 2 && ys.len() >= 4;
        let top = ys.end - 1;
        for tx in xs.clone() {
            for ty in ys.clone() {
                if !seen.insert((tx, ty)) {
                    continue; // overlapping solids: one tile per cell
                }
                let exposed_top = !filled(tx, ty + 1);
                let index = if wall {
                    if exposed_top { MOSS_STONE } else { STONE }
                } else if exposed_top {
                    let open_l = !filled(tx - 1, ty);
                    let open_r = !filled(tx + 1, ty);
                    match (open_l, open_r) {
                        (true, false) => EDGE_LEFT,
                        (false, true) => EDGE_RIGHT,
                        _ => GRASS,
                    }
                } else if top - ty <= 3 {
                    if hash01(tx, ty) < 0.5 {
                        EARTH_A
                    } else {
                        EARTH_B
                    }
                } else {
                    STONE
                };
                out.push(Placement {
                    x: tx as f32 * TILE,
                    y: ty as f32 * TILE,
                    index,
                    layer: Layer::Terrain,
                });
            }
        }
    }
    out
}

/// Thorn tiles covering each hazard, aligned to its collision rectangle.
pub fn thorns(level: &gc::Level) -> Vec<Placement> {
    let mut out = Vec::new();
    for h in &level.hazards {
        for tx in tile_range(h.left(), h.right()) {
            out.push(Placement {
                x: tx as f32 * TILE,
                y: h.bottom() - THORN_SINK,
                index: THORNS,
                layer: Layer::Hazard,
            });
        }
    }
    out
}

/// Cosmetic plants and roots. Never on gameplay-relevant spots.
pub fn decorations(level: &gc::Level) -> Vec<Placement> {
    let occ = occupancy(level);
    // Roots only make sense in soil, never in stone.
    let earth: HashSet<(i32, i32)> = terrain(level)
        .into_iter()
        .filter(|p| matches!(p.index, EARTH_A | EARTH_B))
        .map(|p| ((p.x / TILE) as i32, (p.y / TILE) as i32))
        .collect();
    let keep_clear: Vec<gc::Rect> = level
        .hazards
        .iter()
        .copied()
        .chain(level.checkpoints.iter().map(|c| c.area))
        .chain(std::iter::once(level.goal))
        .chain(std::iter::once(gc::Rect::new(
            level.spawn.x - 16.0,
            level.spawn.y,
            32.0,
            32.0,
        )))
        .map(|r| r.inset(-TILE)) // one tile of margin
        .collect();
    let mut out = Vec::new();
    let mut cells: Vec<_> = occ.iter().copied().collect();
    cells.sort_unstable();
    for (tx, ty) in cells {
        let above = (tx, ty + 1);
        let r = hash01(tx * 3 + 7, ty * 5 + 1);
        if !occ.contains(&above) {
            // Plant on an exposed surface.
            let index = if r < 0.10 {
                FERN
            } else if r < 0.17 {
                FLOWERS
            } else {
                continue;
            };
            let cell = cell_rect(above.0, above.1);
            if keep_clear.iter().any(|k| k.overlaps(&cell)) {
                continue;
            }
            out.push(Placement {
                x: cell.x,
                y: cell.y,
                index,
                layer: Layer::Decor,
            });
        } else if earth.contains(&(tx, ty))
            && occ.contains(&(tx, ty + 2))
            && !occ.contains(&(tx, ty + 3))
            && r < 0.12
        {
            // Exposed root in the soil just below a grass surface.
            let cell = cell_rect(tx, ty);
            if keep_clear.iter().any(|k| k.overlaps(&cell)) {
                continue;
            }
            out.push(Placement {
                x: cell.x,
                y: cell.y,
                index: ROOT,
                layer: Layer::Decor,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level() -> gc::Level {
        gc::tutorial_level()
    }

    #[test]
    fn tutorial_geometry_is_tile_aligned() {
        let l = level();
        for r in l.solids.iter().chain(&l.hazards) {
            for v in [r.x, r.y, r.w] {
                assert_eq!(v % TILE, 0.0, "{r:?} not on the tile grid");
            }
        }
        for s in &l.solids {
            assert_eq!(s.h % TILE, 0.0);
        }
    }

    #[test]
    fn terrain_tiles_match_collision_exactly() {
        let l = level();
        let occ = occupancy(&l);
        let tiles = terrain(&l);
        assert_eq!(tiles.len(), occ.len(), "one tile per solid cell");
        for t in &tiles {
            let cell = gc::Rect::new(t.x, t.y, TILE, TILE);
            let center = gc::Rect::new(t.x + 7.0, t.y + 7.0, 2.0, 2.0);
            assert!(
                l.solids.iter().any(|s| s.overlaps(&center)),
                "tile at {cell:?} is outside every solid"
            );
            assert!(t.index < TILE_COLUMNS as usize);
        }
    }

    #[test]
    fn surfaces_get_grass_and_interiors_do_not() {
        let l = level();
        let occ = occupancy(&l);
        for t in terrain(&l) {
            let tx = (t.x / TILE) as i32;
            let ty = (t.y / TILE) as i32;
            let exposed = !occ.contains(&(tx, ty + 1));
            let surface = matches!(t.index, GRASS | EDGE_LEFT | EDGE_RIGHT | MOSS_STONE);
            assert_eq!(exposed, surface, "cell ({tx},{ty}) index {}", t.index);
        }
    }

    #[test]
    fn edges_face_open_air() {
        let l = level();
        let occ = occupancy(&l);
        for t in terrain(&l) {
            let tx = (t.x / TILE) as i32;
            let ty = (t.y / TILE) as i32;
            if t.index == EDGE_LEFT {
                assert!(!occ.contains(&(tx - 1, ty)) && occ.contains(&(tx + 1, ty)));
            }
            if t.index == EDGE_RIGHT {
                assert!(!occ.contains(&(tx + 1, ty)) && occ.contains(&(tx - 1, ty)));
            }
        }
    }

    #[test]
    fn thorns_cover_hazards_and_match_their_height() {
        let l = level();
        let th = thorns(&l);
        for h in &l.hazards {
            let covering: Vec<_> = th
                .iter()
                .filter(|p| {
                    p.x >= h.left() && p.x + TILE <= h.right() && p.y == h.bottom() - THORN_SINK
                })
                .collect();
            assert_eq!(
                covering.len() as f32 * TILE,
                h.w,
                "hazard {h:?} not fully drawn"
            );
            // Visible thorn height above the surface equals the hazard height.
            let visible = TILE - THORN_EMPTY_TOP_ROWS - THORN_SINK;
            assert_eq!(visible, h.h);
        }
        // No thorn sprite where there is no hazard.
        for p in &th {
            let probe = gc::Rect::new(p.x + 7.0, p.y + THORN_SINK + 1.0, 2.0, 2.0);
            assert!(l.hazards.iter().any(|h| h.overlaps(&probe)));
        }
    }

    #[test]
    fn decorations_stay_off_gameplay_spots_and_solids() {
        let l = level();
        let occ = occupancy(&l);
        let decor = decorations(&l);
        assert!(
            decor.len() > 10,
            "expected some foliage, got {}",
            decor.len()
        );
        for d in &decor {
            let cell = gc::Rect::new(d.x, d.y, TILE, TILE);
            for h in &l.hazards {
                assert!(!h.overlaps(&cell));
            }
            assert!(!l.goal.overlaps(&cell));
            for c in &l.checkpoints {
                assert!(!c.area.overlaps(&cell));
            }
            let tx = (d.x / TILE) as i32;
            let ty = (d.y / TILE) as i32;
            match d.index {
                FERN | FLOWERS => {
                    assert!(!occ.contains(&(tx, ty)), "plant inside a solid");
                    assert!(occ.contains(&(tx, ty - 1)), "plant floating");
                }
                ROOT => {
                    let t = terrain(&l)
                        .into_iter()
                        .find(|p| p.x == d.x && p.y == d.y)
                        .expect("root outside terrain");
                    assert!(matches!(t.index, EARTH_A | EARTH_B), "root on non-soil");
                }
                other => panic!("unexpected decor tile {other}"),
            }
        }
    }

    #[test]
    fn mapping_is_deterministic() {
        let l = level();
        assert_eq!(terrain(&l), terrain(&l));
        assert_eq!(decorations(&l), decorations(&l));
    }

    #[test]
    fn tiles_png_matches_documented_grid() {
        let png = include_bytes!("../../../assets/environment/windwood/tiles.png");
        let w = u32::from_be_bytes(png[16..20].try_into().unwrap());
        let h = u32::from_be_bytes(png[20..24].try_into().unwrap());
        assert_eq!((w, h), (TILE_COLUMNS * 16, 16));
        for name in ["sky", "forest"] {
            let png: &[u8] = match name {
                "sky" => include_bytes!("../../../assets/environment/windwood/sky.png"),
                _ => include_bytes!("../../../assets/environment/windwood/forest.png"),
            };
            let w = u32::from_be_bytes(png[16..20].try_into().unwrap());
            let h = u32::from_be_bytes(png[20..24].try_into().unwrap());
            assert_eq!((w, h), (480, 270), "{name} plate size");
        }
    }
}
