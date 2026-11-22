//! Bevy integration layer: Claude owns this crate.
//!
//! All gameplay rules live in `game_core`. This crate only:
//! - samples the keyboard into a `PlayerInput`,
//! - advances the fixed-step `Simulation` with the real frame time,
//! - draws the Windwood art and the prince (interpolated between steps),
//! - turns simulation events into simple effects and HUD messages,
//! - moves a smoothed follow camera,
//! - waits for the scene to be drawable before play starts (`readiness`),
//! - opens on the main menu (`menu`); the game world is spawned once, on
//!   entering `AppState::InGame`.

pub mod animation;
mod background;
mod debug;
mod effects;
mod hud;
pub mod manifest;
pub mod menu;
pub mod readiness;
pub mod review;
mod tiles;
mod view;
mod world;

use bevy::prelude::*;
use game_core as gc;

pub use menu::AppState;

/// Internal render resolution in render pixels (M2 standard).
pub const RENDER_WIDTH: u32 = 640;
pub const RENDER_HEIGHT: u32 = 360;
/// Render pixels per gameplay unit. `game_core` keeps its units (16 per
/// tile); the presentation draws each unit as 2×2 render pixels, so a
/// 32×32-texel tile covers one 16-unit cell and a 64×96 character covers
/// the 32×48-unit character cell.
pub const PIXELS_PER_UNIT: f32 = 2.0;
/// Visible world area in gameplay units (640×360 / 2).
pub const VIEW_WIDTH: f32 = RENDER_WIDTH as f32 / PIXELS_PER_UNIT;
pub const VIEW_HEIGHT: f32 = RENDER_HEIGHT as f32 / PIXELS_PER_UNIT;
/// M2 art standards in texels.
pub const CHARACTER_FRAME_STANDARD: (u32, u32) = (64, 96);
pub const TILE_TEXELS_STANDARD: u32 = 32;
/// The character cell every character art set must cover, in units.
pub const CHARACTER_CELL_UNITS: (f32, f32) = (32.0, 48.0);

/// Gameplay units covered by one texel of a character frame `frame_height`
/// texels tall: 48-texel legacy art → 1.0, 96-texel M2 art → 0.5.
pub fn character_units_per_texel(frame_height: u32) -> f32 {
    CHARACTER_CELL_UNITS.1 / frame_height as f32
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        let review = review::enabled();
        let level = if review {
            review::review_level()
        } else {
            gc::tutorial_level()
        };
        app.insert_resource(ClearColor(world::BACKGROUND))
            .insert_resource(AutoPause(true))
            .init_resource::<ArtHandles>()
            .init_resource::<ArtReady>()
            .insert_resource(animation::PrinceManifest(
                manifest::AnimationManifest::hero(),
            ))
            .insert_resource(animation::PrincessManifest(
                manifest::AnimationManifest::princess(),
            ))
            .insert_resource(GameSim(gc::Simulation::new(
                level,
                gc::MovementConfig::default(),
            )))
            .insert_state(menu::initial_state())
            .add_plugins(readiness::ReadinessPlugin)
            .add_plugins(debug::DebugPlugin)
            .add_plugins(menu::MenuPlugin)
            // The world is built exactly once, when play begins (straight
            // away when the menu is skipped: the initial state's OnEnter
            // runs before Startup).
            .add_systems(
                OnEnter(AppState::InGame),
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
                    view::fit_integer_scale,
                    view::follow_camera,
                    world::sync_player,
                    animation::animate_prince,
                    animation::animate_princess,
                    world::sync_checkpoints,
                    effects::wall_slide_dust,
                    effects::animate_effects,
                    background::follow_backdrop,
                    hud::update_guide,
                    hud::update_hud,
                )
                    .chain()
                    .run_if(in_state(AppState::InGame)),
            );
        if review {
            app.add_systems(
                OnEnter(AppState::InGame),
                review::setup_review.after(world::spawn_level),
            )
            .add_systems(PostStartup, review::apply_review_env)
            .add_systems(
                Update,
                (
                    review::review_controls,
                    review::review_camera
                        .after(view::follow_camera)
                        .before(background::follow_backdrop),
                ),
            );
        }
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
    scene: Res<readiness::SceneReady>,
) {
    // Play (and the run timer) starts only once the world is drawn.
    if !scene.is_ready() {
        return;
    }
    if let Some(mut auto) = autoplay {
        debug::run_autoplay(&time, &mut auto, &mut sim.0);
        return;
    }
    let input = read_input(&keys);
    sim.0.advance(time.delta_secs(), input);
}
