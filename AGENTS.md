# Agent contract (applies to Codex and Claude)

1. Read `README.md`, `docs/agent-boundaries.md`, `docs/game-design.md`, `docs/contracts/*`, and **your own** `prompts/*.md` before editing.
2. Use the existing Rust workspace and Bevy `=0.20.1`; **do not upgrade the engine**.
3. Only change your OWNED paths listed in `docs/agent-boundaries.md`. Shared files are read-only, including this file, root `Cargo.toml`, `rust-toolchain.toml`, `docs/contracts/`, and `.github/`.
4. No destructive commands (`rm -rf`, hard reset, clean -fd), no rewriting other people's commits, no force push, no deleting user data, no reading secrets, no handling credentials.
5. No sudo, system package install, global tool modification, or external downloads without explicit human authorization. If dependencies are missing, report exact commands in your final report; continue with tasks that do not need them.
6. Do not auto-merge, do not push `main`, do not create PRs or issues. Commit your changes on your own branch after checks; otherwise leave changes and explain.
7. Do not copy intellectual property from Forgotten Warrior or Ori. Do not download art blindly. Temporary art must be original and identified as placeholder.
8. Prioritize a bounded, small, tested delivery rather than a speculative full game or elaborate refactor. Stay within the agreed task.
9. Keep gameplay frame-rate independent, avoid animation or input logic depending on render FPS, and prefer deterministic pure-Rust rules in `game_core`.
10. At the end, provide changed paths, test commands/results, commit SHA (if any), known limitations, and next integration steps. Write that report to your **owned** report file.
