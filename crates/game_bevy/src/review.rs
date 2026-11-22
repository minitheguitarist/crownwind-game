//! Character review sandbox (art-validation tool, not a gameplay feature).
//!
//! Start with `CROWNWIND_REVIEW=1`. The normal game systems run on a small
//! test stage instead of the tutorial, so the controllable prince uses the
//! real physics, camera and pixel snapping. Static characters stand at
//! fixed spots for side-by-side evaluation:
//!
//! - **open floor**: idle placement and floor contact, lineup of all sets;
//! - **under a 48-unit ceiling** (the tutorial's lowest walk-under
//!   clearance): head clearance;
//! - **against a wall**: face/cape overlap with the collider at the wall;
//! - **next to a thorn strip and a 1-tile step**: perceived size.
//!
//! Character sets shown:
//! - the legacy 32×48 prince atlas (frame 0, 1 unit per texel);
//! - a procedural 64×96 calibration mannequin (M2 standard density, 0.5
//!   units per texel), drawn in code, pivot (32, 90) (sole row 89);
//! - any sprites listed in `CROWNWIND_REVIEW_SPRITES`, separated by `;`,
//!   each `path:WxH:px,py` (path relative to `assets/`, frame size and pivot
//!   in texels; the first frame of the image is shown). Density follows the
//!   frame height: 48 → legacy, 96 → M2.
//!
//! **Pivot convention (sandbox and M2 art):** as written in the art docs,
//! `py` is the *ground contact line*: the top edge of the first empty row
//! below the sole (e.g. Princess V2: sole row 43 → pivot y 44). Note that
//! the legacy prince atlas does not follow its own doc (sole row is 44 while
//! the manifest says 44), so its lineup entry uses y 45 and the game's
//! manifest path compensates with +1 (see `animation::sprite_offset`).
//!
//! Keys: arrows/A/D + Space move the prince, C collider/pivot overlay,
//! G 16-unit tile grid, F camera cycle: follow / lineup / ceiling+wall.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use game_core as gc;

use crate::hud::Guide;
use crate::view::{FollowCam, MainCamera};
use crate::world::{PlayerRoot, snap};
use crate::{ArtHandles, GameSim};
use game_core::tutorial::{ground, ledge, spikes, wall};

pub fn enabled() -> bool {
    std::env::var("CROWNWIND_REVIEW").is_ok_and(|v| v == "1")
}

/// Floor surface (tile row) of the review stage.
pub const FLOOR_TY: i32 = 2;
/// Tile columns of the stage features.
pub const CEILING_TX: (i32, i32) = (22, 28);
pub const CEILING_BOTTOM_TY: i32 = FLOOR_TY + 3; // 48 units of head room
pub const WALL_TX: (i32, i32) = (36, 38);
pub const STEP_TX: (i32, i32) = (44, 47);
pub const THORNS_TX: (i32, i32) = (50, 52);

/// The review stage, built from the same tile-aligned helpers as the
/// tutorial. No checkpoints; the goal is out of reach.
pub fn review_level() -> gc::Level {
    let t = gc::TILE_SIZE;
    gc::Level {
        name: "review",
        solids: vec![
            wall(-2, 0, -6, 20),
            ground(0, 60, FLOOR_TY),
            wall(
                CEILING_TX.0,
                CEILING_TX.1,
                CEILING_BOTTOM_TY,
                CEILING_BOTTOM_TY + 2,
            ),
            wall(WALL_TX.0, WALL_TX.1, FLOOR_TY, FLOOR_TY + 6),
            ground(STEP_TX.0, STEP_TX.1, FLOOR_TY + 1),
            ledge(54, 57, FLOOR_TY + 3),
            wall(60, 62, -6, 20),
        ],
        hazards: vec![spikes(THORNS_TX.0, THORNS_TX.1, FLOOR_TY)],
        checkpoints: vec![],
        spawn: gc::Vec2::new(4.0 * t + 8.0, FLOOR_TY as f32 * t),
        kill_y: -5.0 * t,
        goal: gc::Rect::new(56.0 * t, 30.0 * t, 32.0, 48.0),
    }
}

/// One character art set to review.
#[derive(Debug, Clone, PartialEq)]
pub struct SpriteSpec {
    pub path: String,
    pub frame: (u32, u32),
    pub pivot: (f32, f32),
}

/// Parse `path:WxH:px,py;path:WxH:px,py`.
pub fn parse_specs(s: &str) -> Result<Vec<SpriteSpec>, String> {
    s.split(';')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(|item| {
            let mut parts = item.rsplitn(3, ':');
            let pivot = parts.next().ok_or("missing pivot")?;
            let frame = parts.next().ok_or("missing frame size")?;
            let path = parts.next().ok_or("missing path")?;
            let (w, h) = frame
                .split_once('x')
                .ok_or(format!("bad frame size `{frame}`"))?;
            let (px, py) = pivot
                .split_once(',')
                .ok_or(format!("bad pivot `{pivot}`"))?;
            let num = |v: &str| v.trim().parse::<f32>().map_err(|e| format!("`{v}`: {e}"));
            let frame = (num(w)? as u32, num(h)? as u32);
            if frame.0 == 0 || frame.1 == 0 {
                return Err("frame size must be positive".into());
            }
            Ok(SpriteSpec {
                path: path.to_string(),
                frame,
                pivot: (num(px)?, num(py)?),
            })
        })
        .collect()
}

/// Center offset (units) that puts the pivot's sole row on the feet point,
/// and the frame's size in units, for a frame of the given density.
pub fn placement(frame: (u32, u32), pivot: (f32, f32)) -> (Vec2, Vec2) {
    let d = crate::character_units_per_texel(frame.1);
    let (w, h) = (frame.0 as f32, frame.1 as f32);
    let offset = Vec2::new(w * 0.5 - pivot.0, pivot.1 - h * 0.5) * d;
    (offset, Vec2::new(w, h) * d)
}

// --- Procedural mannequin (64×96, pivot (32, 89)) ---------------------------

pub const MANNEQUIN_FRAME: (u32, u32) = (64, 96);
/// Contact line 90 = directly below sole row 89.
pub const MANNEQUIN_PIVOT: (f32, f32) = (32.0, 90.0);

/// RGBA pixels of a neutral calibration figure facing right: head, torso,
/// arms, legs and boots, outlined, with a tick every 8 rows on the left edge
/// (height ruler) and an eye dot showing the facing direction.
pub fn mannequin_pixels() -> Vec<u8> {
    let (w, h) = (MANNEQUIN_FRAME.0 as i32, MANNEQUIN_FRAME.1 as i32);
    let mut fill = vec![0u8; (w * h) as usize];
    let mut set = |x: i32, y: i32, v: u8| {
        if (0..w).contains(&x) && (0..h).contains(&y) {
            fill[(y * w + x) as usize] = v;
        }
    };
    for y in 0..h {
        for x in 0..w {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            // Head ellipse centred (33, 18), radii 10 × 12 (rows 6..30).
            let head = ((fx - 33.0) / 10.0).powi(2) + ((fy - 18.0) / 12.0).powi(2) <= 1.0;
            let neck = (29..37).contains(&x) && (29..34).contains(&y);
            // Torso narrows from shoulders (row 34) to waist (row 60).
            let t = ((fy - 34.0) / 26.0).clamp(0.0, 1.0);
            let half = 12.0 - 3.0 * t;
            let torso = (34..61).contains(&y) && (fx - 32.0).abs() <= half;
            let arm = (36..60).contains(&y) && ((14..19).contains(&x) || (45..50).contains(&x));
            let legs = (61..88).contains(&y) && ((22..30).contains(&x) || (34..42).contains(&x));
            let boots = (84..90).contains(&y) && ((22..33).contains(&x) || (34..46).contains(&x));
            if head || neck {
                set(x, y, 1);
            } else if torso || arm {
                set(x, y, 2);
            } else if legs || boots {
                set(x, y, 3);
            }
        }
    }
    let mut out = vec![0u8; (w * h * 4) as usize];
    let at = |fill: &Vec<u8>, x: i32, y: i32| {
        if (0..w).contains(&x) && (0..h).contains(&y) {
            fill[(y * w + x) as usize]
        } else {
            0
        }
    };
    for y in 0..h {
        for x in 0..w {
            let v = at(&fill, x, y);
            let i = ((y * w + x) * 4) as usize;
            let mut rgba = match v {
                1 => [0xE6, 0xD4, 0xC0, 255],
                2 => [0x9A, 0xA8, 0xBE, 255],
                3 => [0x6E, 0x78, 0x8E, 255],
                _ => [0, 0, 0, 0],
            };
            let edge = v != 0
                && [(1, 0), (-1, 0), (0, 1), (0, -1)]
                    .iter()
                    .any(|(dx, dy)| at(&fill, x + dx, y + dy) != v);
            if edge {
                rgba = [0x29, 0x23, 0x34, 255];
            }
            // Eye (facing right) and height ruler ticks.
            if (x, y) == (38, 16) || (x, y) == (39, 16) {
                rgba = [0x29, 0x23, 0x34, 255];
            }
            if x < 3 && y % 8 == 0 {
                rgba = [0xD8, 0xAC, 0x66, 200];
            }
            out[i..i + 4].copy_from_slice(&rgba);
        }
    }
    out
}

// --- Bevy side -----------------------------------------------------------------

#[derive(Component)]
pub struct ColliderOverlay;
#[derive(Component)]
pub struct GridOverlay;

/// Camera mode: 0 = gameplay follow, 1 = fixed on the lineup,
/// 2 = fixed on the ceiling and wall spots. F cycles.
#[derive(Resource)]
pub struct ReviewState {
    pub camera_mode: u8,
}

const COLLIDER_COLOR: Color = Color::srgba(0.30, 0.95, 1.0, 0.85);
const PIVOT_COLOR: Color = Color::srgba(1.0, 0.25, 0.75, 0.95);
const GRID_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 0.18);
/// One render pixel, in units.
const LINE: f32 = 1.0 / crate::PIXELS_PER_UNIT;

/// Outline of the collider plus a pivot cross, as children of `parent`
/// (whose origin is the feet point).
fn spawn_collider_outline(parent: &mut ChildSpawnerCommands, cfg: &gc::MovementConfig) {
    let (w, h) = (cfg.player_width, cfg.player_height);
    let lines = [
        (Vec2::new(0.0, LINE * 0.5), Vec2::new(w, LINE)),
        (Vec2::new(0.0, h - LINE * 0.5), Vec2::new(w, LINE)),
        (
            Vec2::new(-w * 0.5 + LINE * 0.5, h * 0.5),
            Vec2::new(LINE, h),
        ),
        (Vec2::new(w * 0.5 - LINE * 0.5, h * 0.5), Vec2::new(LINE, h)),
    ];
    for (pos, size) in lines {
        parent.spawn((
            ColliderOverlay,
            Sprite::from_color(COLLIDER_COLOR, size),
            Transform::from_translation(pos.extend(3.0)),
        ));
    }
    for size in [Vec2::new(5.0, LINE), Vec2::new(LINE, 5.0)] {
        parent.spawn((
            ColliderOverlay,
            Sprite::from_color(PIVOT_COLOR, size),
            Transform::from_xyz(0.0, 0.0, 3.1),
        ));
    }
}

fn spawn_label(commands: &mut Commands, at: Vec2, text: &str) {
    commands.spawn((
        Text2d::new(text),
        TextFont::from_font_size(8.0),
        TextColor(Color::srgb(0.98, 0.95, 0.88)),
        Transform::from_xyz(at.x, at.y, 6.0),
    ));
}

/// A static character with its overlay. `feet` in units; `flip` faces left.
#[allow(clippy::too_many_arguments)]
fn spawn_figure(
    commands: &mut Commands,
    image: Handle<Image>,
    frame: (u32, u32),
    pivot: (f32, f32),
    feet: Vec2,
    flip: bool,
    cfg: &gc::MovementConfig,
    label: &str,
) {
    let (offset, size) = placement(frame, pivot);
    let feet = snap(feet);
    commands
        .spawn((
            Transform::from_xyz(feet.x, feet.y, 1.5),
            Visibility::default(),
        ))
        .with_children(|p| {
            p.spawn((
                Sprite {
                    image,
                    custom_size: Some(size),
                    rect: Some(Rect::new(0.0, 0.0, frame.0 as f32, frame.1 as f32)),
                    flip_x: flip,
                    ..default()
                },
                Transform::from_xyz(if flip { -offset.x } else { offset.x }, offset.y, 0.0),
            ));
            spawn_collider_outline(p, cfg);
        });
    if !label.is_empty() {
        spawn_label(commands, Vec2::new(feet.x, feet.y + 52.0), label);
    }
}

/// (image, frame size in texels, pivot in texels, label)
type FigureSet = (Handle<Image>, (u32, u32), (f32, f32), String);

pub fn setup_review(
    mut commands: Commands,
    sim: Res<GameSim>,
    assets: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut art: ResMut<ArtHandles>,
    player: Query<Entity, With<PlayerRoot>>,
) {
    commands.insert_resource(ReviewState { camera_mode: 0 });
    let cfg = sim.0.config;
    let t = gc::TILE_SIZE;
    let floor = FLOOR_TY as f32 * t;

    // Overlay on the controllable prince.
    if let Ok(root) = player.single() {
        commands
            .entity(root)
            .with_children(|p| spawn_collider_outline(p, &cfg));
    }

    let mannequin = images.add(Image::new(
        Extent3d {
            width: MANNEQUIN_FRAME.0,
            height: MANNEQUIN_FRAME.1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        mannequin_pixels(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ));
    let prince: Handle<Image> = assets.load(crate::manifest::PRINCE_ATLAS_PATH);

    let mut sets: Vec<FigureSet> = vec![
        (
            prince,
            (32, 48),
            // Legacy atlas: sole row 44, so its contact line is 45.
            (16.0, 45.0),
            "prince 32x48 (legacy art)".into(),
        ),
        (
            mannequin,
            MANNEQUIN_FRAME,
            MANNEQUIN_PIVOT,
            "mannequin 64x96 (M2)".into(),
        ),
    ];
    match std::env::var("CROWNWIND_REVIEW_SPRITES").map(|s| parse_specs(&s)) {
        Ok(Ok(specs)) => {
            for s in specs {
                let h: Handle<Image> = assets.load(s.path.clone());
                art.0.push(h.clone().untyped());
                let name = s.path.rsplit('/').next().unwrap_or(&s.path).to_string();
                let label = format!("{name} {}x{}", s.frame.0, s.frame.1);
                sets.push((h, s.frame, s.pivot, label));
            }
        }
        Ok(Err(e)) => warn!("CROWNWIND_REVIEW_SPRITES ignored: {e}"),
        Err(_) => {}
    }

    // Open-floor lineup (from tile 8, 3 tiles apart), numbered; the names
    // are listed in the on-screen key.
    let mut key = String::from("LINEUP");
    for (i, (img, frame, pivot, name)) in sets.iter().enumerate() {
        key.push_str(&format!("\n{}  {name}", i + 1));
        let label = (i + 1).to_string();
        let x = (8.0 + 3.0 * i as f32) * t + 8.0;
        spawn_figure(
            &mut commands,
            img.clone(),
            *frame,
            *pivot,
            Vec2::new(x, floor),
            false,
            &cfg,
            &label,
        );
    }
    key.push_str("\n\ncyan: 14x38 collider   magenta: feet pivot");
    commands.spawn((
        Text::new(key),
        TextFont::from_font_size(13.0),
        TextColor(Color::srgb(0.95, 0.93, 0.88)),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(10.0),
            top: Val::Px(8.0),
            padding: UiRect::axes(Val::Px(8.0), Val::Px(6.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.09, 0.07, 0.12, 0.75)),
    ));
    // Under the 48-unit ceiling: the legacy prince and the last candidate.
    let candidate = sets.last().cloned().expect("at least the mannequin");
    for (j, set) in [sets[0].clone(), candidate.clone()].into_iter().enumerate() {
        let x = (CEILING_TX.0 as f32 + 1.5 + 3.0 * j as f32) * t;
        spawn_figure(
            &mut commands,
            set.0,
            set.1,
            set.2,
            Vec2::new(x, floor),
            false,
            &cfg,
            "",
        );
    }
    spawn_label(
        &mut commands,
        Vec2::new((CEILING_TX.0 + CEILING_TX.1) as f32 * 0.5 * t, floor + 72.0),
        "48u ceiling",
    );
    // Against the wall: collider touching the wall face, facing it.
    let half = cfg.player_width * 0.5;
    spawn_figure(
        &mut commands,
        candidate.0.clone(),
        candidate.1,
        candidate.2,
        Vec2::new(WALL_TX.0 as f32 * t - half, floor),
        false,
        &cfg,
        "",
    );
    spawn_figure(
        &mut commands,
        sets[0].0.clone(),
        sets[0].1,
        sets[0].2,
        Vec2::new(WALL_TX.1 as f32 * t + half, floor),
        true,
        &cfg,
        "",
    );

    // 16-unit tile grid over the stage (hidden until G).
    let (x0, x1, y0, y1) = (-2.0 * t, 62.0 * t, -6.0 * t, 20.0 * t);
    let mut x = x0;
    while x <= x1 {
        commands.spawn((
            GridOverlay,
            Sprite::from_color(GRID_COLOR, Vec2::new(LINE, y1 - y0)),
            Transform::from_xyz(x, (y0 + y1) * 0.5, 4.0),
            Visibility::Hidden,
        ));
        x += t;
    }
    let mut y = y0;
    while y <= y1 {
        commands.spawn((
            GridOverlay,
            Sprite::from_color(GRID_COLOR, Vec2::new(x1 - x0, LINE)),
            Transform::from_xyz((x0 + x1) * 0.5, y, 4.0),
            Visibility::Hidden,
        ));
        y += t;
    }

    // Legend.
    commands.spawn((
        Text::new("CHARACTER REVIEW   A/D Space move   C collider   G grid   F camera   R reset"),
        TextFont::from_font_size(14.0),
        TextColor(Color::srgb(0.95, 0.93, 0.88)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            bottom: Val::Px(8.0),
            padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.09, 0.07, 0.12, 0.75)),
    ));
}

/// Centers of the fixed review cameras (mode 1 lineup, mode 2 ceiling +
/// wall). Same vertical framing as gameplay: floor ~50 units above the
/// bottom edge.
pub fn fixed_camera_center(mode: u8) -> Vec2 {
    let t = gc::TILE_SIZE;
    let x = if mode == 2 { 31.0 * t } else { 17.0 * t };
    Vec2::new(x, FLOOR_TY as f32 * t + crate::VIEW_HEIGHT * 0.5 - 50.0)
}

pub fn review_controls(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<ReviewState>,
    mut guide: ResMut<Guide>,
    mut overlays: Query<&mut Visibility, (With<ColliderOverlay>, Without<GridOverlay>)>,
    mut grid: Query<&mut Visibility, (With<GridOverlay>, Without<ColliderOverlay>)>,
) {
    guide.dismissed = true;
    guide.alpha = 0.0;
    if keys.just_pressed(KeyCode::KeyC) {
        for mut v in &mut overlays {
            *v = if *v == Visibility::Hidden {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
    if keys.just_pressed(KeyCode::KeyG) {
        for mut v in &mut grid {
            *v = if *v == Visibility::Hidden {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
    if keys.just_pressed(KeyCode::KeyF) {
        state.camera_mode = (state.camera_mode + 1) % 3;
    }
}

/// Overrides the follow camera while the fixed view is selected.
pub fn review_camera(
    state: Res<ReviewState>,
    mut cam: ResMut<FollowCam>,
    mut camera: Query<&mut Transform, With<MainCamera>>,
) {
    if state.camera_mode == 0 {
        return;
    }
    let c = snap(fixed_camera_center(state.camera_mode));
    cam.center = gc::Vec2::new(c.x, c.y);
    if let Ok(mut t) = camera.single_mut() {
        t.translation.x = c.x;
        t.translation.y = c.y;
    }
}

/// Env override for automated captures: start with the grid shown and/or
/// the fixed camera (`CROWNWIND_REVIEW_GRID=1`, `CROWNWIND_REVIEW_FIXED=1|2`).
pub fn apply_review_env(
    mut state: ResMut<ReviewState>,
    mut grid: Query<&mut Visibility, With<GridOverlay>>,
) {
    if let Some(mode) = std::env::var("CROWNWIND_REVIEW_FIXED")
        .ok()
        .and_then(|v| v.parse::<u8>().ok())
    {
        state.camera_mode = mode.min(2);
    }
    if std::env::var("CROWNWIND_REVIEW_GRID").is_ok_and(|v| v == "1") {
        for mut v in &mut grid {
            *v = Visibility::Inherited;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sprite_specs() {
        let s = parse_specs("characters/a.png:64x96:32,89; characters/b.png:32x48:16,44").unwrap();
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].path, "characters/a.png");
        assert_eq!(s[0].frame, (64, 96));
        assert_eq!(s[0].pivot, (32.0, 89.0));
        assert!(parse_specs("x.png:64:32,89").is_err());
        assert!(parse_specs("x.png:0x96:32,89").is_err());
        assert!(parse_specs("").unwrap().is_empty());
    }

    #[test]
    fn m2_and_legacy_frames_share_feet_and_cell() {
        // Contact-line convention: legacy prince (sole row 44) → y 45.
        let (o1, s1) = placement((32, 48), (16.0, 45.0));
        let (o2, s2) = placement(MANNEQUIN_FRAME, MANNEQUIN_PIVOT);
        assert_eq!(s1, Vec2::new(32.0, 48.0));
        assert_eq!(s2, Vec2::new(32.0, 48.0));
        assert_eq!(o1, Vec2::new(0.0, 21.0));
        // M2: contact line 90 texels from the top = 45 units below the top.
        assert_eq!(o2, Vec2::new(0.0, 21.0));
        // Princess V2 (sole row 43, documented anchor y 44).
        let (o3, _) = placement((32, 48), (16.0, 44.0));
        assert_eq!(o3, Vec2::new(0.0, 20.0));
        let _ = s2;
    }

    #[test]
    fn mannequin_sole_is_the_pivot_row() {
        let px = mannequin_pixels();
        let (w, h) = (MANNEQUIN_FRAME.0 as usize, MANNEQUIN_FRAME.1 as usize);
        // Ignore the ruler ticks (x < 3).
        let opaque_row = |y: usize| (3..w).any(|x| px[(y * w + x) * 4 + 3] > 0);
        let bottom = (0..h).rev().find(|&y| opaque_row(y)).unwrap();
        assert_eq!(bottom as f32 + 1.0, MANNEQUIN_PIVOT.1);
        let top = (0..h).find(|&y| opaque_row(y)).unwrap();
        // Head top 84 texels (42 units) above the ground line: ~4 units over
        // the 38-unit collider, to show the hair/head allowance visually.
        assert_eq!(bottom + 1 - top, 84);
    }

    #[test]
    fn review_stage_has_the_reference_clearances() {
        let l = review_level();
        let cfg = gc::MovementConfig::default();
        let floor = FLOOR_TY as f32 * gc::TILE_SIZE;
        let ceiling = CEILING_BOTTOM_TY as f32 * gc::TILE_SIZE;
        assert_eq!(ceiling - floor, 48.0);
        assert!(ceiling - floor >= cfg.player_height + 2.0);
        let spawn_hb = gc::movement::hitbox_at(l.spawn, &cfg);
        assert!(l.solids.iter().all(|s| !s.overlaps(&spawn_hb)));
    }
}
