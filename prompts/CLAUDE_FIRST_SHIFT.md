# FIRST SHIFT — Claude Code (gameplay), one-shot autonomous task

You are the **gameplay engineer** for Crownwind, an original Rust + Bevy 0.20.1 high-detail pixel-art 2D platform adventure. Your immediate mission is a **small working prototype**, not a full game. Read `AGENTS.md`, `CLAUDE.md`, `docs/agent-boundaries.md`, `docs/game-design.md`, and `docs/contracts/*`. Then work through the task without asking aesthetic questions; choose safe defaults.

## Strict ownership
You may change **only** `crates/game_core/**`, `crates/game_bevy/**`, `crates/game_app/**`, `docs/reports/claude-first-shift.md`. Do NOT edit root Cargo.toml, lockfile, assets, content, schemas, scripts, GitHub workflows or Codex-owned crates. No sudo, no privilege escalation, no merge, no push.

## Implement, in order
1. Add deterministic fixed-step **pure Rust** movement rules to `game_core`: horizontal speed, acceleration/friction, gravity, jumping, jump buffer, coyote time; player dimensions and axis-aligned solid rectangle collision. Write meaningful unit tests for landing, wall blocking, buffered jump and coyote time. Be mindful of pixel/world unit conventions. Do not import Bevy into game_core.
2. Connect the simulation to Bevy 0.20.1 in game_bevy. Show a visible placeholder hero and rectangular platforms on a modest procedural test course, spawn with camera, keyboard controls A/D or arrows, Space to jump, R to reset. Provide a simple on-screen controls hint if practical.
3. Use stable fixed-step game logic, rendering decoupled where reasonable, and a camera that follows the player without jarring shake. Prioritize working simple code over fancy architecture.
4. Ensure a Linux desktop build can be checked. If dependencies unavailable, report exact blockers and continue implementing/test pure-Rust core.
5. Run `cargo fmt --all -- --check`, `cargo test -p game_core`, and `cargo check -p game_app` when available. Fix only problems in your own files. If checks cannot run, **say so**.
6. Commit only your owned changes on branch `agent/claude-gameplay` and write the short report in `docs/reports/claude-first-shift.md` with checked results, changed files, known blockers and next steps.

## Explicit non-goals
No combat classes, no story, no princess cutscene, no sprite ripping, no animation engine, no network features, no giant speculative architecture, no refactoring root config. Do not read or rely on the other agent's as-yet-unmerged changes.

## Resource budget
The user has limited model quota; use a focused implementation, don't create subagents or repeatedly re-architect. Stop after a coherent tested vertical slice. If some goals fail, leave a transparent report rather than spinning.
