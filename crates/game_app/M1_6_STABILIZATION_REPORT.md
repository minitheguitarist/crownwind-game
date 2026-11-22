# M1.6 Stabilization Report

Worktree `/home/smurat/Projects/crownwind-integration`, branch `integration/m1-polish`. The branch and a clean working tree were checked at the start, on top of `294da06`.

Only `crates/game_core`, `crates/game_bevy` and `crates/game_app` were changed. Assets, content, schemas, root config and `Cargo.lock` were not touched, and no dependencies were added. Nothing was pushed or merged.

## Commits
| SHA | Content |
| --- | --- |
| `561f032` | Phase 1: the collider now matches the prince art (14×38) + regression tests |
| `52fda2c` | Phases 2–4: startup readiness gate, steadier pixel rendering, forest seam, HUD cleanup |
| (next commit) | this report |

## Bugs fixed

### 1. Head going into ceilings and ledges (Phase 1)
**Analysis** (pixel measurements of `prince_debug.png`):
- With pivot (16,44), the head top (hair) is 40 px above the ground line in idle/run, and 42 px in the jump frame.
- The legs and torso span pivot −6..+7 px, i.e. 14 px. The face reaches +9 px; the cape at the back reaches −14 px.
- The old collider was 12×24, so the head poked 16+ px into ceilings.

**Solution:** the default `MovementConfig` collider is now **14×38**.
- Width matches the legs and torso.
- Height matches the head; 2 px of hair is allowed to pass ceilings, the usual forgiving choice.
- The face can visually touch a wall by up to 2 px and the cape by more; these are cosmetic only.

**Consequence check:**
- Every walk-under overhang in the level leaves 48 px of head room. That is ≥ 38 + 2, so a test now enforces it.
- The 3-tile chimneys can still be climbed with wall jumps; the route solver passes.
- Checkpoint areas are 48 px tall, the exit is 48 px tall, and spikes are at foot level, so none of them are affected.
- **No level geometry needed to change.**

**Movement rules unchanged:** acceleration, gravity, jump, wall jump, coyote time, and so on all keep their values. Only the collider size changed.

**Broken assumptions found:** two old tests had the old 6 px half-width hard-coded. The corner-correction test never actually overlapped the ceiling, so it wasn't testing what it claimed. All of these were fixed.

### 2. Black world at startup (Phase 2): reproduced, diagnosed and fixed
**Method:**
- `CROWNWIND_STARTUP_TRACE=1` logs Bevy's `PipelineCache` state every frame (total / pending / failed), read from the render world and passed to the main world with atomics.
- `CROWNWIND_SCREENSHOT_SERIES=<dir>` saves a screenshot every 0.2 s for the first 3 s after launch.

**Evidence** (a freshly built binary, then the same binary run again):

| Run | First frame | Pipelines | World appears | Pending reaches 0 |
| --- | --- | --- | --- | --- |
| Cold | 0.85 s | 47 (40 pending at first) | 1.6–1.7 s | ~1.93 s |
| Warm | 0.75 s | 47 | ~1.8 s | ~1.79 s |

**Conclusion:** the black world happens on **every** launch, not just cold ones. Render pipelines are compiled in the background after the window opens. The UI text pipeline finishes early, while sprites are not drawn until their pipeline is ready.

My earlier "only on the first launch after a build" note was wrong. Screenshots taken 1.5 s after the art loaded sometimes landed inside this window and sometimes didn't.

**Not proven:** whether this is the GPU driver's shader compilation or wgpu/Bevy's own pipeline creation. The measurement only shows Bevy's `PipelineCache` state.

**Fix** (no arbitrary waits; the condition is measured):
- **Scene ready** = all 4 images have loaded **and** at least 3 render frames in a row with no pipeline pending.
- Until then a small "Loading Windwood..." notice is shown, and the simulation, the run timer and the debug screenshot timers do not start.
- Verified on a later run: during the black period only the loading text is visible (`loading_1.2s.png`). The world appears and the timer starts at 0 at ~2.0 s.
- In every run, "ready" fired 0–0.2 s after the world became visible, never before.

### 3. Pixel jitter / movement quality (Phase 3)
- **1 px wobble:** the camera eases smoothly but is drawn at `round(camera)`. The prince was drawn independently at `round(x)`, so while the camera eased he could wobble by 1 px on screen.
  - Now the prince's x is rounded as an offset from the drawn camera.
  - Vertically, plain `round(y)` is kept so the feet always sit on the tile line.
  - A test shows the old method produced the wobble and the new one doesn't.
- **Run animation:** the run cycle now plays slower when moving slower (½× to 1×), so the boots don't appear to slide while accelerating or slowing down.
- **Forest seams:** examined on a composite image. The mirror seam at the plate's *left* edge doubles a tall tree into an obvious symmetric shape; the seam at the *right* edge is unobtrusive.
  - Parallax was reduced from 0.25 → **0.12** and its starting phase set so that **only the unobtrusive seam** ever appears across the tutorial's full camera range. This is tested.
  - Proper parallax needs wider or tileable plates.
- **Unchanged:** the pivot / feet alignment from M1.5 still holds. The simulation does not depend on rendering: the autoplay result went from 0:30.87 to 0:30.88 because of the collider change only.

### 4. Interface (Phase 4)
- **Top-left status bar:** labelled **TIME · DEATHS · CHECKPOINT k / 5**. Labels are 13 px, values 18 px, on a dark rounded panel.
- **The small hint along the bottom edge is gone.** In its place is a controls guide in the lower middle at the start of play. It fades out over 0.6 s once the prince moves 24 px from the spawn point and does not come back.
- The controls are also listed in the pause panel, so they can always be looked up again.
- Pause, restart, victory, checkpoint toasts and the death flash are kept.
- **Layout bug fixed:** UI padding given as a percentage is computed from the *width*, which pushed the guide off screen. It now uses pixels.

## New regression tests

**game_core (+7, 97 total):**
- `collider_matches_prince_art_dimensions`
- `head_stops_exactly_at_ceiling`
- `low_clearance_passage_fits_only_if_taller_than_collider`
- `landing_under_low_ledge_keeps_head_clear`
- `wall_contact_registers_at_head_height`
- `walk_under_clearances_fit_the_collider`
- `respawning_at_each_checkpoint_is_stable`
- The corner-correction test was made real.

**game_bevy (+6, 34 total):**
- `readiness::needs_art_and_consecutive_settled_frames`
- `world::camera_relative_rounding_removes_one_pixel_wobble`
- `world::render_x_stays_within_one_pixel_of_world_position`
- `background::tutorial_never_shows_the_left_edge_seam`
- `animation::run_cycle_speed_follows_ground_speed`
- `hud::guide_stays_until_player_moves_then_fades`

## Test results (final, `CARGO_BUILD_JOBS=4`)
| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | pass |
| `cargo test --workspace` | pass: game_core **97** (+1 ignored statistics test), game_bevy **34**, asset_pipeline 8 + 4 |
| `cargo clippy --workspace --all-targets -- -D warnings` | pass |
| `cargo check -p game_app` | pass |
| `cargo build -p game_app` | pass |

These pass with the **new collider**:
- The route solver and deterministic replay: the full level twice, 0 deaths, all 5 checkpoints in order.
- Continuing from every checkpoint.
- The margin test with 5% weaker physics.
- The teaching-order tests: areas 1/2/4 and the final descent can be done without wall jumps, while the chimneys cannot.

## Autoplay
`CROWNWIND_AUTOPLAY=1`, real window: **0:30.88, 0 deaths, 5/5 checkpoints, victory panel shown** (`end_victory.png`). The route is 1854 fixed steps.

## Screenshots (`crates/game_app/screenshots/m1_6/`; I looked at each one)
| File | Shows |
| --- | --- |
| `before_fix_black_1.0s.png` | Before the fix: black world at 1.0 s with only the HUD visible |
| `loading_1.2s.png` | After the fix: same moment, only the loading text |
| `start_guide.png` | Start: status bar and controls guide |
| `guide_faded.png` | Guide faded once movement started |
| `paused_controls.png` | Pause panel with the controls |
| `middle_chimney.png` | Wall jump in the first chimney (overhang, flags) |
| `end_victory.png` | Exit and victory panel |

## Automated vs. manual
- **Everything here was tested automatically:** unit and regression tests, route solving and replay, the autoplay run in the real window, and screenshots I inspected.
- **I did not play the game by hand in this round.** The user's earlier manual result (1:16.55, 1 death, 5/5) was on the **old collider**.
- The new collider (bigger head and body) changes how close calls feel, especially the chimneys, the narrow pillars and jumping under ledges. **It needs to be played by hand.**
- Jitter, seams and animation smoothness can only be partly judged from still images; play-testing is needed to confirm them.

## Still open / limitations
1. **Loading time:** about 1.7–2 s of loading screen at startup in the debug build. The release build is probably shorter, but I did not measure it. Pipelines could be shortened by disabling unused render plugins (3D/PBR), but I left that out: risky and out of scope.
2. When the prince faces a wall, his face can overlap the wall by up to 2 px and the cape by more. The collider deliberately matches the body, not the cape.
3. Forest parallax is small (0.12) because of the plate width; it needs wider/tileable art.
4. Vertically the prince still uses plain rounding. While the camera eases vertically in the air there could, in theory, be a 1 px vertical shift on screen; this has not been confirmed visually.
5. The start guide only appears near the level spawn point; it does not show when starting from a checkpoint with the debug switch.

## Recommended next steps
1. **Manual play-test** with the new collider: chimneys, pillars, under-ledge jumps, the feel of the jump. Then decide on fine-tuning.
2. Measure startup time in a release build; if needed, cut unused render plugins and measure again with the trace.
3. Art requests: wider/tileable backdrop plates, checkpoint and door sprites, wall-slide and landing frames.
4. Load levels from data in `content/levels` and run the same tile, route and readiness tests against them.
