//! Main menu: Crownwind logo, three entries and the animated camp scene.
//!
//! App flow: [`AppState::MainMenu`] → (fade out) → [`AppState::InGame`].
//! The gameplay world, camera and HUD are only spawned on entering
//! `InGame` (once), and gameplay systems only run in that state, so menu
//! keys never reach the simulation and no physics runs behind the menu.
//!
//! Developer switches (environment variables):
//! - `CROWNWIND_SKIP_MENU=1`: start directly in game (also implied by
//!   review mode, autoplay, and screenshot runs unless `CROWNWIND_MENU=1`).
//! - `CROWNWIND_MENU=1`: force the menu even in screenshot runs.
//! - `CROWNWIND_MENU_AUTOSTART=<secs>`: press "Tek Oyunculu" automatically
//!   (same path as a click) after the menu has been visible that long.
//! - `CROWNWIND_MENU_FRAMES=<dir>`: save a menu screenshot every
//!   `CROWNWIND_MENU_FRAMES_STEP` s (default 0.1) for
//!   `CROWNWIND_MENU_FRAMES_SECS` s (default 6), then exit (GIF previews).

mod camp_layout;
mod font_data;
pub mod model;
pub mod pixel_font;
pub mod scene;
mod ui;

use bevy::camera::{ScalingMode, Viewport};
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::window::PrimaryWindow;

use crate::view::{integer_scale, letterbox};

/// Top-level application state.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default]
    MainMenu,
    InGame,
}

/// Initial state from the environment (see module docs).
pub fn initial_state() -> AppState {
    let on = |k: &str| std::env::var(k).is_ok_and(|v| v == "1");
    let screenshot_run = std::env::var("CROWNWIND_SCREENSHOT").is_ok()
        || std::env::var("CROWNWIND_SCREENSHOT_SERIES").is_ok();
    let skip = on("CROWNWIND_SKIP_MENU")
        || crate::review::enabled()
        || on("CROWNWIND_AUTOPLAY")
        || (screenshot_run && !on("CROWNWIND_MENU"));
    if skip {
        AppState::InGame
    } else {
        AppState::MainMenu
    }
}

/// Menu artwork handles (readiness gate for the menu screen).
#[derive(Resource, Default)]
pub struct MenuArt(pub Vec<UntypedHandle>);

/// True once every menu image has loaded (debug screenshots wait for it).
#[derive(Resource, Default)]
pub struct MenuReady(pub bool);

/// Gameplay images loaded in the background while the menu is shown, so
/// entering the game does not stall on disk I/O.
#[derive(Resource, Default)]
pub struct GamePreload(pub Vec<UntypedHandle>);

/// Seconds the screen takes to fade to black after "Tek Oyunculu".
pub const FADE_OUT: f32 = 0.35;
/// Seconds the game fades in once its scene is ready.
pub const FADE_IN: f32 = 0.4;

/// Pending menu → game transition.
#[derive(Resource, Default, Debug)]
pub struct MenuTransition {
    pub started: bool,
    pub elapsed: f32,
    /// Seconds since the menu art finished loading (drives the fade-in).
    pub shown: f32,
}

/// Menu fade-in: black until the art is on screen, then fades over this.
pub const MENU_FADE_IN: f32 = 0.5;
/// Grace period after loading for textures/pipelines to reach the GPU.
pub const MENU_SETTLE: f32 = 0.3;

/// Overlay opacity while the menu is idle (pure, tested).
pub fn intro_alpha(shown: f32) -> f32 {
    1.0 - ((shown - MENU_SETTLE) / MENU_FADE_IN).clamp(0.0, 1.0)
}

#[derive(Component)]
pub struct MenuCamera;

/// Full-screen black overlay used by both fades.
#[derive(Component)]
pub struct FadeOverlay;

/// Fade-in after entering the game from the menu.
#[derive(Component)]
pub struct GameFadeIn(f32);

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MenuArt>()
            .init_resource::<MenuReady>()
            .init_resource::<GamePreload>()
            .init_resource::<model::MenuModel>()
            .init_resource::<MenuTransition>()
            .add_systems(
                OnEnter(AppState::MainMenu),
                (
                    spawn_menu_camera,
                    scene::spawn_scene,
                    ui::spawn_menu_ui,
                    preload_game_art,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (
                    fit_menu_view,
                    check_menu_ready,
                    ui::menu_keyboard,
                    ui::refresh_buttons,
                    run_transition,
                    scene::animate_patches,
                    scene::animate_eyes,
                    scene::animate_glow,
                    scene::animate_mist,
                    scene::animate_particles,
                    menu_autostart,
                    menu_frames,
                )
                    .chain()
                    .run_if(in_state(AppState::MainMenu)),
            )
            .add_systems(OnExit(AppState::MainMenu), release_menu)
            .add_systems(OnEnter(AppState::InGame), spawn_game_fade_in)
            .add_systems(
                Update,
                (game_fade_in, log_single_world_once).run_if(in_state(AppState::InGame)),
            );
    }
}

fn spawn_menu_camera(mut commands: Commands) {
    commands.spawn((
        DespawnOnExit(AppState::MainMenu),
        Camera2d,
        MenuCamera,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::Fixed {
                width: scene::SCENE_W,
                height: scene::SCENE_H,
            },
            ..OrthographicProjection::default_2d()
        }),
        Transform::default(),
    ));
}

/// Integer-scaled, letterboxed 640×360 view (same rule as gameplay), and a
/// UI scale where one UI unit is one render pixel of that view.
fn fit_menu_view(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut camera: Query<&mut Camera, With<MenuCamera>>,
    mut ui_scale: ResMut<UiScale>,
    mut last: Local<(UVec2, f32)>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let physical = window.physical_size();
    let sf = window.scale_factor();
    if physical.x == 0 || physical.y == 0 || (physical, sf) == *last {
        return;
    }
    *last = (physical, sf);
    let n = integer_scale(physical);
    let lb = letterbox(physical, n);
    ui_scale.0 = match lb {
        Some(_) => n as f32 / sf,
        // Smaller than one 640×360 view: fill the window.
        None => (physical.x as f32 / 640.0).min(physical.y as f32 / 360.0) / sf,
    };
    if let Ok(mut c) = camera.single_mut() {
        c.viewport = lb.map(|(pos, size)| Viewport {
            physical_position: pos,
            physical_size: size,
            ..default()
        });
    }
}

fn check_menu_ready(art: Res<MenuArt>, server: Res<AssetServer>, mut ready: ResMut<MenuReady>) {
    if ready.0 || art.0.is_empty() {
        return;
    }
    if art
        .0
        .iter()
        .all(|h| server.is_loaded_with_dependencies(h.id()))
    {
        ready.0 = true;
        info!("menu artwork loaded");
    }
}

fn preload_game_art(assets: Res<AssetServer>, mut preload: ResMut<GamePreload>) {
    for path in [
        crate::tiles::TILES_PATH,
        crate::background::SKY_PATH,
        crate::background::FOREST_PATH,
        crate::manifest::HERO_ATLAS_PATH,
        crate::manifest::PRINCESS_ATLAS_PATH,
    ] {
        let h: Handle<Image> = assets.load(path);
        preload.0.push(h.untyped());
    }
}

/// Start the menu → game transition (shared by mouse, keyboard, autostart).
pub fn request_start(transition: &mut MenuTransition) {
    if !transition.started {
        transition.started = true;
        transition.elapsed = 0.0;
        info!("menu: single player selected");
    }
}

fn run_transition(
    time: Res<Time<Real>>,
    ready: Res<MenuReady>,
    mut transition: ResMut<MenuTransition>,
    mut overlay: Query<&mut BackgroundColor, With<FadeOverlay>>,
    mut next: ResMut<NextState<AppState>>,
) {
    if ready.0 {
        transition.shown += time.delta_secs();
    }
    if !transition.started {
        let a = intro_alpha(transition.shown);
        for mut bg in &mut overlay {
            if bg.0.alpha() != a {
                bg.0 = Color::srgba(0.0, 0.0, 0.0, a);
            }
        }
        return;
    }
    transition.elapsed += time.delta_secs();
    // Fade out from wherever the intro fade currently is.
    let start = intro_alpha(transition.shown - transition.elapsed);
    let a = (start + transition.elapsed / FADE_OUT).min(1.0);
    for mut bg in &mut overlay {
        bg.0 = Color::srgba(0.0, 0.0, 0.0, a);
    }
    if transition.elapsed >= FADE_OUT {
        next.set(AppState::InGame);
    }
}

fn release_menu(mut commands: Commands) {
    // Menu art is no longer referenced once the scene despawns; dropping the
    // handles lets Bevy free the textures. The preload handles are kept:
    // the game holds its own handles to the same images by then.
    commands.insert_resource(MenuArt::default());
    commands.insert_resource(MenuTransition::default());
}

fn spawn_game_fade_in(mut commands: Commands, came_from_menu: Res<MenuReady>) {
    if !came_from_menu.0 {
        return;
    }
    commands.spawn((
        GameFadeIn(0.0),
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        GlobalZIndex(100),
        BackgroundColor(Color::BLACK),
    ));
}

fn game_fade_in(
    mut commands: Commands,
    time: Res<Time<Real>>,
    scene: Res<crate::readiness::SceneReady>,
    mut q: Query<(Entity, &mut GameFadeIn, &mut BackgroundColor)>,
) {
    for (e, mut fade, mut bg) in &mut q {
        if !scene.is_ready() {
            continue;
        }
        fade.0 += time.delta_secs();
        let a = 1.0 - (fade.0 / FADE_IN).min(1.0);
        bg.0 = Color::srgba(0.0, 0.0, 0.0, a);
        if fade.0 >= FADE_IN {
            commands.entity(e).despawn();
        }
    }
}

/// Logs (once, after the world is drawable) how many gameplay cameras and
/// players exist: both must be exactly one after a menu → game transition.
fn log_single_world_once(
    scene: Res<crate::readiness::SceneReady>,
    cameras: Query<(), With<crate::view::MainCamera>>,
    menu_cameras: Query<(), With<MenuCamera>>,
    players: Query<(), With<crate::world::PlayerRoot>>,
    mut done: Local<bool>,
) {
    if *done || !scene.is_ready() {
        return;
    }
    *done = true;
    let (c, m, p) = (
        cameras.iter().count(),
        menu_cameras.iter().count(),
        players.iter().count(),
    );
    if c == 1 && m == 0 && p == 1 {
        info!("world check: 1 game camera, 0 menu cameras, 1 player");
    } else {
        error!("world check FAILED: {c} game cameras, {m} menu cameras, {p} players");
    }
}

// --- Developer automation --------------------------------------------------

fn env_f32(key: &str) -> Option<f32> {
    std::env::var(key).ok().and_then(|v| v.parse().ok())
}

fn menu_autostart(
    time: Res<Time<Real>>,
    ready: Res<MenuReady>,
    mut transition: ResMut<MenuTransition>,
    model: Res<model::MenuModel>,
    mut waited: Local<f32>,
) {
    let Some(at) = env_f32("CROWNWIND_MENU_AUTOSTART") else {
        return;
    };
    if !ready.0 {
        return;
    }
    // Counted from when the menu is visible (after its fade-in).
    *waited += time.delta_secs();
    if *waited >= at + MENU_SETTLE + MENU_FADE_IN
        && let Some(model::MenuAction::StartSinglePlayer) = model.activate(0)
    {
        request_start(&mut transition);
    }
}

fn menu_frames(
    mut commands: Commands,
    time: Res<Time<Real>>,
    ready: Res<MenuReady>,
    mut state: Local<(f32, u32)>,
    mut exit: MessageWriter<AppExit>,
) {
    let Ok(dir) = std::env::var("CROWNWIND_MENU_FRAMES") else {
        return;
    };
    if !ready.0 {
        return;
    }
    let step = env_f32("CROWNWIND_MENU_FRAMES_STEP").unwrap_or(0.1);
    let secs = env_f32("CROWNWIND_MENU_FRAMES_SECS").unwrap_or(6.0);
    // Start capturing once the intro fade has finished.
    state.0 += time.delta_secs();
    let t = state.0 - (MENU_SETTLE + MENU_FADE_IN + 0.2);
    if t >= 0.0 && t >= state.1 as f32 * step && (state.1 as f32) * step < secs {
        let path = format!("{dir}/menu_{:04}.png", state.1);
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
        state.1 += 1;
    }
    if t > secs + 0.5 {
        exit.write(AppExit::Success);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intro_fade_is_black_until_settled_then_clears() {
        assert_eq!(intro_alpha(0.0), 1.0);
        assert_eq!(intro_alpha(MENU_SETTLE), 1.0);
        assert!((intro_alpha(MENU_SETTLE + MENU_FADE_IN * 0.5) - 0.5).abs() < 1e-5);
        assert_eq!(intro_alpha(MENU_SETTLE + MENU_FADE_IN), 0.0);
        assert_eq!(intro_alpha(100.0), 0.0);
    }

    #[test]
    fn menu_is_the_default_state() {
        assert_eq!(AppState::default(), AppState::MainMenu);
    }
}
