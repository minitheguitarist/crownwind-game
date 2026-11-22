//! Startup readiness: play starts only once the scene can actually be seen.
//!
//! "Ready" means:
//! 1. every artwork handle has loaded (`ArtReady`), and
//! 2. the renderer has no pipelines still compiling, observed on at least
//!    [`SETTLE_FRAMES`] consecutive render frames *after* the art loaded
//!    (sprite pipelines are only requested once their images reach the GPU,
//!    so a single "nothing pending" frame right after loading is not enough).
//!
//! The render-world numbers come from Bevy's `PipelineCache` and are shared
//! with the main world through atomics. `CROWNWIND_STARTUP_TRACE=1` logs them
//! every frame for the first seconds (used to diagnose the cold-start black
//! world).

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use bevy::prelude::*;
use bevy::render::render_resource::{CachedPipelineState, PipelineCache};
use bevy::render::{Render, RenderApp};

use crate::ArtReady;

/// Consecutive "nothing compiling" render frames required after art loads.
pub const SETTLE_FRAMES: u64 = 3;

#[derive(Default)]
pub struct RenderCounters {
    pub frames: AtomicU64,
    pub total: AtomicUsize,
    pub pending: AtomicUsize,
    pub failed: AtomicUsize,
}

/// Shared between the main and render worlds.
#[derive(Resource, Clone, Default)]
pub struct RenderStatus(pub Arc<RenderCounters>);

/// Set once the scene is visible and gameplay may start.
#[derive(Resource, Default)]
pub struct SceneReady {
    pub ready: bool,
    /// Consecutive settled render frames seen after the art loaded.
    settled: u64,
    last_frame: u64,
    started: Option<f64>,
}

impl SceneReady {
    pub fn is_ready(&self) -> bool {
        self.ready
    }
}

/// Pure decision step, unit-tested: returns the new settled count.
pub fn settle_step(art_loaded: bool, new_frame: bool, pending: usize, settled: u64) -> u64 {
    if !art_loaded {
        return 0;
    }
    if !new_frame {
        return settled;
    }
    if pending == 0 { settled + 1 } else { 0 }
}

pub struct ReadinessPlugin;

impl Plugin for ReadinessPlugin {
    fn build(&self, app: &mut App) {
        let status = RenderStatus::default();
        app.insert_resource(status.clone())
            .init_resource::<SceneReady>()
            .add_systems(Update, update_scene_ready);
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .insert_resource(status)
                .add_systems(Render, sample_pipeline_cache);
        }
    }
}

fn sample_pipeline_cache(cache: Res<PipelineCache>, status: Res<RenderStatus>) {
    let mut total = 0;
    let mut pending = 0;
    let mut failed = 0;
    for p in cache.pipelines() {
        total += 1;
        match p.state {
            CachedPipelineState::Ok(_) => {}
            CachedPipelineState::Err(_) => failed += 1,
            _ => pending += 1,
        }
    }
    let c = &status.0;
    c.total.store(total, Ordering::Relaxed);
    c.pending.store(pending, Ordering::Relaxed);
    c.failed.store(failed, Ordering::Relaxed);
    c.frames.fetch_add(1, Ordering::Relaxed);
}

fn update_scene_ready(
    time: Res<Time<Real>>,
    art: Res<ArtReady>,
    status: Res<RenderStatus>,
    mut scene: ResMut<SceneReady>,
    mut trace_until: Local<Option<f64>>,
) {
    let now = time.elapsed_secs_f64();
    let start = *scene.started.get_or_insert(now);
    let trace = *trace_until.get_or_insert_with(|| {
        if std::env::var("CROWNWIND_STARTUP_TRACE").is_ok_and(|v| v == "1") {
            start + 5.0
        } else {
            -1.0
        }
    });
    let c = &status.0;
    let frames = c.frames.load(Ordering::Relaxed);
    let pending = c.pending.load(Ordering::Relaxed);
    let total = c.total.load(Ordering::Relaxed);
    let failed = c.failed.load(Ordering::Relaxed);
    if now < trace {
        info!(
            "startup trace t={:.3}s render_frames={frames} pipelines total={total} \
             pending={pending} failed={failed} art={} ready={}",
            now - start,
            art.0,
            scene.ready
        );
    }
    if scene.ready {
        return;
    }
    let new_frame = frames != scene.last_frame;
    scene.last_frame = frames;
    scene.settled = settle_step(art.0, new_frame, pending, scene.settled);
    if scene.settled >= SETTLE_FRAMES {
        scene.ready = true;
        info!(
            "scene ready after {:.2}s ({frames} render frames, {total} pipelines, {failed} failed)",
            now - start
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn needs_art_and_consecutive_settled_frames() {
        // No art: never settles.
        assert_eq!(settle_step(false, true, 0, 5), 0);
        // Art loaded, pipelines compiling: resets.
        assert_eq!(settle_step(true, true, 2, 2), 0);
        // Settled frames accumulate one per *new* render frame only.
        let mut s = 0;
        for _ in 0..3 {
            s = settle_step(true, true, 0, s);
        }
        assert_eq!(s, 3);
        assert_eq!(settle_step(true, false, 0, s), 3);
    }
}
