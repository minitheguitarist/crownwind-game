//! Optional developer switches read from environment variables. They do
//! nothing unless set, and never change the simulation rules.
//!
//! - `CROWNWIND_CHECKPOINT=<n>`: start at checkpoint number `n` (1-based).
//! - `CROWNWIND_PAUSED=1`: start paused (to capture the pause panel).
//! - `CROWNWIND_AUTOPLAY=1`: plan a route with `game_core::route` and feed
//!   its inputs to the simulation instead of the keyboard (end-to-end demo).
//! - `CROWNWIND_SCREENSHOT=<file.png>`: save a screenshot of the window and
//!   exit. `CROWNWIND_SCREENSHOT_AT=<seconds>` sets when (default 1.5 s),
//!   counted from the moment the scene is ready (art loaded and render
//!   pipelines compiled, see `readiness`), or from when the main menu's art
//!   loaded if the run starts on the menu (`CROWNWIND_MENU=1`).
//! - `CROWNWIND_SCREENSHOT_SERIES=<dir>`: startup diagnosis, a screenshot
//!   every 0.2 s of real time from launch for 3 s, then exit.
//! - `CROWNWIND_STARTUP_TRACE=1`: log render-pipeline readiness per frame.
//! - `CROWNWIND_FPS_LOG=1`: log frame-time diagnostics every second.

use std::collections::VecDeque;

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use game_core as gc;

use crate::readiness::SceneReady;
use crate::{AutoPause, GameSim};

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, apply_start_options);
        if std::env::var("CROWNWIND_FPS_LOG").is_ok_and(|v| v == "1") {
            app.add_plugins((
                bevy::diagnostic::FrameTimeDiagnosticsPlugin::default(),
                bevy::diagnostic::LogDiagnosticsPlugin::default(),
            ));
        }
        let automated = std::env::var("CROWNWIND_SCREENSHOT").is_ok()
            || std::env::var("CROWNWIND_AUTOPLAY").is_ok_and(|v| v == "1");
        if automated {
            // A background test window never has focus; do not auto-pause.
            app.insert_resource(AutoPause(false));
        }
        if let Ok(dir) = std::env::var("CROWNWIND_SCREENSHOT_SERIES") {
            app.insert_resource(Series { dir, next: 0 })
                .add_systems(Update, screenshot_series);
        }
        if let Ok(path) = std::env::var("CROWNWIND_SCREENSHOT") {
            let at = std::env::var("CROWNWIND_SCREENSHOT_AT")
                .ok()
                .and_then(|v| v.parse::<f32>().ok())
                .unwrap_or(1.5);
            app.insert_resource(ScreenshotRequest {
                path,
                at,
                elapsed: 0.0,
                taken: false,
            })
            .add_systems(Update, screenshot_and_exit);
        }
    }
}

/// Queue of inputs replacing the keyboard (see `CROWNWIND_AUTOPLAY`).
#[derive(Resource)]
pub struct Autoplay {
    pub inputs: VecDeque<gc::PlayerInput>,
    pub accumulator: f32,
}

#[derive(Resource)]
struct ScreenshotRequest {
    path: String,
    at: f32,
    elapsed: f32,
    taken: bool,
}

fn apply_start_options(mut commands: Commands, mut sim: ResMut<GameSim>) {
    if let Some(n) = std::env::var("CROWNWIND_CHECKPOINT")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        && n >= 1
    {
        sim.0.start_at_checkpoint(n - 1);
    }
    if std::env::var("CROWNWIND_PAUSED").is_ok_and(|v| v == "1") {
        sim.0.set_paused(true);
    }
    if std::env::var("CROWNWIND_AUTOPLAY").is_ok_and(|v| v == "1") {
        match gc::route::plan_level_route(&sim.0.level, &sim.0.config, 250_000) {
            Some(inputs) => {
                info!("autoplay: planned {} steps", inputs.len());
                commands.insert_resource(Autoplay {
                    inputs: inputs.into(),
                    accumulator: 0.0,
                });
            }
            None => warn!("autoplay: no route found"),
        }
    }
}

/// Feed queued inputs one per fixed step, paced by real time.
pub fn run_autoplay(time: &Time, auto: &mut Autoplay, sim: &mut gc::Simulation) {
    if sim.is_paused() {
        return;
    }
    auto.accumulator += time.delta_secs();
    let mut n = 0;
    while auto.accumulator >= gc::FIXED_DT && n < gc::simulation::MAX_STEPS_PER_ADVANCE {
        auto.accumulator -= gc::FIXED_DT;
        let input = auto.inputs.pop_front().unwrap_or_default();
        sim.fixed_step(input);
        n += 1;
    }
}

/// `CROWNWIND_SCREENSHOT_SERIES=<dir>`: from app start (real time, not
/// gated on readiness), save a screenshot every 0.2 s for 3 s, then exit.
/// Used to see exactly when the world becomes visible on cold starts.
#[derive(Resource)]
struct Series {
    dir: String,
    next: u32,
}

fn screenshot_series(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut series: ResMut<Series>,
    mut exit: MessageWriter<AppExit>,
) {
    let t = time.elapsed_secs();
    if t >= series.next as f32 * 0.2 && series.next <= 15 {
        let path = format!("{}/t{:02}.png", series.dir, series.next);
        info!("series screenshot {} at t={t:.3}s", series.next);
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
        series.next += 1;
    }
    if t > 3.6 {
        exit.write(AppExit::Success);
    }
}

fn screenshot_and_exit(
    mut commands: Commands,
    time: Res<Time>,
    mut req: ResMut<ScreenshotRequest>,
    mut exit: MessageWriter<AppExit>,
    scene: Res<SceneReady>,
    menu: Res<crate::menu::MenuReady>,
) {
    // Count time from the moment the scene (or the main menu, when the run
    // starts on it) is actually drawable.
    if !scene.is_ready() && !menu.0 {
        return;
    }
    req.elapsed += time.delta_secs();
    if !req.taken && req.elapsed >= req.at {
        req.taken = true;
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(req.path.clone()));
    }
    if req.elapsed >= req.at + 0.5 {
        exit.write(AppExit::Success);
    }
}
