//! Bevy integration layer: Claude owns this crate.
//!
//! All gameplay rules live in `game_core`. This crate only:
//! - samples the keyboard into a `PlayerInput`,
//! - advances the fixed-step `Simulation` with the real frame time,
//! - draws placeholder shapes (interpolated between fixed steps),
//! - turns simulation events into simple effects and HUD messages,
//! - moves a smoothed follow camera.
//!
//! All visuals are temporary geometric placeholders (no external art).

mod effects;
mod hud;
mod view;
mod world;

use bevy::prelude::*;
use game_core as gc;

/// Virtual viewport in world pixels (480×270 design target).
pub const VIRTUAL_WIDTH: f32 = 480.0;
pub const VIRTUAL_HEIGHT: f32 = 270.0;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(world::BACKGROUND))
            .insert_resource(GameSim(gc::Simulation::new(
                gc::test_course(),
                gc::MovementConfig::default(),
            )))
            .add_systems(
                Startup,
                (world::spawn_level, view::spawn_camera, hud::spawn_hud),
            )
            .add_systems(
                Update,
                (
                    control_keys,
                    simulate,
                    effects::process_events,
                    world::sync_player,
                    world::sync_checkpoints,
                    effects::wall_slide_dust,
                    effects::animate_effects,
                    view::fit_integer_scale,
                    view::follow_camera,
                    hud::update_hud,
                )
                    .chain(),
            );
    }
}

/// The deterministic game-core simulation.
#[derive(Resource)]
pub struct GameSim(pub gc::Simulation);

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

/// Esc toggles pause; R restarts the level (and un-pauses).
fn control_keys(keys: Res<ButtonInput<KeyCode>>, mut sim: ResMut<GameSim>) {
    if keys.just_pressed(KeyCode::KeyR) {
        sim.0.set_paused(false);
        sim.0.restart_level();
    } else if keys.just_pressed(KeyCode::Escape) {
        let paused = sim.0.is_paused();
        sim.0.set_paused(!paused);
    }
}

fn simulate(keys: Res<ButtonInput<KeyCode>>, time: Res<Time>, mut sim: ResMut<GameSim>) {
    let input = read_input(&keys);
    sim.0.advance(time.delta_secs(), input);
}
