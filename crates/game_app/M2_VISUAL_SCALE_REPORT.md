# M2-A Visual Scale Migration Report

Branch `integration/m2-visual-scale`, created from `integration/m1-polish` (`8ec1791`).

## 1. Migration plan (written before implementation)

### Current state (M1.6)
- `game_core` works in **gameplay units**, where 1 unit = 1 pixel of the old art. That means:
  - tiles are 16 units;
  - the prince frame is 32×48, with pivot (16,44) and a 14×38 collider;
  - the physics is tuned in these units (run 130 u/s, jump 350 u/s, gravity 1000 u/s²);
  - the 5-area tutorial, route solver, deterministic replay and teaching-order tests all rely on these values.
- `game_bevy` draws one gameplay unit as one virtual pixel, on a 480×270 view with integer upscaling.

### Target
- Characters 64×96 texels, tiles 32×32 texels.
- Internal render 640×360, nearest-neighbour sampling, integer upscaling.

### Options considered
| Option | What it means | Consequence |
| --- | --- | --- |
| A. Double the world | Multiply every length in `game_core` by 2: tile 32, collider 28×76, speeds ×2, gravity ×2, level ×2 | Feel is unchanged in theory, but about 90 tests with hard-coded pixel values would need rewriting. Many constants (sub-step, probes, nudges, forgiveness) would need re-tuning. The validated level proofs would have to be rebuilt. High risk, no gameplay gain. |
| **B. Change the render scale (chosen)** | `game_core` stays in gameplay units. The renderer draws **2 render pixels per gameplay unit** | Physics, level, route proofs and determinism stay exactly the same. A 32-texel tile covers exactly one 16-unit cell, and a 64×96 character covers exactly the current 32×48-unit character cell. |

### Assumptions
1. Ratios stay the same at the new standard. A 64×96 character is 2×3 tiles of 32 px, the same footprint as 32×48 on 16 px tiles. So **the character's size relative to the world does not change**; what changes is the detail (texel density) and how much of the world the camera shows.
2. The 640×360 render shows **320×180 gameplay units** (20×11.25 tiles) instead of 480×270 (30×17 tiles). The camera is effectively zoomed in 1.5×. The character is 27% of the screen height instead of 18%. This is the expected result of "a larger, more expressive character". The follow settings (dead zones, look-ahead, height offset) need adjusting for the smaller view.
3. The virtual pixel grid becomes **0.5 gameplay units**. The camera, characters and particles snap to that grid, so motion actually gets finer.
4. Each art set declares its texel density:
   - existing (legacy) 16 px tiles, the 32×48 prince and the 480×270 plates: **1 unit per texel** (2×2 render pixels per art pixel);
   - new standard art (32 px tiles, 64×96 characters): **0.5 units per texel** (1 render pixel per art pixel).
   Placement is computed from texels × density, so new art can be dropped in without code changes.
5. The backdrop plates (480×270) are larger than the 320×180 view at 1 unit per texel. That removes the edge problem, but at 640×360 they can only be shown cropped:
   - the sky is aligned to the **top** of the view, so the sun and mountains show;
   - the forest is aligned to the **bottom** of the view, so all the tree rows show.
6. There is currently **no 64×96 character art** in this branch or in any other. The approved princess exists as the "Princess V2" master on the `agent/codex-character-rebuild` branch, but at 32×48. The sandbox therefore:
   - can load any sprite PNG at runtime (size, pivot and density given by environment variable);
   - also provides a procedural 64×96 "mannequin" calibration figure drawn in code.
7. HUD text is not pixel art. It uses a `UiScale` based on the viewport height, so it keeps the same proportions as before (designed for 540 px height).

### Steps
1. Rendering: constants for 640×360 / 2 px per unit, camera projection 320×180 units, letterbox at 640×360 multiples, snapping to the 0.5-unit grid.
2. Art density: compute tile, character and backdrop sprite sizes and pivot offsets from texel density; read the character density from the manifest frame height (48 → 1.0, 96 → 0.5).
3. Camera: follow settings for the smaller view; vertical alignment of the backdrops; forest seam analysis for the new view width.
4. Gameplay: no change to physics, level or collider. Verify with all `game_core` tests and autoplay.
5. Sandbox: `CROWNWIND_REVIEW=1`. A controllable prince with real physics on a test stage, plus a static lineup on the floor, under a ceiling and against a wall. Collider, pivot and tile-grid overlays; a fixed/follow camera toggle.

## 2. What was migrated (implementation)

**Rendering (`game_bevy`):**
- `RENDER_WIDTH/HEIGHT = 640×360` and `PIXELS_PER_UNIT = 2`; the visible area is `VIEW = 320×180` gameplay units.
- The camera projection is a fixed 320×180 units. The window opens at 1280×720 (2×) and is letterboxed at integer multiples of 640×360: 1280×720 → 2×, 1920×1080 → 3×, 2560×1440 → 4×.
- HUD text uses a `UiScale` tied to the viewport height, so it keeps its M1.6 proportions. HUD text is not pixel art.
- The camera, characters and particles snap to the **render-pixel grid of 0.5 units**. Camera-relative horizontal rounding is kept, so there is no 1 px wobble.
- **Art density:**
  - Characters: `character_units_per_texel(frame_height)` gives 48 texels → 1.0 and 96 texels → 0.5. The frame size (`custom_size`) and pivot offset are computed from texels × density.
  - Tiles always cover one 16-unit cell (`custom_size`).
  - So 64×96 / 32×32 art can be dropped in without code changes. Tests cover that legacy and M2 frames land on the same cell and the same feet line.
- **Camera framing (`m2_camera_rules`):**
  - Standing, the feet are 55 units (31%) above the bottom edge; the character is 27% of the screen height.
  - Look-ahead is 28, the horizontal dead zone 12, and the airborne zone 62 up / 30 down.
  - A full jump (~61 units) does not move the camera vertically, and the head at the top of a jump stays on screen (tested).
- **Backdrops:** the 480×270 plates are larger than the 320×180 view.
  - The sky is aligned to the top of the view, so the sun and mountains show.
  - The forest's bottom is sunk 45 units below the view bottom, putting the tree line at ~58% of the view height so the mountains stay visible.
  - The forest uses mirrored copies with phased 0.12 parallax. The obvious seam still never shows across the whole tutorial camera range (test updated for the new view width).

**Gameplay (`game_core`):** **no physics, collider or level change.** Only the level helpers (`ground`, `ledge`, `wall`, `spikes`) were made `pub` so the sandbox can use them. The course logic and layout are unchanged. The 32-texel tile standard maps exactly onto 16-unit cells, so no visual re-layout was needed.

**Sandbox (`game_bevy/src/review.rs`):** `CROWNWIND_REVIEW=1`.
- A controllable prince with the real physics, camera and pixel snapping, on a test stage with a floor, a 48-unit ceiling (the tutorial's lowest walk-under clearance), a 6-tile wall, a 1-tile step, thorns and a ledge.
- **Static lineup:** the legacy prince, a 64×96 **calibration mannequin** drawn in code (pivot (32,90), height ruler, facing dot), and any sprites given in `CROWNWIND_REVIEW_SPRITES="path:WxH:px,py;..."`.
- The last candidate also stands under the ceiling and against the wall, and the prince stands on the other side of the wall.
- **Overlays:** the 14×38 collider (cyan, 1 render px thick) and the feet pivot (magenta); `G` toggles the 16-unit tile grid.
- **Camera:** `F` cycles follow (gameplay framing) → lineup → ceiling+wall. For automated captures, `CROWNWIND_REVIEW_FIXED=1|2` and `CROWNWIND_REVIEW_GRID=1`.

**Pivot convention (decided here, for M2):**
- In the sandbox and for M2 art, `pivot.y` is the **ground contact line**: the top edge of the first empty row below the sole. This is how the art docs define it.
- Princess V2 follows it (sole row 43, anchor 44).
- **The legacy prince atlas does not:** its sole row is 44 while the manifest also says 44. The game's manifest path keeps its existing +1 correction for the prince, and in the sandbox the prince is shown with y 45. When the M2 prince is drawn, the art and the manifest must follow the same convention.

## 3. Verification

| Command (`CARGO_BUILD_JOBS` 1–4) | Result |
| --- | --- |
| `cargo fmt --all -- --check` | pass |
| `cargo test --workspace` | pass: game_core **97** (+1 ignored statistics test), game_bevy **42**, asset_pipeline 8 + 4 |
| `cargo clippy --workspace --all-targets -- -D warnings` | pass |
| `cargo check -p game_app` | pass |
| `cargo build -p game_app` | pass |

- **Autoplay** (real window, new scale): **0:30.88, 0 deaths, 5/5 checkpoints, victory panel**. This is the same result as M1.6 (1854 fixed steps), as expected since the physics did not change.
- **The current level is still playable:** all `game_core` route, replay, checkpoint-continuation and teaching-order tests pass unchanged.
- **New tests (game_bevy +8):**
  - 640×360 view;
  - letterbox and integer scale;
  - M2 camera framing;
  - legacy vs. M2 frame cell and feet;
  - vertical coverage of the backdrop plates;
  - sprite-spec parsing;
  - that the mannequin's sole row matches its pivot;
  - the sandbox's reference clearances.
- **Build environment note:** during this run the machine's RAM and 8 GB zram swap were full, mostly from other open applications. The first `cargo test` with two parallel links got stuck in swap for 20+ minutes. I stopped my own run and repeated it with `CARGO_BUILD_JOBS=1`; it passed.

### Screenshots (`crates/game_app/screenshots/m2/`, 1280×720; I looked at each one)
| File | Shows |
| --- | --- |
| `game_start.png` | Level start: framing, mountains and forest, controls guide |
| `game_end_autoplay.png` | Exit and victory panel at the new scale |
| `review_follow.png` | Sandbox in follow mode: controllable prince + lineup |
| `review_lineup.png` | Lineup: prince, mannequin, Princess V2 32×48, Princess V2 test 64×96 |
| `review_ceiling_wall.png` | Under the 48-unit ceiling and against the wall, with the grid |

The Princess V2 images in the screenshots were **loaded temporarily** from the `agent/codex-character-rebuild` branch through a test asset folder in the scratchpad (`BEVY_ASSET_ROOT`). **No princess file was added to this branch.** `princess_v2_x2_test.png` is a 2× nearest-neighbour copy made only to exercise the 64×96 density path; it is not real M2 art.

### Observations from the screenshots
- All figures' soles sit on the grass line; the pivot markers are on the surface.
- Heads go **2–4 units** above the 14×38 collider; for the mannequin it is 4 units, by design. Under the 48-unit ceiling there are **~6 units** of visual head room.
- At the wall, Princess V2's face visually overlaps the wall face by about 2 units, and the prince's cape at the back overlaps a little. The collider is set to the body, as decided earlier.
- **Framing:** the character takes up 27% of the screen height. 20 tiles are visible horizontally and the jumps in the opening area fit on screen comfortably. This is a **zoomed-in** view of the old 30-tile world; I have not **verified by hand** whether it feels too tight in the long jumps and chimney climbs.

## 4. What is still placeholder
- **Every piece of art is still legacy density** (1 art pixel = 2×2 render pixels): the Windwood tiles (16 px), the prince atlas (32×48), and the sky and forest plates (480×270).
- There is **no M2 art yet** (64×96 characters, 32×32 tiles, plates at least as large as 640×360). The only 64×96 figure is the procedural mannequin.
- The checkpoint flags, door, particles and HUD shapes are colour placeholders.
- Once M2 art arrives, legacy and M2 densities will sit side by side for a while, e.g. 64×96 characters on 16 px tiles. This is a temporary inconsistency; the tile set should switch to 32 px together.

## 5. What has to happen next for full character integration
1. **The 64×96 princess and prince:** an animation atlas per character, plus a manifest that follows the **contact-line pivot convention** (e.g. `content/characters/princess.json`, frame 64×96). Code changes needed: a manifest/atlas path for each character (only the prince is wired today) and selecting which character is playable.
2. **Collider:** the 14×38 collider was measured from the legacy prince. Measure the M2 art's body and head line in the sandbox, then decide whether the collider changes. If it does, run the `game_core` route and clearance tests again.
3. **32×32 Windwood tiles:** to keep the same indices, an 11×1 atlas of 32 px (or a new mapping); change `TILE_TEXELS`.
4. **Backdrops:** at least 640×360, preferably wider or tileable plates. That would allow real parallax and get rid of the mirroring.
5. **Manual play-test:** the new framing in the chimneys and long jumps; if needed, tune `m2_camera_rules`.
6. Have Codex fix the prince atlas / manifest pivot off-by-one in the art data.
