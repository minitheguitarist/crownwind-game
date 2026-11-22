# Crownwind (working title)

An original open-source **high-detail pixel-art 2D platform-adventure**, developed in Rust with Bevy 0.20.1.

> **Temporary name:** Crownwind is an internal working title and can be changed before release.
> **Rights:** Do not copy names, art, code, music, storyboards, or level layouts from Forgotten Warrior, Ori, or any other game.

## v0.1.0-beta: first playable beta

The first beta is a complete, short loop: **main menu → Windwood tutorial course → finish**.

- **Main menu:** CROWNWIND logo with three buttons over an animated camp scene in the Windwood forest. The prince and princess sit by a campfire; wind moves their hair, the cape, the ribbon and the dress, they breathe and blink independently, the fire flickers with sparks and smoke, and leaves, grass and the lantern sway.
  - **Tek Oyunculu** (Single player): starts the game with a short fade.
  - **Çok Oyunculu** (Multiplayer) and **Ayarlar** (Settings): visible but disabled (“YAKINDA”, coming soon).
- **Windwood tutorial:** five areas with jumps, wall slides and wall jumps, thorn hazards, 5 checkpoints, death and respawn, a run timer and a best time. The PixelLab-animated prince is the player, and the princess waits at the goal.
- **Presentation:** 640×360 internal resolution with integer scaling and letterboxing, so pixel art stays crisp at any window size. The camera is smoothed and looks ahead. Gameplay is frame-rate independent, running a deterministic fixed-step simulation in `game_core`.

### Controls

| Where | Action | Keys |
| --- | --- | --- |
| Menu | Move selection | ↑ / ↓ or W / S, or hover with the mouse |
| Menu | Activate | Enter / Space or left click (disabled entries never activate) |
| Game | Move | A / D or ← / → |
| Game | Jump | Space, W or ↑ (hold to jump higher) |
| Game | Wall jump | Jump while sliding on a wall |
| Game | Restart / Pause | R / Esc |

The game pauses automatically when the window loses focus.

### Run

```bash
cargo run --release -p game_app
```

The release build gives the smoothest frame rate; `cargo run -p game_app` also works. Assets load from the workspace `assets/` folder (override with `BEVY_ASSET_ROOT`).

### Checks

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

### Developer switches (environment variables)

| Variable | Effect |
| --- | --- |
| `CROWNWIND_SKIP_MENU=1` | Start directly in the game. |
| `CROWNWIND_AUTOPLAY=1` | Plan a route and play the tutorial automatically (also skips the menu). |
| `CROWNWIND_CHECKPOINT=<n>` | Start at checkpoint `n`. |
| `CROWNWIND_REVIEW=1` | Character review sandbox. |
| `CROWNWIND_SCREENSHOT=<file>` (+ `_AT=<s>`) | Save a screenshot and exit; skips the menu unless `CROWNWIND_MENU=1`. |
| `CROWNWIND_MENU_AUTOSTART=<s>` | Press “Tek Oyunculu” automatically after `s` seconds. |
| `CROWNWIND_MENU_FRAMES=<dir>` | Capture menu frames for GIF previews. |
| `CROWNWIND_FPS_LOG=1` | Log frames per second. |

### Known beta limitations

- There is one tutorial level, with no story, dialogue, sound or music yet.
- Multiplayer and Settings are placeholders.
- There is no way back to the menu from the game; close the window to quit.
- Keyboard only; gamepad support comes later.

Reports with details and screenshots:
- [Main menu](crates/game_app/MAIN_MENU_REPORT.md)
- [640×360 visual scale](crates/game_app/M2_VISUAL_SCALE_REPORT.md)
- [`docs/reports/`](docs/reports/)

## Architecture

```
crates/
  game_core/        # Rust-only deterministic player rules (Claude owns)
  game_bevy/        # Bevy integration: menu, movement, camera, levels (Claude owns)
    src/menu/       #   main menu state, UI and animated camp scene
    tools/menu_art/ #   reproducible build of the menu art (Python)
  game_app/         # Binary entrypoint (Claude owns)
  asset_pipeline/   # JSON validation, pixel art tooling (Codex owns)
assets/             # Original or licensed art (Codex owns)
content/            # Data-driven levels, character manifests (Codex owns)
docs/contracts/     # Shared interfaces; never modify unilaterally
prompts/            # One-shot tasks for both agents
scripts/            # Human-controlled setup and launch scripts
```

App flow: `AppState::MainMenu` → `AppState::InGame`. The game world, camera and HUD are spawned once, on entering `InGame`. Gameplay systems only run in that state.

## Safety and collaboration

Start from independent worktrees, never let two agents edit the same directory, and do not auto-merge `main`. Each agent writes a final report and a local commit. Agent runs terminate when their single assigned task completes; they are **not** unlimited overnight workers. Claude uses auto-mode approval classification (availability depends on installed Claude Code and account); Codex uses workspace-write with no prompts and blocked privilege escalation. See [AGENTS.md](AGENTS.md) and [docs/agent-boundaries.md](docs/agent-boundaries.md).

## First-time setup on Fedora 45 — Desktop apps

1. Clone this repository and open a terminal in `~/Projects/crownwind-game/`.
2. Run `bash scripts/desktop-setup.sh` to initialize per-user Rust and **two separate Git worktrees**. This uses no Claude/Codex CLI and does not start either AI app.
3. Optionally publish the public personal GitHub repo when prompted (`gh` authentication may be needed).
4. Open `~/Projects/crownwind-claude` in **Claude Code Desktop** and paste `prompts/CLAUDE_FIRST_SHIFT.md`.
5. Open `~/Projects/crownwind-codex` in **Codex Desktop** and paste `prompts/CODEX_FIRST_SHIFT.md`.

See [Desktop quickstart](docs/desktop-quickstart.md) for exact commands and review steps. The old CLI helpers are left as optional tools, but are not used by Desktop setup.

**Never run both Desktop agents on the same worktree.** Do not grant blanket system permissions. Desktop tasks may stop due to approval requests, quotas, sleep or connectivity, and cannot be guaranteed to work unattended overnight.

## Art direction
Original high-detail pixel art at a **640×360** (16:9) internal resolution, integer-scaled to the window. Gameplay keeps 16-unit tiles drawn at 2 render pixels per unit. Characters use 96×96 frames, and the world design is not fixed by viewport size. The prince has chestnut hair, a night-blue gold-trimmed coat, metal gauntlets and a long burgundy cloak. The princess has long golden hair with a pink flower and turquoise ribbon, and a pink and cream gown with turquoise jewellery. See [docs/game-design.md](docs/game-design.md) and [assets/ART_SPEC.md](assets/ART_SPEC.md).

## Licenses
Rust source is under MIT (see `LICENSE`). Every added asset needs attribution, permission and a compatible license, recorded in `assets/ASSET_PROVENANCE.md`. For the main-menu art, see [crates/game_bevy/tools/menu_art/README.md](crates/game_bevy/tools/menu_art/README.md).
