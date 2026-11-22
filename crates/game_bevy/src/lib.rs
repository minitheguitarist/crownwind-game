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

pub mod animation;
mod background;
mod debug;
mod effects;
mod hud;
pub mod manifest;
mod tiles;
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
            .insert_resource(AutoPause(true))
            .init_resource::<ArtHandles>()
            .init_resource::<ArtReady>()
            .insert_resource(animation::PrinceManifest(
                manifest::AnimationManifest::prince(),
            ))
            .insert_resource(GameSim(gc::Simulation::new(
                gc::tutorial_level(),
                gc::MovementConfig::default(),
            )))
            .add_plugins(debug::DebugPlugin)
            .add_systems(
                Startup,
                (
                    world::spawn_level,
                    background::spawn_backdrop,
                    view::spawn_camera,
                    hud::spawn_hud,
                ),
            )
            .add_systems(
                Update,
                (
                    check_art_ready,
                    control_keys,
                    pause_on_focus_loss,
                    simulate,
                    effects::process_events,
                    world::sync_player,
                    animation::animate_prince,
                    world::sync_checkpoints,
                    effects::wall_slide_dust,
                    effects::animate_effects,
                    view::fit_integer_scale,
                    view::follow_camera,
                    background::follow_backdrop,
                    hud::update_hud,
                )
                    .chain(),
            );
    }
}

/// Handles of the artwork the scene needs before play starts.
#[derive(Resource, Default)]
pub struct ArtHandles(pub Vec<UntypedHandle>);

/// Set once every handle in [`ArtHandles`] has loaded. Gameplay (and debug
/// screenshot timers) wait for it, so play never starts under a black screen.
#[derive(Resource, Default)]
pub struct ArtReady(pub bool);

fn check_art_ready(art: Res<ArtHandles>, server: Res<AssetServer>, mut ready: ResMut<ArtReady>) {
    if ready.0 {
        return;
    }
    if !art.0.is_empty()
        && art
            .0
            .iter()
            .all(|h| server.is_loaded_with_dependencies(h.id()))
    {
        ready.0 = true;
        info!("artwork loaded");
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

/// Whether losing window focus pauses the game (off in automated runs).
#[derive(Resource)]
pub struct AutoPause(pub bool);

/// Pause automatically when the window loses focus (alt-tab etc.).
fn pause_on_focus_loss(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    enabled: Res<AutoPause>,
    mut was_focused: Local<bool>,
    mut sim: ResMut<GameSim>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    if !enabled.0 {
        return;
    }
    if *was_focused && !window.focused && sim.0.phase == gc::Phase::Playing {
        sim.0.set_paused(true);
    }
    *was_focused = window.focused;
}

fn simulate(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut sim: ResMut<GameSim>,
    autoplay: Option<ResMut<debug::Autoplay>>,
    ready: Res<ArtReady>,
) {
    if !ready.0 {
        return;
    }
    if let Some(mut auto) = autoplay {
        debug::run_autoplay(&time, &mut auto, &mut sim.0);
        return;
    }
    let input = read_input(&keys);
    sim.0.advance(time.delta_secs(), input);
}
