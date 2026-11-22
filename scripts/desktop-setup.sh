#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

printf '%s\n' '== Crownwind: Fedora + Desktop agents setup ==' 
printf '%s\n' 'This script does NOT launch Claude or Codex CLIs and will NOT start AI agents.'

# Fedora's dnf rustup package provides rustup-init, not an initialized rustup toolchain.
export PATH="$HOME/.cargo/bin:$PATH"
if ! command -v rustup >/dev/null 2>&1; then
  if command -v rustup-init >/dev/null 2>&1; then
    printf '%s\n' '[1/4] Initializing per-user Rust toolchain from Fedora rustup-init...'
    rustup-init -y --profile minimal --component rustfmt,clippy
  else
    printf '%s\n' 'Rust bootstrap missing. On Fedora run: sudo dnf install rustup' >&2
    exit 1
  fi
fi
if [[ -f "$HOME/.cargo/env" ]]; then
  # shellcheck source=/dev/null
  source "$HOME/.cargo/env"
fi
rustup toolchain install stable --profile minimal --component rustfmt --component clippy
rustup default stable
printf '[2/4] Rust ready: '; cargo --version

# Fetch ahead of the Desktop agents, but do not prevent local Git setup if network is unavailable.
printf '%s\n' '[3/4] Fetching Rust workspace dependencies (may require network)...'
if ! cargo fetch; then
  printf '%s\n' 'WARNING: cargo fetch failed. Continue setup; agents must report any remaining build blockers.' >&2
fi

printf '%s\n' '[4/4] Preparing local Git repo and isolated Desktop worktrees...'
if ! git var GIT_AUTHOR_IDENT >/dev/null 2>&1; then
  printf '%s\n' 'Git author identity is not configured. Set it, then rerun:' >&2
  printf '%s\n' '  git config --global user.name "Your Name"' >&2
  printf '%s\n' '  git config --global user.email "your-github-email@example.com"' >&2
  exit 1
fi
bash "$ROOT/scripts/init-local-repo.sh"

create_worktree() {
  local dir="$1" branch="$2" main_root
  if [[ -e "$dir" ]]; then
    if ! git -C "$dir" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
      printf 'ERROR: %s already exists and is not a Git worktree. No files changed there.\n' "$dir" >&2
      return 1
    fi
    main_root="$(git -C "$dir" rev-parse --path-format=absolute --git-common-dir)"
    if [[ "$main_root" != "$(git rev-parse --path-format=absolute --git-common-dir)" ]]; then
      printf 'ERROR: %s is not part of this repository.\n' "$dir" >&2
      return 1
    fi
    if [[ "$(git -C "$dir" branch --show-current)" != "$branch" ]]; then
      printf 'ERROR: %s is using unexpected branch. Inspect manually.\n' "$dir" >&2
      return 1
    fi
    printf '  Already ready: %s\n' "$dir"
  elif git show-ref --verify --quiet "refs/heads/$branch"; then
    git worktree add "$dir" "$branch"
  else
    git worktree add -b "$branch" "$dir" main
  fi
}
create_worktree "$(dirname "$ROOT")/crownwind-claude" agent/claude-gameplay
create_worktree "$(dirname "$ROOT")/crownwind-codex" agent/codex-assets

printf '\n%s\n' 'Local Desktop worktrees ready:'
git worktree list

if command -v gh >/dev/null 2>&1; then
  printf '\n%s\n' 'GitHub publication is optional and PUBLIC. No organization repo will be created.'
  read -r -p 'Publish personal minitheguitarist/crownwind-game now? [y/N] ' RESPONSE
  if [[ "$RESPONSE" =~ ^[Yy]$ ]]; then
    if ! gh auth status >/dev/null 2>&1; then
      echo 'GitHub CLI login required (unrelated to Claude/Codex Desktop):'
      gh auth login
    fi
    if ! bash "$ROOT/scripts/publish-github.sh"; then
      echo 'WARNING: GitHub publish failed; local worktrees are still usable.' >&2
      echo 'Later: bash scripts/publish-github.sh' >&2
    fi
  else
    echo 'Skipped remote publishing. Later: bash scripts/publish-github.sh'
  fi
else
  echo 'GitHub CLI not found; remote publishing skipped.'
fi

cat <<TXT

READY FOR DESKTOP APPS (open these as separate projects):
  Claude Code Desktop -> $(dirname "$ROOT")/crownwind-claude
  Codex Desktop       -> $(dirname "$ROOT")/crownwind-codex

Paste prompt contents from the corresponding project folder:
  Claude: prompts/CLAUDE_FIRST_SHIFT.md
  Codex:  prompts/CODEX_FIRST_SHIFT.md

Do NOT open both agents in the same project directory.
Agents are not running yet: start each task explicitly in its Desktop app.
TXT
