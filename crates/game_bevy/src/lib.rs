//! Bevy integration layer: Claude owns this crate.
//!
//! All gameplay rules live in `game_core`. This crate only:
//! - samples the keyboard into a `PlayerInput`,
//! - advances the fixed-step `Simulation` with the real frame time,
//! - draws placeholder rectangles (interpolated between fixed steps),
//! - moves a smoothed follow camera and shows a small HUD.
//!
//! All visuals are temporary colored rectangles (no external art).

use bevy::camera::ScalingMode;
use bevy::prelude::*;
use game_core as gc;

/// Virtual viewport in world pixels (480×270 design target).
pub const VIRTUAL_WIDTH: f32 = 480.0;
pub const VIRTUAL_HEIGHT: f32 = 270.0;

const BACKGROUND: Color = Color::srgb(0.11, 0.13, 0.19);
const SOLID_COLOR: Color = Color::srgb(0.36, 0.42, 0.52);
const SOLID_TOP_COLOR: Color = Color::srgb(0.55, 0.70, 0.45);
const GOAL_COLOR: Color = Color::srgba(0.95, 0.80, 0.25, 0.85);
const HERO_COLOR: Color = Color::srgb(0.86, 0.32, 0.30);
const HERO_FACE_COLOR: Color = Color::srgb(0.98, 0.88, 0.75);

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(BACKGROUND))
            .insert_resource(GameSim(gc::Simulation::new(
                gc::test_course(),
                gc::MovementConfig::default(),
            )))
            .insert_resource(FollowCam {
                rules: gc::CameraFollow {
                    view_width: VIRTUAL_WIDTH,
                    view_height: VIRTUAL_HEIGHT,
                    ..default()
                },
                center: gc::Vec2::ZERO,
                seen_resets: 0,
            })
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (simulate, sync_player, follow_camera, update_hud).chain(),
            );
    }
}

/// The deterministic game-core simulation.
#[derive(Resource)]
pub struct GameSim(pub gc::Simulation);

#[derive(Resource)]
struct FollowCam {
    rules: gc::CameraFollow,
    center: gc::Vec2,
    seen_resets: u32,
}

#[derive(Component)]
struct PlayerVisual;

#[derive(Component)]
struct PlayerFace;

#[derive(Component)]
struct MainCamera;

#[derive(Component)]
struct HudStatus;

fn camera_bounds(level: &gc::Level) -> gc::Rect {
    let b = level.bounds();
    // Let the camera look a little below the ground line.
    gc::Rect::new(b.x, b.y - 48.0, b.w, b.h + 48.0 + VIRTUAL_HEIGHT)
}

fn spawn_rect(commands: &mut Commands, rect: gc::Rect, color: Color, z: f32) {
    let c = rect.center();
    commands.spawn((
        Sprite::from_color(color, Vec2::new(rect.w, rect.h)),
        Transform::from_xyz(c.x, c.y, z),
    ));
}

fn setup(mut commands: Commands, sim: Res<GameSim>, mut cam: ResMut<FollowCam>) {
    let level = &sim.0.level;
    let cfg = &sim.0.config;

    cam.center = cam.rules.snap(sim.0.player.position, camera_bounds(level));
    cam.seen_resets = sim.0.resets;
    commands.spawn((
        Camera2d,
        MainCamera,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: VIRTUAL_HEIGHT,
            },
            ..OrthographicProjection::default_2d()
        }),
        Transform::from_xyz(cam.center.x, cam.center.y, 0.0),
    ));

    // Level geometry: body plus a thin "grass" strip on top for readability.
    for solid in &level.solids {
        spawn_rect(&mut commands, *solid, SOLID_COLOR, 0.0);
        let lip = gc::Rect::new(solid.x, solid.top() - 2.0, solid.w, 2.0);
        spawn_rect(&mut commands, lip, SOLID_TOP_COLOR, 0.1);
    }
    spawn_rect(&mut commands, level.goal, GOAL_COLOR, -0.5);

    // Placeholder hero: a rectangle with a small face marker that shows
    // which way the hero is facing. Sprites are center-anchored, so the body
    // is offset up by half its height (simulation position = feet).
    let pos = sim.0.player.position;
    commands
        .spawn((
            PlayerVisual,
            Transform::from_xyz(pos.x, pos.y, 1.0),
            Visibility::default(),
        ))
        .with_children(|p| {
            p.spawn((
                Sprite::from_color(HERO_COLOR, Vec2::new(cfg.player_width, cfg.player_height)),
                Transform::from_xyz(0.0, cfg.player_height * 0.5, 0.0),
            ));
            p.spawn((
                PlayerFace,
                Sprite::from_color(HERO_FACE_COLOR, Vec2::new(4.0, 5.0)),
                Transform::from_xyz(3.0, cfg.player_height - 6.0, 0.1),
            ));
        });

    // HUD.
    commands.spawn((
        Text::new("A/D or Left/Right: move   Space: jump (hold for higher)   R: reset"),
        TextFont::from_font_size(16.0),
        TextColor(Color::srgb(0.9, 0.9, 0.9)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            top: Val::Px(8.0),
            ..default()
        },
    ));
    commands.spawn((
        HudStatus,
        Text::new(""),
        TextFont::from_font_size(16.0),
        TextColor(Color::srgb(1.0, 0.85, 0.35)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            top: Val::Px(30.0),
            ..default()
        },
    ));
}

/// Map the keyboard state onto core input. `jump_pressed` is the edge.
fn read_input(keys: &ButtonInput<KeyCode>) -> gc::PlayerInput {
    let left = keys.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]);
    let right = keys.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]);
    let jump_keys = [KeyCode::Space, KeyCode::KeyW, KeyCode::ArrowUp];
    gc::PlayerInput {
        move_x: (right as i32 - left as i32) as f32,
        jump_pressed: keys.any_just_pressed(jump_keys),
        jump_held: keys.any_pressed(jump_keys),
    }
}

fn simulate(keys: Res<ButtonInput<KeyCode>>, time: Res<Time>, mut sim: ResMut<GameSim>) {
    if keys.just_pressed(KeyCode::KeyR) {
        sim.0.reset();
    }
    let input = read_input(&keys);
    sim.0.advance(time.delta_secs(), input);
}

fn sync_player(
    sim: Res<GameSim>,
    mut body: Query<&mut Transform, (With<PlayerVisual>, Without<PlayerFace>)>,
    mut face: Query<&mut Transform, (With<PlayerFace>, Without<PlayerVisual>)>,
) {
    let pos = sim.0.interpolated_position();
    if let Ok(mut t) = body.single_mut() {
        t.translation.x = pos.x;
        t.translation.y = pos.y;
    }
    let vx = sim.0.player.velocity.x;
    if vx.abs() > 1.0
        && let Ok(mut t) = face.single_mut()
    {
        t.translation.x = 3.0 * vx.signum();
    }
}

fn follow_camera(
    sim: Res<GameSim>,
    time: Res<Time>,
    mut cam: ResMut<FollowCam>,
    mut camera: Query<&mut Transform, With<MainCamera>>,
) {
    let bounds = camera_bounds(&sim.0.level);
    let target = sim.0.interpolated_position();
    let rules = cam.rules;
    cam.center = if cam.seen_resets != sim.0.resets {
        cam.seen_resets = sim.0.resets;
        rules.snap(target, bounds)
    } else {
        rules.update(cam.center, target, bounds, time.delta_secs())
    };
    if let Ok(mut t) = camera.single_mut() {
        t.translation.x = cam.center.x;
        t.translation.y = cam.center.y;
    }
}

fn update_hud(sim: Res<GameSim>, mut hud: Query<&mut Text, With<HudStatus>>) {
    let Ok(mut text) = hud.single_mut() else {
        return;
    };
    let status = if sim.0.reached_goal {
        format!(
            "Goal reached! (resets: {})  Press R to run again.",
            sim.0.resets
        )
    } else {
        format!("Resets: {}", sim.0.resets)
    };
    if text.0 != status {
        text.0 = status;
    }
}
