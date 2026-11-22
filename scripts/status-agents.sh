#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
for s in crownwind-claude crownwind-codex; do
  if tmux has-session -t "$s" 2>/dev/null; then echo "$s : RUNNING"; else echo "$s : STOPPED/FINISHED"; fi
done
printf '\nLocal worktrees:\n'
git -C "$ROOT" worktree list || true
printf '\nRecent Claude log:\n'
if [[ -f "$ROOT/logs/claude.log" ]]; then tail -n 25 "$ROOT/logs/claude.log"; else echo '(not started)'; fi
printf '\nRecent Codex log:\n'
if [[ -f "$ROOT/logs/codex.log" ]]; then tail -n 25 "$ROOT/logs/codex.log"; else echo '(not started)'; fi
