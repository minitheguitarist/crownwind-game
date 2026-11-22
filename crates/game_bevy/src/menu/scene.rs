//! The animated Windwood camp behind the main menu.
//!
//! Layers (z, back to front): base plate 0, mist 0.5, animated patches 1
//! (foliage, grass, prince, princess) and fire 1.2, eyelids 2, fire glow 3,
//! sparks / smoke 4, drifting leaves 5, left readability shade 6.
//!
//! Everything is placed in 640×360 image pixels (one world unit per pixel)
//! on whole-pixel positions. All motion is driven by wall-clock time
//! (`Time<Real>` elapsed), never by frame counts.

use bevy::prelude::*;

use super::camp_layout::{
    BLINK_PRINCE, BLINK_PRINCESS, FIRE_GLOW_SIZE, MIST_SIZE, PATCHES, SHADE_WIDTH,
};
use super::model::{Blinker, loop_frame, prince_blinker, princess_blinker};
use super::{AppState, MenuArt};

pub const SCENE_W: f32 = 640.0;
pub const SCENE_H: f32 = 360.0;

/// A rectangle in 640×360 image pixels (origin top-left, y down).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl Rect {
    /// World-space center (origin at the scene center, y up).
    pub fn center(self) -> Vec2 {
        Vec2::new(
            self.x as f32 + self.w as f32 * 0.5 - SCENE_W * 0.5,
            SCENE_H * 0.5 - (self.y as f32 + self.h as f32 * 0.5),
        )
    }
}

/// An animated region of the plate (only its changing pixels are opaque).
#[derive(Debug, Clone, Copy)]
pub struct Patch {
    pub path: &'static str,
    pub rect: Rect,
    pub frames: usize,
    pub fps: f32,
    pub columns: u32,
    pub rows: u32,
}

/// Image point (x right, y down) → world point.
pub fn image_to_world(x: f32, y: f32) -> Vec2 {
    Vec2::new(x - SCENE_W * 0.5, SCENE_H * 0.5 - y)
}

#[derive(Component)]
pub struct PatchAnim {
    fps: f32,
    frames: usize,
}

#[derive(Component)]
pub struct Eyelids(pub Blinker);

#[derive(Component)]
pub struct FireGlow;

#[derive(Component)]
pub struct Mist {
    phase: f32,
    y: f32,
}

/// A recycled particle: `slot` staggers its start time.
#[derive(Component)]
pub struct Particle {
    kind: ParticleKind,
    slot: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ParticleKind {
    Spark,
    Smoke,
    Leaf,
}

/// Fire center (image pixels) used by glow and particles.
const FIRE_X: f32 = 426.0;
const FIRE_Y: f32 = 290.0;

const SPARKS: u32 = 9;
const SMOKE: u32 = 6;
const LEAVES: u32 = 4;

const Z_BASE: f32 = 0.0;
const Z_MIST: f32 = 0.5;
const Z_PATCH: f32 = 1.0;
const Z_FIRE: f32 = 1.2;
const Z_EYES: f32 = 2.0;
const Z_GLOW: f32 = 3.0;
const Z_PARTICLES: f32 = 4.0;
const Z_LEAVES: f32 = 5.0;
const Z_SHADE: f32 = 6.0;

fn scoped() -> DespawnOnExit<AppState> {
    DespawnOnExit(AppState::MainMenu)
}

pub fn spawn_scene(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut art: ResMut<MenuArt>,
) {
    let mut load = |path: &'static str| {
        let h: Handle<Image> = assets.load(path);
        art.0.push(h.clone().untyped());
        h
    };

    commands.spawn((
        scoped(),
        Sprite::from_image(load("menu/camp_base.png")),
        Transform::from_xyz(0.0, 0.0, Z_BASE),
    ));

    let mist = load("menu/mist.png");
    for (i, (phase, y)) in [(0.0, 206.0), (2.1, 238.0)].into_iter().enumerate() {
        commands.spawn((
            scoped(),
            Mist { phase, y },
            Sprite {
                color: Color::srgba(1.0, 1.0, 1.0, 0.55 - i as f32 * 0.15),
                ..Sprite::from_image(mist.clone())
            },
            Transform::from_xyz(0.0, 0.0, Z_MIST + i as f32 * 0.01),
        ));
    }

    for patch in PATCHES {
        let layout = layouts.add(TextureAtlasLayout::from_grid(
            UVec2::new(patch.rect.w, patch.rect.h),
            patch.columns,
            patch.rows,
            None,
            None,
        ));
        let c = patch.rect.center();
        let z = if patch.path.ends_with("fire.png") {
            Z_FIRE
        } else {
            Z_PATCH
        };
        commands.spawn((
            scoped(),
            PatchAnim {
                fps: patch.fps,
                frames: patch.frames,
            },
            Sprite::from_atlas_image(load(patch.path), TextureAtlas { layout, index: 0 }),
            Transform::from_xyz(c.x, c.y, z),
        ));
    }

    for (path, rect, blinker) in [
        ("menu/blink_prince.png", BLINK_PRINCE, prince_blinker()),
        (
            "menu/blink_princess.png",
            BLINK_PRINCESS,
            princess_blinker(),
        ),
    ] {
        let layout = layouts.add(TextureAtlasLayout::from_grid(
            UVec2::new(rect.w, rect.h),
            2,
            1,
            None,
            None,
        ));
        let c = rect.center();
        commands.spawn((
            scoped(),
            Eyelids(blinker),
            Sprite::from_atlas_image(load(path), TextureAtlas { layout, index: 0 }),
            Transform::from_xyz(c.x, c.y, Z_EYES),
            Visibility::Hidden,
        ));
    }

    let glow = image_to_world(FIRE_X, FIRE_Y - 6.0);
    // Even-sized texture: center on a whole pixel keeps texels aligned.
    let _ = FIRE_GLOW_SIZE;
    commands.spawn((
        scoped(),
        FireGlow,
        Sprite {
            color: Color::srgba(1.0, 1.0, 1.0, 0.2),
            ..Sprite::from_image(load("menu/fire_glow.png"))
        },
        Transform::from_xyz(glow.x.round(), glow.y.round(), Z_GLOW),
    ));

    let particles = (0..SPARKS)
        .map(|i| (ParticleKind::Spark, i))
        .chain((0..SMOKE).map(|i| (ParticleKind::Smoke, i)))
        .chain((0..LEAVES).map(|i| (ParticleKind::Leaf, i)));
    for (kind, slot) in particles {
        let z = if kind == ParticleKind::Leaf {
            Z_LEAVES
        } else {
            Z_PARTICLES
        };
        commands.spawn((
            scoped(),
            Particle { kind, slot },
            Sprite::from_color(Color::NONE, Vec2::ONE),
            Transform::from_xyz(0.0, 0.0, z),
        ));
    }

    // Left readability shade: a 1-px-tall gradient stretched to full height.
    let shade_w = SHADE_WIDTH as f32;
    commands.spawn((
        scoped(),
        Sprite {
            custom_size: Some(Vec2::new(shade_w, SCENE_H)),
            ..Sprite::from_image(load("menu/menu_shade.png"))
        },
        Transform::from_xyz(-SCENE_W * 0.5 + shade_w * 0.5, 0.0, Z_SHADE),
    ));
}

fn now(time: &Time<Real>) -> f64 {
    time.elapsed_secs_f64()
}

pub fn animate_patches(time: Res<Time<Real>>, mut q: Query<(&PatchAnim, &mut Sprite)>) {
    let t = now(&time);
    for (anim, mut sprite) in &mut q {
        let frame = loop_frame(t, anim.fps, anim.frames);
        if let Some(atlas) = sprite.texture_atlas.as_mut()
            && atlas.index != frame
        {
            atlas.index = frame;
        }
    }
}

pub fn animate_eyes(
    time: Res<Time<Real>>,
    mut q: Query<(&mut Eyelids, &mut Sprite, &mut Visibility)>,
) {
    let t = now(&time) as f32;
    for (mut lids, mut sprite, mut vis) in &mut q {
        match lids.0.frame(t) {
            Some(f) => {
                *vis = Visibility::Inherited;
                if let Some(atlas) = sprite.texture_atlas.as_mut() {
                    atlas.index = f;
                }
            }
            None => *vis = Visibility::Hidden,
        }
    }
}

/// Smooth pseudo-random flicker in roughly [-1, 1] (sum of incommensurate
/// sines: never visibly repeats).
fn flicker(t: f32) -> f32 {
    0.5 * (t * 13.1).sin() + 0.3 * (t * 7.3 + 1.0).sin() + 0.2 * (t * 23.7 + 2.0).sin()
}

pub fn animate_glow(time: Res<Time<Real>>, mut q: Query<&mut Sprite, With<FireGlow>>) {
    let t = now(&time) as f32;
    // Stepped (4 levels) so the light pulses like pixel art, not a fade.
    let level = ((flicker(t) * 0.5 + 0.5) * 4.0).floor() / 4.0;
    for mut s in &mut q {
        s.color = Color::srgba(1.0, 1.0, 1.0, 0.15 + 0.07 * level);
    }
}

pub fn animate_mist(time: Res<Time<Real>>, mut q: Query<(&Mist, &mut Transform)>) {
    let t = now(&time) as f32;
    for (mist, mut tr) in &mut q {
        // Slow back-and-forth drift over the far forest on the left.
        let x = 120.0 + 50.0 * (t * 0.045 + mist.phase).sin();
        let p = image_to_world(x, mist.y);
        tr.translation.x = p.x.round();
        // MIST_SIZE is even in both axes: a whole-pixel center keeps texels
        // on the grid.
        tr.translation.y = p.y.round();
        debug_assert!(MIST_SIZE.0.is_multiple_of(2) && MIST_SIZE.1.is_multiple_of(2));
    }
}

fn hash(n: u32) -> f32 {
    let mut x = n.wrapping_mul(0x9E37_79B9);
    x ^= x >> 16;
    x = x.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 13;
    (x & 0xFFFF) as f32 / 65535.0
}

/// Particle state at time `t`: (image position, color, size) or `None`.
fn particle_at(kind: ParticleKind, slot: u32, t: f32) -> Option<(Vec2, Color, f32)> {
    match kind {
        ParticleKind::Spark => {
            let life = 1.6;
            let period = life + 0.4;
            let offset = slot as f32 * period / SPARKS as f32;
            let cycle = ((t + offset) / period).floor() as u32;
            let age = (t + offset) - cycle as f32 * period;
            if age > life {
                return None;
            }
            let r = hash(slot * 977 + cycle);
            let r2 = hash(slot * 131 + cycle * 7 + 3);
            let x = FIRE_X - 14.0 + 28.0 * r + (age * 5.0 + r2 * 6.0).sin() * 2.0 - age * 5.0;
            let y = FIRE_Y - 22.0 - age * (24.0 + 16.0 * r2);
            let k = 1.0 - age / life;
            let color = Color::srgba(1.0, 0.55 + 0.4 * k, 0.2 + 0.3 * k * k, 0.95 * k);
            Some((Vec2::new(x, y), color, 1.0))
        }
        ParticleKind::Smoke => {
            let life = 4.5;
            let offset = slot as f32 * life / SMOKE as f32;
            let cycle = ((t + offset) / life).floor() as u32;
            let age = (t + offset) - cycle as f32 * life;
            let r = hash(slot * 313 + cycle * 11 + 1);
            let x = FIRE_X - 4.0 + 8.0 * r - age * 6.0 + (age * 1.3 + r * 5.0).sin() * 2.5;
            let y = FIRE_Y - 40.0 - age * 15.0;
            let f = age / life;
            let a = 0.16 * (f * 6.0).min(1.0) * (1.0 - f);
            let size = (2.0 + f * 4.0).round();
            Some((Vec2::new(x, y), Color::srgba(0.62, 0.62, 0.72, a), size))
        }
        ParticleKind::Leaf => {
            let life = 14.0;
            let offset = slot as f32 * life / LEAVES as f32;
            let cycle = ((t + offset) / life).floor() as u32;
            let age = (t + offset) - cycle as f32 * life;
            let r = hash(slot * 53 + cycle * 17 + 5);
            // Enters from the right of the clearing, drifts left and down.
            let x = 340.0 - age * 24.0 + (age * 1.7 + r * 6.0).sin() * 5.0;
            let y = 40.0 + 120.0 * r + age * 6.0 + (age * 2.3).sin() * 3.0;
            let a = ((age / 1.5).min(1.0) * ((life - age) / 1.5).min(1.0)).clamp(0.0, 1.0);
            let gold = Color::srgba(0.84, 0.72, 0.30, 0.9 * a);
            let green = Color::srgba(0.55, 0.66, 0.28, 0.9 * a);
            let c = if slot.is_multiple_of(2) { gold } else { green };
            Some((Vec2::new(x, y), c, 2.0))
        }
    }
}

pub fn animate_particles(
    time: Res<Time<Real>>,
    mut q: Query<(&Particle, &mut Sprite, &mut Transform)>,
) {
    let t = now(&time) as f32;
    for (p, mut sprite, mut tr) in &mut q {
        match particle_at(p.kind, p.slot, t) {
            Some((pos, color, size)) => {
                let leaf = p.kind == ParticleKind::Leaf;
                let dims = if leaf {
                    Vec2::new(size, 1.0)
                } else {
                    Vec2::splat(size)
                };
                sprite.color = color;
                sprite.custom_size = Some(dims);
                // Whole-pixel edges: offset the center by half the size.
                let w = image_to_world(pos.x.floor(), pos.y.floor());
                tr.translation.x = w.x + dims.x * 0.5;
                tr.translation.y = w.y - dims.y * 0.5;
            }
            None => sprite.color = Color::NONE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patches_fit_the_plate_and_loop_cleanly() {
        for p in PATCHES {
            assert!(
                p.rect.x + p.rect.w <= 640 && p.rect.y + p.rect.h <= 360,
                "{}",
                p.path
            );
            assert!(p.columns * p.rows >= p.frames as u32);
            // Whole seconds per loop keep all clips phase-stable.
            let secs = p.frames as f32 / p.fps;
            assert!(secs >= 1.0, "{} loop {secs}s", p.path);
        }
    }

    #[test]
    fn rect_centers_put_edges_on_whole_pixels() {
        for r in PATCHES
            .iter()
            .map(|p| p.rect)
            .chain([BLINK_PRINCE, BLINK_PRINCESS])
        {
            let c = r.center();
            let left = c.x - r.w as f32 * 0.5;
            let top = c.y + r.h as f32 * 0.5;
            assert_eq!(left.fract(), 0.0);
            assert_eq!(top.fract(), 0.0);
        }
    }

    #[test]
    fn particles_stay_in_the_scene() {
        let mut t = 0.0;
        while t < 60.0 {
            for (kind, n) in [
                (ParticleKind::Spark, SPARKS),
                (ParticleKind::Smoke, SMOKE),
                (ParticleKind::Leaf, LEAVES),
            ] {
                for slot in 0..n {
                    if let Some((p, _, _)) = particle_at(kind, slot, t) {
                        assert!((-20.0..660.0).contains(&p.x) && (-20.0..380.0).contains(&p.y));
                    }
                }
            }
            t += 0.05;
        }
    }

    #[test]
    fn menu_assets_exist_on_disk() {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/");
        let mut paths: Vec<&str> = PATCHES.iter().map(|p| p.path).collect();
        paths.extend([
            "menu/camp_base.png",
            "menu/blink_prince.png",
            "menu/blink_princess.png",
            "menu/fire_glow.png",
            "menu/menu_shade.png",
            "menu/mist.png",
            "menu/logo.png",
        ]);
        for p in paths {
            assert!(
                std::path::Path::new(&format!("{root}{p}")).is_file(),
                "missing {p}"
            );
        }
    }
}
