# Claude Code Desktop + Codex Desktop: quick start

This project uses **two separate local Git worktrees**. The apps do not need to be started from the terminal. CLI login/launch commands for Claude/Codex are **not required**.

## Fedora onboarding

The RPM `rustup` package provides `rustup-init` on Fedora. To initialize and load Rust manually:

```bash
rustup-init -y --profile minimal --component rustfmt,clippy
source "$HOME/.cargo/env"
rustc --version
cargo --version
```

From the extracted `~/Projects/crownwind-game` folder, run:

```bash
bash scripts/desktop-setup.sh
```

It installs or verifies a **per-user** Rust toolchain, attempts `cargo fetch`, initializes a local Git repo and the two worktrees, and optionally offers to publish `minitheguitarist/crownwind-game` as a **public personal** repo (via `gh`, not an AI CLI). If Rust or worktrees already exist, the script tries to reuse them rather than overwrite anything. It does not launch AI agents or enable unrestricted approvals.

## Launch in the desktop GUIs

- Claude Code Desktop: **Open Project/Folder** `~/Projects/crownwind-claude`, select **Opus 5.5 / Medium** if available in your app; paste **all contents** of `prompts/CLAUDE_FIRST_SHIFT.md` into the new task.
- Codex Desktop: **Open Project/Folder** `~/Projects/crownwind-codex`, select **GPT-6.2 Sol / Lite (or closest available low reasoning mode)** if available; paste **all contents** of `prompts/CODEX_FIRST_SHIFT.md` into the new task.

Each agent should read `AGENTS.md`, task instructions and its immutable contracts, edit only owned paths, run tests when feasible, then commit **only to its own branch** and write its report. Avoid blanket system permissions. No sudo, no force push, no overwrite of files outside its own worktree. Approval prompts may still occur in Desktop apps, and any long-running work depends on app settings, quotas, power and connectivity; it is not guaranteed to run unattended overnight.

## See results

```bash
cd ~/Projects/crownwind-game
git worktree list
git log --oneline -4 agent/claude-gameplay
git log --oneline -4 agent/codex-assets
git diff --stat main...agent/claude-gameplay
git diff --stat main...agent/codex-assets
```

Check `docs/reports/claude-first-shift.md` under the Claude worktree and `docs/reports/codex-first-shift.md` under the Codex worktree. **Do not merge automatically**. The main folder is the integration workspace.

If you have already run the old `scripts/tonight.sh` and it failed at `rustup: command not found`, the DNF packages completed successfully. You do **not** need to repeat `sudo dnf install`.
