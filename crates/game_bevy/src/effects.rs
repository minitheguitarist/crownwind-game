//! Short-lived presentation effects driven by simulation events.
//! Purely visual: nothing here feeds back into gameplay.

use bevy::prelude::*;
use game_core as gc;

use crate::GameSim;
use crate::world::snap;

const DEATH_COLOR: Color = Color::srgb(0.95, 0.30, 0.30);
const RESPAWN_COLOR: Color = Color::srgb(0.85, 0.90, 1.0);
const CHECKPOINT_COLOR: Color = Color::srgb_u8(0xD8, 0xAC, 0x66);
const GOAL_COLOR: Color = Color::srgb(1.0, 0.85, 0.30);
const DUST_COLOR: Color = Color::srgb(0.75, 0.75, 0.80);

#[derive(Component)]
pub struct Particle {
    velocity: Vec2,
    /// Exact (unsnapped) position; the transform shows it on the pixel grid.
    position: Vec2,
    life: f32,
    max_life: f32,
    gravity: f32,
}

/// Message shown in the middle of the screen for a short time.
#[derive(Resource, Default)]
pub struct Toast {
    pub text: String,
    pub remaining: f32,
}

/// Fastest completion time in this session.
#[derive(Resource, Default)]
pub struct BestTime(pub Option<f32>);

/// Full-screen tint that fades out (death flash).
#[derive(Resource, Default)]
pub struct ScreenFlash {
    pub remaining: f32,
}

pub const FLASH_DURATION: f32 = 0.35;
pub const TOAST_DURATION: f32 = 1.6;

/// Deterministic pseudo-random sequence so bursts look varied without
/// pulling in an RNG dependency.
fn hash01(seed: u32) -> f32 {
    let mut x = seed.wrapping_mul(0x9E37_79B9) ^ 0x85EB_CA6B;
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    (x & 0xFFFF) as f32 / 65535.0
}

fn burst(
    commands: &mut Commands,
    at: Vec2,
    color: Color,
    count: u32,
    speed: f32,
    seed: u32,
    up_bias: f32,
) {
    for i in 0..count {
        let a = hash01(seed.wrapping_add(i * 7)) * std::f32::consts::TAU;
        let s = speed * (0.5 + 0.5 * hash01(seed.wrapping_add(i * 13 + 1)));
        let velocity = Vec2::new(a.cos() * s, a.sin() * s + up_bias);
        // Even sizes keep particle edges on the pixel grid.
        let size = if i % 3 == 0 { 4.0 } else { 2.0 };
        let max_life = 0.45 + 0.3 * hash01(seed.wrapping_add(i * 31 + 2));
        let p = snap(at);
        commands.spawn((
            Particle {
                velocity,
                position: at,
                life: max_life,
                max_life,
                gravity: 380.0,
            },
            Sprite::from_color(color, Vec2::splat(size)),
            Transform::from_xyz(p.x, p.y, 5.0),
        ));
    }
}

pub fn process_events(
    mut commands: Commands,
    mut sim: ResMut<GameSim>,
    mut toast: ResMut<Toast>,
    mut flash: ResMut<ScreenFlash>,
    mut best: ResMut<BestTime>,
) {
    let half_h = sim.0.config.player_height * 0.5;
    let seed = sim.0.steps as u32;
    let total = sim.0.level.checkpoints.len();
    for event in sim.0.take_events() {
        match event {
            gc::GameEvent::Died { at, .. } => {
                let c = Vec2::new(at.x, at.y + half_h);
                burst(&mut commands, c, DEATH_COLOR, 18, 140.0, seed, 60.0);
                flash.remaining = FLASH_DURATION;
            }
            gc::GameEvent::Respawned { at } => {
                let c = Vec2::new(at.x, at.y + half_h);
                burst(
                    &mut commands,
                    c,
                    RESPAWN_COLOR,
                    10,
                    70.0,
                    seed ^ 0xABCD,
                    20.0,
                );
            }
            gc::GameEvent::CheckpointActivated { index } => {
                let cp = &sim.0.level.checkpoints[index];
                let c = Vec2::new(cp.spawn.x - 21.0, cp.spawn.y + 31.0);
                burst(
                    &mut commands,
                    c,
                    CHECKPOINT_COLOR,
                    14,
                    90.0,
                    seed ^ 0x1234,
                    50.0,
                );
                toast.text = format!("Checkpoint {}/{} reached", index + 1, total);
                toast.remaining = TOAST_DURATION;
            }
            gc::GameEvent::LevelCompleted => {
                let t = sim.0.elapsed;
                if best.0.is_none_or(|b| t < b) {
                    if best.0.is_some() {
                        toast.text = "New best time!".into();
                        toast.remaining = TOAST_DURATION * 1.5;
                    }
                    best.0 = Some(t);
                }
                let g = sim.0.level.goal.center();
                for k in 0..3 {
                    burst(
                        &mut commands,
                        Vec2::new(g.x, g.y),
                        GOAL_COLOR,
                        16,
                        130.0 + 30.0 * k as f32,
                        seed ^ (0x5555 + k),
                        80.0,
                    );
                }
            }
            gc::GameEvent::LevelRestarted => {
                toast.text = "Level restarted".into();
                toast.remaining = TOAST_DURATION;
            }
        }
    }
}

/// Dust puffs while sliding down a wall: visual feedback for the mechanic.
pub fn wall_slide_dust(
    mut commands: Commands,
    sim: Res<GameSim>,
    time: Res<Time>,
    mut timer: Local<f32>,
) {
    let p = &sim.0.player;
    if !p.wall_sliding || sim.0.is_paused() {
        *timer = 0.0;
        return;
    }
    *timer -= time.delta_secs();
    if *timer > 0.0 {
        return;
    }
    *timer = 0.07;
    let x = p.position.x + f32::from(p.wall_dir) * sim.0.config.player_width * 0.5;
    let at = Vec2::new(x, p.position.y + 4.0);
    let p0 = snap(at);
    commands.spawn((
        Particle {
            velocity: Vec2::new(-f32::from(p.wall_dir) * 20.0, 25.0),
            position: at,
            life: 0.3,
            max_life: 0.3,
            gravity: 0.0,
        },
        Sprite::from_color(DUST_COLOR, Vec2::splat(2.0)),
        Transform::from_xyz(p0.x, p0.y, 3.0),
    ));
}

pub fn animate_effects(
    mut commands: Commands,
    time: Res<Time>,
    sim: Res<GameSim>,
    mut toast: ResMut<Toast>,
    mut flash: ResMut<ScreenFlash>,
    mut particles: Query<(Entity, &mut Particle, &mut Transform, &mut Sprite)>,
) {
    if sim.0.is_paused() {
        return;
    }
    let dt = time.delta_secs();
    toast.remaining = (toast.remaining - dt).max(0.0);
    flash.remaining = (flash.remaining - dt).max(0.0);
    for (entity, mut p, mut t, mut sprite) in &mut particles {
        p.life -= dt;
        if p.life <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        p.velocity.y -= p.gravity * dt;
        let v = p.velocity;
        p.position += v * dt;
        let s = snap(p.position);
        t.translation.x = s.x;
        t.translation.y = s.y;
        sprite
            .color
            .set_alpha((p.life / p.max_life).clamp(0.0, 1.0));
    }
}
