# Crownwind (working title)

An original open-source **high-detail pixel-art 2D platform-adventure**, developed in Rust with Bevy 0.20.1.

> **Temporary name:** Crownwind is an internal working title and can be changed before release.
> **Rights:** Do not copy names, art, code, music, storyboards, or level layouts from Forgotten Warrior, Ori, or any other game.

## Milestone 0: playable movement slice
- Fedora/Linux desktop first; keyboard first, gamepad later.
- A temporary player that can move, jump and land with a stable camera.
- A tiny obstacle-course scene; simple visual placeholders rather than final art.
- Character and asset contracts checked by an independent Rust CLI.
- Tests and CI; no automatic merging of AI contributions.

## Architecture

```
crates/
  game_core/        # Rust-only deterministic player rules (Claude owns)
  game_bevy/        # Bevy integration, movement, camera, levels (Claude owns)
  game_app/         # Binary entrypoint (Claude owns)
  asset_pipeline/   # JSON validation, pixel art tooling (Codex owns)
assets/             # Original or licensed art (Codex owns)
content/            # Data-driven levels, character manifests (Codex owns)
docs/contracts/     # Shared interfaces; never modify unilaterally
prompts/            # One-shot tasks for both agents
scripts/            # Human-controlled setup and launch scripts
```

## Safety and collaboration

Start from independent worktrees, never let two agents edit the same directory, and do not auto-merge `main`. Each agent writes a final report and a local commit. Agent runs terminate when their single assigned task completes; they are **not** unlimited overnight workers. Claude uses auto-mode approval classification (availability depends on installed Claude Code and account); Codex uses workspace-write with no prompts and blocked privilege escalation. See [AGENTS.md](AGENTS.md) and [docs/agent-boundaries.md](docs/agent-boundaries.md).

## First-time setup on Fedora 45 — Desktop apps

1. Unzip this starter and open a terminal in `~/Projects/crownwind-game/`.
2. Run `bash scripts/desktop-setup.sh` to initialize per-user Rust and **two separate Git worktrees**. This uses no Claude/Codex CLI and does not start either AI app.
3. Optionally publish the public personal GitHub repo when prompted (`gh` authentication may be needed).
4. Open `~/Projects/crownwind-claude` in **Claude Code Desktop** and paste `prompts/CLAUDE_FIRST_SHIFT.md`.
5. Open `~/Projects/crownwind-codex` in **Codex Desktop** and paste `prompts/CODEX_FIRST_SHIFT.md`.

See [Desktop quickstart](docs/desktop-quickstart.md) for exact commands and review steps. The old CLI helpers are left as optional tools, but are not used by Desktop setup.

**Never run both Desktop agents on the same worktree.** Do not grant blanket system permissions. Desktop tasks may stop due to approval requests, quotas, sleep or connectivity, and cannot be guaranteed to work unattended overnight.

### Run the empty starter app

```
cargo run -p game_app
cargo test -p game_core -p asset_pipeline
```

## Art direction
Original high-detail pixel art, initial virtual viewport **480x270** (16:9), integer scaling where appropriate, world design not fixed by viewport size. The prince has medium-length hair and a flowing cloak, the blonde princess has layered hair and elegant costume details. Authoring specifications will be revised after the first visual prototype. See [docs/game-design.md](docs/game-design.md).

## Licenses
Rust source is under MIT (see `LICENSE`). The empty `assets/` and `content/` directories carry no third-party materials by default. Every added external asset requires attribution, permission and a compatible license recorded in `assets/ASSET_PROVENANCE.md`.
