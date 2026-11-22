# Crownwind — Second Shift Report (Gameplay Milestone 1)

Branch: `agent/claude-gameplay` (worktree `/home/smurat/Projects/crownwind-claude`), on top of the first-shift commit `c64ebea`. Nothing was merged or pushed. No files outside `crates/game_core`, `crates/game_bevy` and `crates/game_app` were changed. No dependencies were added; the workspace still uses Bevy `=0.20.1`. Builds used `-j 4`.

## Commits
| SHA | Phase |
| --- | --- |
| `390d73b` | 1 — wall slide, wall jump, sub-stepped collision, corner correction |
| `3be5dca` | 2 — hazards, checkpoints, death/respawn lifecycle (also the first camera, HUD, pause and pixel-snapping pass) |
| `dc9d90c` | 3 — five-area tutorial level, plus a route finder and traversal tests |
| `50fec2f` | 4 — look-ahead stability, HUD polish, auto-pause on focus loss, debug autoplay and screenshot switches |
| (next commit) | this report |

## Implemented mechanics
**Movement (`game_core::movement`, pure Rust, fixed 60 Hz step)**
- Kept from shift 1: acceleration and friction, gravity, max fall speed, variable jump height (releasing jump cuts upward speed), coyote time (0.10 s), jump buffer (0.12 s).
- **Wall detection:** a 1 px probe on each side, active only while airborne.
- **Wall slide:** while pressing into a wall and falling, fall speed is capped at 70 px/s. Dust particles show it.
- **Wall jump:** the impulse is always exactly −170 px/s sideways and +330 px/s up. Sideways input is ignored for 0.14 s afterwards, so the push-off is always the same. There is 0.08 s of wall coyote time, and the jump buffer also works for wall jumps. A ground jump takes priority over a wall jump.
- **No endless climbing:** you cannot wall-jump twice in a row from the same side. This resets when you land or touch the opposite wall. Chimneys (alternating walls) can be climbed; a single wall cannot. Tests check both.
- **Collision robustness:** each axis move is split into sub-steps of at most 4 px (tested at 6000 px/s against 4 px walls and floors). The ground check uses a half-pixel probe. The head gets a sideways nudge of up to 4 px when it clips a ceiling corner. Strict-overlap AABBs mean the player does not snag on floor seams or ledge sides.

**Lifecycle (`game_core::simulation`)**
- The game is in one of three explicit phases: `Playing`, `Dying` (0.6 s), or `Completed`.
- A `GameEvent` stream reports: died (cause: hazard or fall), respawned, checkpoint activated, level completed, and level restarted.
- Hazards use hitboxes shrunk by 2 px. Each contact kills only once, and the player is frozen while dying.
- Checkpoints are ordered. Each one activates once and never moves backwards. You respawn at the latest active checkpoint.
- `restart_level()` (R key) clears checkpoints, deaths and the timer. The game also tracks restart and teleport counters, an elapsed-time timer, and pause.
- `start_at_checkpoint(n)` exists for practice and debugging.

**Level (`game_core::tutorial`)**: about 3.4k px long, with 5 checkpoints, built on a 16 px tile grid.
1. **Opening:** running, a hop, a 2-tile pit, a staircase.
2. **Landing practice:** pillars and ledges over pits with varied spacing and height, ending with a 4-tile running jump.
3. **Walls:** walk under an overhang into a 3-tile-wide chimney, climb it by wall-jumping, and exit onto a high ledge.
4. **Hazards:** floor spikes, then a spike pit crossed on floating ledges.
5. **Finale:** a taller chimney, then descending ledges over a long spike pit to a gold exit door.

**Presentation (`game_bevy`)**
- 480×270 virtual view with **integer scaling** (largest whole multiple that fits the window).
- The camera, the player and the particles are **snapped to the world-pixel grid**, while the simulation itself stays smooth. Sprites use even sizes so their edges land on the grid.
- Camera behaviour:
  - Horizontal dead zone.
  - A look-ahead that only changes while you are running on the ground, so it does not swing back and forth during chimney wall jumps.
  - The vertical position snaps to the ground height when you land, ignores ordinary jump arcs, and follows big falls and climbs.
  - Exponential smoothing, so camera motion does not depend on frame rate.
  - The camera snaps instantly on respawn.
- Visuals: spike triangles, checkpoint flags (grey, turning green), a gold door, and a placeholder hero (body, facing marker, and a cape that reacts to speed).
- Death feedback: a particle burst, a red screen flash, and the hero is hidden while dying. A checkpoint shows a green burst and a toast message. Completion shows a gold burst and a victory panel with time, best time and deaths.
- HUD: time, deaths and checkpoint k/5, plus a controls hint.
- **Esc** pauses and resumes the game, and it also pauses automatically when the window loses focus.
- Debug environment variables (they only work when set): `CROWNWIND_CHECKPOINT=n`, `CROWNWIND_PAUSED=1`, `CROWNWIND_AUTOPLAY=1`, `CROWNWIND_SCREENSHOT=file.png`, `CROWNWIND_SCREENSHOT_AT=seconds`.

## Tests and checks (all run in the worktree, final state)
| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | pass |
| `cargo test -p game_core` | **90 passed, 0 failed, 1 ignored** (the ignored one is `print_route_stats`, which only prints statistics) |
| `cargo test -p game_bevy` | **2 passed** (integer scale, time format) |
| `cargo check -p game_app` | pass |
| `cargo clippy -p game_core -p game_bevy -p game_app --all-targets -- -D warnings` | pass |
| `cargo build -p game_app` | pass |

Notable `game_core` tests:
- **Movement (36):** wall slide, the exact wall-jump impulse and lock, refusing a same-side wall jump, a single wall can't be climbed in 10 s of jump mashing, chimney height increases every cycle, wall coyote time works and expires, tunneling at 6000 px/s, landing on 1 px of a ledge, no snagging on edges or floor seams, ceiling corner nudge vs. block, and determinism.
- **Lifecycle (20):** a single death per contact, freezing while dying, respawning at the level spawn or checkpoint, fall death, checkpoint ordering and no backwards activation, a dead player can't activate checkpoints, completion stops the timer and blocks death, restart, pause, and frame-rate independence.
- **Level (11 run + 1 ignored):**
  - **The full-level input sequence, replayed through `Simulation`, finishes with 0 deaths and all 5 checkpoints in order, identically twice.**
  - Every checkpoint respawn can continue to the next point.
  - The level is still beatable with 5% weaker run and jump.
  - Areas 1, 2, 4 and the final descent can be done **without** wall jumps.
  - Neither chimney can be passed without wall jumps (no route found within the 250k-node search budget).
- **Route finder (5 negative/positive checks):** for example, it refuses unreachable ledges, uncrossable spikes, and single-wall climbs.
- **Camera (10), plus geometry (3), the old level module (4) and one smoke test (1).**

## Interactive vs automated testing — read this
- **I did not play the game myself.** Every claim above about the level being beatable comes from the automated route finder and input replay, which use the real movement code. They show the level is physically possible with 100 ms input decisions. They say nothing about whether it is comfortable or fun for a human.
- **Automated app smoke tests (real window, Fedora/Wayland, RTX 4060):**
  - The plain launch ran for 10 s with no panic, error or warning lines.
  - Screenshots were taken at checkpoints 0, 2 and 4 and of the paused state. I looked at them: the HUD, flags, spikes and overhang shapes render correctly.
  - **`CROWNWIND_AUTOPLAY=1`** feeds the planned inputs into the running app. It **finished the course in 0:30.87 with 0 deaths and 5/5 checkpoints, and the victory panel appeared** (screenshot confirmed).
  - These screenshots caught two real bugs, which I fixed: the window auto-paused in a background test run, and the arrow glyphs were missing from the default font.
- The user played and finished the shift-1 prototype. **This milestone has not been played by a human yet.**

## Known problems / limitations
- The tuning values and difficulty are guesses. The chimneys in particular may feel hard or fiddly. Wall-jump strength, the input lock, and the same-side rule all need someone to play-test them.
- Because the camera and the player are snapped to the pixel grid separately, the hero can wobble by 1 world pixel relative to the screen while the camera is easing. The proper fix is to render into a 480×270 texture, or to snap the player relative to the snapped camera.
- The chimney "cannot be skipped" tests only prove that no route was found within the search budget. That is strong evidence, not a mathematical proof.
- The level is still hard-coded in Rust and does not read `docs/contracts/level.schema.json` data yet.
- The HUD uses Bevy's default font, which lacks some characters (for example arrows). There is no pixel font yet.
- There is no audio and no options menu, and the session best time is not saved to disk.
- Debug builds take a few seconds to plan the route for autoplay. This only happens when `CROWNWIND_AUTOPLAY=1` is set.

## Integrating Codex sprite assets later
1. Keep gameplay rules in `game_core`; art only replaces presentation in `game_bevy/src/world.rs`. The simulation's player position is the **bottom-centre of the feet**, and the hitbox is 12×24 px. Sprite frames should use a bottom-centre anchor (`Anchor::BOTTOM_CENTER`) at that point.
2. Choose the animation state from `PlayerState`: `grounded`, `velocity`, `wall_sliding`, `wall_dir`, `facing` and `jumping`, plus `Simulation::phase` and `death_progress()` for death, and `GameEvent` for one-off moments such as landing, checkpoints and completion. Do not let animation drive the physics.
3. Advance animations with real time, or better with `Simulation::steps`, so they stay independent of frame rate. The existing render snapping keeps them on the pixel grid. Keep frame sizes even or use pixel-aligned anchors.
4. Map the level data from Codex's `level.schema.json` (`tile_size`, `spawn`, `solids`) into `game_core::Level`, adding `hazards`, `checkpoints`, `goal` and `kill_y` if the schema doesn't have them yet. This needs a schema/content change that I don't own, so it should be done in an integration branch. The tutorial's tests (spawn safety, route replay, teaching order) can then run against loaded levels too.
5. Use `ImagePlugin::default_nearest()`, which is already set. Do not add non-integer camera zoom.

## Suggested next steps
1. A human play session focused on the chimneys and the area-2 jumps. Then tune `MovementConfig` and re-run the route tests.
2. Render-to-texture at 480×270 for completely shimmer-free scrolling.
3. Loading levels from data in the integration branch.
4. Play-test the controls with a gamepad (Bevy already supports gamepads; input mapping would be a small change in `read_input`).
