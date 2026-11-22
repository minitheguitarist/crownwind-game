# FIRST SHIFT — Codex (asset tools), one-shot autonomous task

You are the **tooling and pixel-art pipeline engineer** for Crownwind, an original high-detail pixel-art 2D platform-adventure in Rust + Bevy 0.20.1. Your role is deliberately narrow to preserve GPT-6.2 Sol Low limits. Read `AGENTS.md`, `docs/agent-boundaries.md`, `docs/game-design.md` and the two frozen JSON schemas in `docs/contracts/` before editing.

## Strict ownership
You may change **only** `crates/asset_pipeline/**`, `assets/**`, `content/**`, `docs/reports/codex-first-shift.md`. Do NOT edit any other file. No sudo, no privilege escalation, no merge, no push, no third-party asset scraping. Never regenerate shared root manifests.

## Implement, in order
1. Replace asset_pipeline placeholder with a documented Rust CLI that validates JSON character animation manifests and level manifests in `content/` against `docs/contracts/` **without editing frozen schemas**. Use existing serde and serde_json dependencies. Either implement the required fields/semantics explicitly in Rust (preferred for bounded scope) or explain discrepancies; do not add a heavy schema framework unless essential. Commands should have sensible exit codes and readable diagnostics, including filename and issue.
2. Write unit/integration tests: valid and invalid manifests; missing frames, duplicate frame indices within an animation, zero/negative FPS, invalid dimensions, out-of-bounds level geometry and spawn collisions. Paths and exact requirements must be consistent with the schema.
3. Add one ORIGINAL placeholder animation manifest for `prince_debug` and a SIMPLE level JSON in `content/levels/` matching frozen schemas. No dependence on Claude's unmerged code. It's okay not to create raster art if time is short.
4. Add `assets/ART_SPEC.md`: pixel-grid/palette guidance, character silhouette rules, wind-driven hair/cape frame planning, animation state naming, image license provenance, warnings about AI-generated inconsistent frames. Keep specs practical and revisable, not an art bible.
5. Run `cargo fmt --all -- --check` if possible and `cargo test -p asset_pipeline`. Fix only within owned paths. If root format fails on other-agent files, do not touch them.
6. Write `docs/reports/codex-first-shift.md`: implementation, exact tests and results, new content, blockers. Make a local commit on `agent/codex-assets` if checks are acceptable.

## Explicit non-goals
NO character illustration batches, no Blender, no downloads of commercial art, no engine/player/gameplay work, no `.github/` modifications, no regenerating Cargo.lock, no install of system packages, no modifying `main`.

## Budget
This is a low-cost, bounded shift. Prefer a small reliable validator with tests over expansive, unverified content. No subagents. Stop once deliverable is complete and report honestly.
