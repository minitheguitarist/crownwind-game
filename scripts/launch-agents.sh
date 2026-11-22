#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
for x in claude codex tmux cargo; do
  command -v "$x" >/dev/null || { echo "Missing '$x'. Check README and bootstrap, then retry." >&2; exit 1; }
done
git rev-parse --is-inside-work-tree >/dev/null || { echo 'Initialize repo first.' >&2; exit 1; }
[[ "$(git branch --show-current)" == main ]] || { echo 'Launch from main worktree only.' >&2; exit 1; }
[[ -z "$(git status --porcelain)" ]] || { echo 'Commit or stash local changes before starting parallel agents.' >&2; exit 1; }
CLAUDE_WORKTREE="$(dirname "$ROOT")/crownwind-claude"
CODEX_WORKTREE="$(dirname "$ROOT")/crownwind-codex"
if [[ -e "$CLAUDE_WORKTREE" || -e "$CODEX_WORKTREE" ]]; then
  echo "Worktree paths already exist. Refusing to overwrite them. Inspect 'git worktree list'." >&2
  exit 1
fi
git show-ref --verify --quiet refs/heads/agent/claude-gameplay && { echo 'Claude branch already exists; refusing duplicate run.' >&2; exit 1; }
git show-ref --verify --quiet refs/heads/agent/codex-assets && { echo 'Codex branch already exists; refusing duplicate run.' >&2; exit 1; }
if tmux has-session -t crownwind-claude 2>/dev/null || tmux has-session -t crownwind-codex 2>/dev/null; then
  echo 'A crownwind session already exists.' >&2; exit 1
fi
mkdir -p "$ROOT/logs"
git worktree add -b agent/claude-gameplay "$CLAUDE_WORKTREE" main
git worktree add -b agent/codex-assets "$CODEX_WORKTREE" main
# Each tmux session starts a bounded, one-shot task. It is not a persistent background daemon.
tmux new-session -d -s crownwind-claude "bash '$ROOT/scripts/run-claude.sh' '$CLAUDE_WORKTREE' '$ROOT/logs/claude.log'"
tmux new-session -d -s crownwind-codex "bash '$ROOT/scripts/run-codex.sh' '$CODEX_WORKTREE' '$ROOT/logs/codex.log'"
cat <<EOF
Launched two independent tmux sessions:
  crownwind-claude -> $CLAUDE_WORKTREE
  crownwind-codex  -> $CODEX_WORKTREE

Inspect logs: bash scripts/status-agents.sh
Watch live: tmux attach -t crownwind-claude   (detach: Ctrl+B, D)
Watch live: tmux attach -t crownwind-codex    (detach: Ctrl+B, D)
Stop one: tmux kill-session -t crownwind-claude

Do not allow the Fedora laptop to suspend. Network/auth and model quota must remain available.
EOF
