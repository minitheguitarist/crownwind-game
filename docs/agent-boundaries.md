# Ownership and integration policy

| Owner | Writable directories | Read-only or forbidden areas |
| --- | --- | --- |
| Claude | `crates/game_core/**`, `crates/game_bevy/**`, `crates/game_app/**`, `docs/reports/claude-first-shift.md` | `crates/asset_pipeline/**`, `assets/**`, `content/**`, root config, contracts, workflows |
| Codex | `crates/asset_pipeline/**`, `assets/**`, `content/**`, `docs/reports/codex-first-shift.md` | `crates/game_core/**`, `crates/game_bevy/**`, `crates/game_app/**`, root config, contracts, workflows |
| Human/integrator | Everything, after review | No auto-merge from agents |

A Git worktree isolates working files and Git branches but does NOT imply API compatibility. In the first two parallel shifts, Claude **must not depend on** Codex's unmerged assets/content. Claude uses procedural temporary level geometry. Codex creates data/contracts tests which can be integrated in a later shift.

Shared contracts are immutable during concurrent work: `docs/contracts/animation-manifest.schema.json`, `docs/contracts/level.schema.json`.

Actions requiring human confirmation: changing root Cargo workspace, installing system packages with sudo, downloading models/assets, changing license, accessing secrets, force-pushing, pushing the default branch, merging pull requests.

After each shift:
1. Both agents finish local commits and reports.
2. Review `git diff main...agent/claude-gameplay` and `git diff main...agent/codex-assets`.
3. Integrate sequentially; run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and `cargo check -p game_app` (when dependencies are available).
4. Resolve content/API mismatches in an integration branch managed by the human, never in parallel agents.
