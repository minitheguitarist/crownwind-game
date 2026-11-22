# Windwood Integration Report (Milestone 1.5)

Worktree `/home/smurat/Projects/crownwind-integration`, branch `integration/m1-preview`. I confirmed that the gameplay branch (`c341cef`) and the asset branch (`f63672a`) were already merged before I started. Nothing was pushed or merged into `main`.

I only changed `crates/game_bevy/**` and `crates/game_app/**`. Assets, content, contracts, root config, `Cargo.lock`, `game_core` and `asset_pipeline` were not touched. No dependencies were added. Gameplay physics are unchanged: the autoplay route, which runs the real game, still finishes in exactly **0:30.87 with 0 deaths**, the same as before the art was added.

## Commits
| SHA | Content |
| --- | --- |
| `f70dc73` | Windwood art, prince animation, background plates, letterboxed camera |
| `23b2176` | Prince shown in the idle pose at spawn, gameplay waits for the art to load, screenshots |
| (next commit) | this report |

## What was integrated
### Phase 1: prince animation
- `assets/characters/prince_debug.png` (14 frames of 32×48) replaces the debug rectangle.
- **Frame numbers come from the manifest.** `content/characters/prince_debug.json` is built into the binary with `include_str!` and parsed by a small JSON reader. I wrote my own reader because `Cargo.lock` is read-only, so I could not add `serde_json`. The manifest is checked at startup: every frame must be inside the 14-frame atlas, fps must be between 1 and 60, and the pivot must be inside the frame. A test also reads the PNG header to confirm the atlas is 448×48.
- **Choosing the state:**
  - Standing → idle or run. To avoid flicker, run starts above 20 px/s and only stops below 8 px/s.
  - In the air → jump while rising, fall while falling.
  - When the prince steps off a ledge or down a step, the ground pose is kept for a moment, so it doesn't flicker to the fall pose.
  - Right after spawn or respawn, before the simulation has taken a step, a ground check keeps the pose at idle. Without it the prince showed the fall pose here; the screenshot exposed this and I fixed it.
- **Time:** frames advance with `Time::delta_secs` and freeze while paused. A test confirms 30, 60 and 144 Hz show the same frame after the same time. Looping and non-looping clips (jump and fall hold their last frame) follow the manifest.
- **Pivot (16,44):** the sprite is a child of the feet point (the collider's bottom centre), offset (0, +21). This puts **sole row 44** exactly on the ground line.
  - Note: the art spec says the pivot line is "below sole row 43", but in the atlas the lowest opaque row of the grounded frames is 44. Using row 44 as the contact row keeps the boots from sinking 1 px into the ground.
  - Facing left is done with `flip_x`. Since the pivot is the frame's centre column, the feet do not shift when the sprite flips.
- **Atlas:** `TextureAtlasLayout::from_grid` with no padding, nearest-neighbour sampling, integer positions and an even frame size. Every texel lands on a whole screen pixel, so neighbouring frames do not bleed in.
- **The collider is unchanged** (12×24). The art is about 41 px tall, so the head extends about 17 px above the collider (see limitations).

### Phase 2: Windwood terrain (`tiles.rs`, pure functions with tests)
- **The collision cells map one to one onto tiles.** The tests check that every solid cell gets exactly one terrain tile and that no tile sits outside a solid. The collision geometry and level layout are unchanged.
- **Tile rules:**
  - A cell open above → grass (tile 0). The left and right ends of a surface use the edge tiles (5/6).
  - Narrow tall columns (≤2 tiles wide, ≥4 tall) → stone (3) with a moss-stone top (4).
  - Soil → earth tiles 1/2, varied with a fixed hash so the pattern is the same every run.
  - More than 3 rows below the surface → stone strata.
- **Thorns (tile 10):** drawn over every hazard rectangle and sunk 4 px into the ground. Since the top 4 rows of the tile are transparent, the visible thorn is exactly as tall as the 8 px hazard rectangle. Tested.
- **Decoration:** ferns (7), flowers (9) and roots (8; in soil only, never in stone).
  - They have no collision.
  - They are never placed within 1 tile of a hazard, checkpoint, exit or spawn point (tested).
- **Checkpoints and exit door:** no art exists for these yet, so they stay simple shapes recoloured with the Windwood/prince palette.
  - Checkpoint pole: ink colour. Flag: slate blue, turning brass once reached.
  - The pole now stands 14 px to the left of the respawn point. In the first screenshot the prince hid it.

### Phase 3: background (`background.rs`)
- The plates are not seamless horizontally, so they are never naively repeated.
- **Sky:** locked to the view, so it reads as infinitely far away.
- **Forest:** horizontal parallax at 0.25, using a strip of copies that alternate normal / mirrored. Each mirrored copy shares the same edge column with its neighbour, so there is neither a hard seam nor a gap. Tests: copy coverage, alternating mirroring, landing on whole pixels.
- Vertically the forest is locked to the bottom of the view, so no edge can show.
- **Limitation (for later art work):** the README's suggested parallax factors (0.15 for sky, 0.4 for forest) are not possible with 480-wide plates. Real parallax needs wider or truly tileable plates. The mirror symmetry could also become noticeable on a careful look.

### Phase 4: pixel-art presentation
- **The view is always exactly 480×270 world pixels**, scaled up by an integer factor. When the window is larger it is **letterboxed** (unit-tested), so a plate never ends up smaller than the view. Before, the view grew with the window, which would have exposed the plate edges.
- The camera can no longer look below the bottom of the terrain, so the backdrop never shows underneath the ground.
- The camera, the prince, tiles and particles are all snapped to the 1-world-pixel grid. The plates follow the already-snapped camera.
- Draw order (back to front): sky, forest, door, terrain, roots, plants, thorns, checkpoint, prince, particles.
- The HUD, pause, checkpoint counter, victory panel, toasts and death flash still work (seen in the screenshots).
- **Loading gate:** the simulation and the debug screenshot timer wait until the 4 images have loaded.

## Test results (final state, `CARGO_BUILD_JOBS=4`)
| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | pass |
| `cargo test --workspace` | pass: asset_pipeline 8 + 4, **game_bevy 28**, game_core 90 (+1 ignored), game_app 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | pass |
| `cargo check -p game_app` | pass |
| `cargo build -p game_app` | pass |

The new game_bevy tests cover:
- manifest parsing and validation;
- the PNG dimensions;
- animation state, hysteresis, the ledge grace period and the idle pose at spawn;
- frame timing and refresh-rate independence;
- the pivot offset;
- matching tiles to collision, grass/edge rules, thorn alignment and decoration placement;
- forest coverage and mirroring;
- the letterbox calculation.

## Screenshots (taken from the real app; I looked at each one)
In `crates/game_app/screenshots/`: `start.png`, `paused_spawn.png`, `checkpoint3.png`, `auto_chimney.png` (wall jump in the first chimney), `auto_spikes.png` (spike pit), `auto_exit.png` (exit and victory panel). There is also `cold_start_black.png` (see limitations).

What I checked in them:
- **Sprite scale:** correct, 1 art pixel = 2 screen pixels at 960×540.
- **Feet:** sit on the grass line; the sprite does not sink into the ground.
- **Animation frames:** idle at the start and on the checkpoint, the run cycle, jump/fall in the air.
- **Background:** no gaps.
- **Draw order:** correct.
- **Thorns:** sit inside the pit and are readable.
- **Collision vs. visuals:** for the terrain this is guaranteed by the tests.

**Pixel shimmer** cannot be judged from still images. It is only argued from the code (everything sits on the integer grid) and has not been confirmed by a person watching it move.

Autoplay (`CROWNWIND_AUTOPLAY=1`) finished the whole level in the real window: 0:30.87, 0 deaths, 5/5 checkpoints, and the victory panel appeared. `cargo run` resolves the asset path correctly; I took a screenshot to confirm.

## Known visual limitations
1. **Black world on the first launch after a fresh build.** On the first launch of a newly built binary, the world can stay black for about 1–2 s while the UI text is visible (`cold_start_black.png`). The art had already loaded by then, so this is most likely the GPU driver compiling shaders for the first time. It did not happen on later launches of the same binary. The gameplay timer is already running during this, and I could not find a reliable way to gate on it.
2. **Collider vs. sprite height.** The sprite is about 41 px tall, the collider 24 px. When jumping under a ledge, the head can visually go into the ledge. Fixing this means making the collider taller (≈12×38), which is a `game_core` change I am not allowed to make here, and the level would then need to be checked again with the route tests.
3. **Background parallax** is limited (sky fixed, forest at 0.25 and mirrored); see Phase 3.
4. **Checkpoints and exit door** are still palette-coloured simple shapes; there is no art for them.
5. Thorns are 16 px wide, so hazards must be whole tiles wide; all hazards in the current level are.
6. There is no wall-slide animation; the fall pose is used, plus the dust particles.
7. Shimmer, jitter and the mirrored-forest symmetry have not been checked by a person watching the game move.

## Manual testing
```bash
cd /home/smurat/Projects/crownwind-integration && CARGO_BUILD_JOBS=4 cargo run -p game_app
```
- Controls: A/D or Left/Right to move, Space to jump (hold for higher), jump while touching a wall to wall-jump, R to restart, Esc to pause.
- Start at a checkpoint: `CROWNWIND_CHECKPOINT=2 cargo run -p game_app` (2 = the first chimney).
- Watch the game play itself: `CROWNWIND_AUTOPLAY=1 cargo run -p game_app`.
- What to look at:
  - whether the run cycle's speed matches the movement speed;
  - the jump/fall transition;
  - whether the forest's mirror symmetry is noticeable;
  - head clipping under ledges;
  - in a resized window, whether the letterbox bars appear and the pixels stay sharp.

## Suggestions for the next milestone
1. **Collider vs. art:** decide whether to make the collider ≈12×38, then run the route and teaching-order tests again in `game_core`.
2. **Art requests:**
   - wider or truly tileable backdrop plates (at least 2–3 views wide), so real parallax becomes possible;
   - checkpoint and exit-door sprites;
   - wall-slide and landing frames.
3. **Load levels from data:** map `content/levels` (`level.schema.json`) into `game_core::Level` and run the existing tile mapper and route tests on loaded levels too.
4. A **render-to-texture** pipeline at 480×270, for completely shimmer-free scrolling and simpler letterboxing.
5. **Warm up shaders/pipelines** with a short loading screen, so the black frames on first launch go away.
