# Claude — First Shift Report (gameplay)

Branch: `agent/claude-gameplay` (worktree `../crownwind-claude`), based on `181d3a2`.
Toolchain: rustc/cargo 1.100.0 (`~/.cargo/bin`, not on the default PATH of the agent shell). Bevy `=0.20.1` unchanged. Builds used `-j 4`.

## What was built
- **`game_core` (pure Rust, no Bevy)**
  - `geometry`: `Vec2`, `Rect` (bottom-left + size, strict-overlap AABB). World units = pixels, +y up.
  - `movement`: fixed-step `step()` with run speed, ground/air acceleration & friction, gravity, max fall speed, jump, variable jump height (release cuts upward velocity), **coyote time (0.10 s)**, **jump buffer (0.12 s)**, axis-separated AABB collision (X then Y) with landing/ceiling/wall resolution. Player hitbox 12×24 px; position = feet (bottom-center). `FIXED_DT = 1/60 s`.
  - `simulation`: `Simulation` with accumulator (`advance(frame_dt, input)`), latching of jump presses across zero-step frames, max 8 steps per frame (hitch cap), render interpolation (`interpolated_position`), kill-plane reset, goal detection.
  - `level`: explicit 16 px tile grid; procedural placeholder `test_course()` (step, wall jump, gaps, floating stones, low-ceiling tunnel, stair to a goal ledge, boundary walls).
  - `camera`: frame-rate independent follow (dead zone + exponential smoothing), clamped to level bounds, snap on spawn/reset.
- **`game_bevy`**: `GamePlugin` — orthographic camera with `ScalingMode::FixedVertical { 270 }` (480×270 virtual view), colored-rectangle placeholders (platforms with a top lip, red hero with facing marker, yellow goal), keyboard input (A/D or ←/→ move, Space/W/↑ jump, R reset), simulation driven from `Update` via the core accumulator (rendering decoupled, interpolated), smoothed follow camera, HUD with controls hint and reset/goal status.
- **`game_app`**: 960×540 window (integer 2× of the virtual viewport), `ImagePlugin::default_nearest()`.

No external assets, no Codex files, no shared/root files touched.

## Changed files
- `crates/game_core/src/{lib.rs, geometry.rs, movement.rs, simulation.rs, level.rs, camera.rs}`
- `crates/game_bevy/src/lib.rs`
- `crates/game_app/src/main.rs`
- `docs/reports/claude-first-shift.md`

## Checks actually run (in the worktree)
| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | pass |
| `cargo test -p game_core` | **29 passed, 0 failed** |
| `cargo clippy -p game_core -p game_bevy -p game_app --all-targets -- -D warnings` | pass (after fixing one `is_multiple_of` lint in a test) |
| `cargo check -p game_app` | pass |
| `cargo build -p game_app` | pass (~3 min debug build) |
| `timeout 12 ./target/debug/game_app` | started on Fedora 45 / Wayland, NVIDIA RTX 4060 adapter, window "Crownwind (prototype)" created, ran until timeout; no panic/error/warning lines in the log |

Core tests cover: landing on floor top, resting stability, max run speed and friction, wall blocking from both sides, ceiling hit, normal jump arc, variable jump height, **buffered jump firing on landing**, stale buffer expiry, **coyote jump allowed**, coyote expiry, no double jump, fall speed clamp, determinism, **frame-rate independence (30/60/144 FPS → identical state)**, jump-press latching, kill-plane reset, hitch cap, level spawn/goal sanity, camera dead zone / no overshoot / FPS-independent smoothing / bounds clamp.

`cargo clippy --workspace` / `cargo test --workspace` were not run, because they include the Codex-owned `asset_pipeline` crate that I don't own.

## Known limitations
- **I did not play the game interactively or take a screenshot**: no screenshot tool was available. I confirmed that the app starts and stays up, but not how it looks or feels. The course jumps were checked only on paper against the jump height (≈61 px peak, ~130 px/s run). No automated test plays the course from start to goal.
- The fixed step uses the core accumulator inside Bevy's `Update` instead of Bevy's `FixedUpdate`. This was deliberate (deterministic, easy to test, no lost input edges), but it is worth knowing during integration.
- Render positions are not snapped to whole pixels. With rectangles this is invisible, but sprite art may shimmer by a sub-pixel at 2× scale. Consider snapping the sprite and the camera together once there is real art.
- The level is procedural and hard-coded. It does not yet load `docs/contracts/level.schema.json` data.
- The window is resizable. `FixedVertical` keeps the 270 px height but does not force integer scaling at other window sizes.
- Tuning values are first guesses.

## Suggested next steps
1. Do a manual play session and tune `MovementConfig` (feel, jump height, coyote and buffer windows).
2. Add a scripted "bot run" test that finishes `test_course()` start to goal, to guarantee it can be traversed.
3. After the Codex shift is merged, map the level schema (`tile_size`, `spawn`, `solids`) onto `game_core::Level` in an integration branch.
4. Add pixel snapping and integer-scale letterboxing for real pixel art.
5. Add a player state enum (idle/run/jump/fall) for the future animation manifest. Do not build an animation engine yet.
