//! Level and player placeholder visuals.

use bevy::prelude::*;
use game_core as gc;

use crate::GameSim;

pub const BACKGROUND: Color = Color::srgb(0.10, 0.12, 0.18);
const FAR_BACKDROP: Color = Color::srgb(0.13, 0.16, 0.24);
const SOLID_COLOR: Color = Color::srgb(0.33, 0.38, 0.48);
const SOLID_EDGE_COLOR: Color = Color::srgb(0.24, 0.28, 0.36);
const SOLID_TOP_COLOR: Color = Color::srgb(0.52, 0.70, 0.42);
const SPIKE_COLOR: Color = Color::srgb(0.90, 0.22, 0.25);
const SPIKE_BASE_COLOR: Color = Color::srgb(0.45, 0.10, 0.14);
const POLE_COLOR: Color = Color::srgb(0.75, 0.75, 0.80);
pub const FLAG_IDLE: Color = Color::srgb(0.45, 0.47, 0.55);
pub const FLAG_ACTIVE: Color = Color::srgb(0.35, 0.90, 0.45);
const GOAL_FRAME: Color = Color::srgb(0.95, 0.78, 0.25);
const GOAL_INNER: Color = Color::srgb(0.30, 0.22, 0.08);
const HERO_COLOR: Color = Color::srgb(0.86, 0.32, 0.30);
const HERO_CAPE: Color = Color::srgb(0.30, 0.40, 0.85);
const HERO_FACE_COLOR: Color = Color::srgb(0.98, 0.88, 0.75);

const Z_BACKDROP: f32 = -5.0;
const Z_GOAL: f32 = -1.0;
const Z_SOLID: f32 = 0.0;
const Z_DECOR: f32 = 0.5;
const Z_PLAYER: f32 = 2.0;

#[derive(Component)]
pub struct PlayerRoot;

#[derive(Component)]
pub struct PlayerFace;

#[derive(Component)]
pub struct PlayerCape;

#[derive(Component)]
pub struct CheckpointFlag(pub usize);

/// Round to the 1-world-pixel grid used by the 480×270 virtual viewport.
pub fn snap(v: Vec2) -> Vec2 {
    v.round()
}

fn rect_sprite(commands: &mut Commands, rect: gc::Rect, color: Color, z: f32) {
    let c = rect.center();
    commands.spawn((
        Sprite::from_color(color, Vec2::new(rect.w, rect.h)),
        Transform::from_xyz(c.x, c.y, z),
    ));
}

pub fn spawn_level(
    mut commands: Commands,
    sim: Res<GameSim>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let level = &sim.0.level;
    let cfg = &sim.0.config;
    let b = level.bounds();

    // A flat backdrop band so the playfield reads as a place.
    rect_sprite(
        &mut commands,
        gc::Rect::new(b.x, b.y, b.w, b.h + 200.0),
        FAR_BACKDROP,
        Z_BACKDROP,
    );

    for solid in &level.solids {
        rect_sprite(&mut commands, *solid, SOLID_EDGE_COLOR, Z_SOLID);
        rect_sprite(&mut commands, solid.inset(1.0), SOLID_COLOR, Z_SOLID + 0.05);
        let lip = gc::Rect::new(solid.x, solid.top() - 3.0, solid.w, 3.0);
        rect_sprite(&mut commands, lip, SOLID_TOP_COLOR, Z_SOLID + 0.1);
    }

    // Hazards: a dark base strip with a row of bright triangular spikes.
    let tooth = meshes.add(Triangle2d::new(
        Vec2::new(-4.0, 0.0),
        Vec2::new(4.0, 0.0),
        Vec2::new(0.0, 1.0),
    ));
    let spike_mat = materials.add(ColorMaterial::from_color(SPIKE_COLOR));
    for h in &level.hazards {
        rect_sprite(
            &mut commands,
            gc::Rect::new(h.x, h.y, h.w, 2.0),
            SPIKE_BASE_COLOR,
            Z_DECOR,
        );
        let teeth = (h.w / 8.0).floor().max(1.0) as i32;
        let pitch = h.w / teeth as f32;
        for i in 0..teeth {
            let x = h.x + pitch * (i as f32 + 0.5);
            commands.spawn((
                Mesh2d(tooth.clone()),
                MeshMaterial2d(spike_mat.clone()),
                Transform::from_xyz(x, h.y, Z_DECOR - 0.01).with_scale(Vec3::new(
                    pitch / 8.0,
                    h.h,
                    1.0,
                )),
            ));
        }
    }

    // Checkpoints: pole + flag (flag color shows activation).
    for (i, cp) in level.checkpoints.iter().enumerate() {
        let base = cp.area.x + cp.area.w * 0.5;
        let floor = cp.spawn.y;
        rect_sprite(
            &mut commands,
            gc::Rect::new(base - 1.0, floor, 2.0, 34.0),
            POLE_COLOR,
            Z_DECOR,
        );
        commands.spawn((
            CheckpointFlag(i),
            Sprite::from_color(FLAG_IDLE, Vec2::new(12.0, 8.0)),
            Transform::from_xyz(base + 7.0, floor + 29.0, Z_DECOR + 0.01),
        ));
    }

    // Goal: a glowing doorway.
    let g = level.goal;
    rect_sprite(&mut commands, g.inset(-2.0), GOAL_FRAME, Z_GOAL);
    rect_sprite(&mut commands, g.inset(2.0), GOAL_INNER, Z_GOAL + 0.01);

    // Placeholder hero. Sprites are center-anchored; the root sits at the
    // feet, so the body is offset up by half its height.
    let pos = snap(Vec2::new(level.spawn.x, level.spawn.y));
    commands
        .spawn((
            PlayerRoot,
            Transform::from_xyz(pos.x, pos.y, Z_PLAYER),
            Visibility::default(),
        ))
        .with_children(|p| {
            p.spawn((
                PlayerCape,
                Sprite::from_color(HERO_CAPE, Vec2::new(6.0, 14.0)),
                Transform::from_xyz(-5.0, cfg.player_height - 13.0, -0.1),
            ));
            p.spawn((
                Sprite::from_color(HERO_COLOR, Vec2::new(cfg.player_width, cfg.player_height)),
                Transform::from_xyz(0.0, cfg.player_height * 0.5, 0.0),
            ));
            p.spawn((
                PlayerFace,
                Sprite::from_color(HERO_FACE_COLOR, Vec2::new(4.0, 4.0)),
                Transform::from_xyz(3.0, cfg.player_height - 6.0, 0.1),
            ));
        });
}

type FaceQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut Transform,
    (With<PlayerFace>, Without<PlayerRoot>, Without<PlayerCape>),
>;
type CapeQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut Transform,
    (With<PlayerCape>, Without<PlayerRoot>, Without<PlayerFace>),
>;

pub fn sync_player(
    sim: Res<GameSim>,
    mut root: Query<(&mut Transform, &mut Visibility), With<PlayerRoot>>,
    mut face: FaceQuery,
    mut cape: CapeQuery,
) {
    let p = &sim.0.player;
    let pos = sim.0.interpolated_position();
    let pos = snap(Vec2::new(pos.x, pos.y));
    if let Ok((mut t, mut vis)) = root.single_mut() {
        t.translation.x = pos.x;
        t.translation.y = pos.y;
        *vis = if sim.0.death_progress().is_some() {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
    let facing = f32::from(p.facing);
    if let Ok(mut t) = face.single_mut() {
        t.translation.x = 3.0 * facing;
    }
    if let Ok(mut t) = cape.single_mut() {
        // Cape trails behind and lifts with speed: quantized to whole pixels.
        let lift = (-p.velocity.y / 120.0).clamp(-2.0, 3.0).round();
        let sway = (p.velocity.x.abs() / 65.0).clamp(0.0, 2.0).round();
        t.translation.x = -facing * (5.0 + sway);
        t.translation.y = sim.0.config.player_height - 13.0 + lift;
    }
}

pub fn sync_checkpoints(sim: Res<GameSim>, mut flags: Query<(&CheckpointFlag, &mut Sprite)>) {
    let active = sim.0.active_checkpoint;
    for (flag, mut sprite) in &mut flags {
        let reached = active.is_some_and(|a| flag.0 <= a);
        let want = if reached { FLAG_ACTIVE } else { FLAG_IDLE };
        if sprite.color != want {
            sprite.color = want;
        }
    }
}
