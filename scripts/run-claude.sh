#!/usr/bin/env bash
set -uo pipefail
WORKTREE="$1"; LOG="$2"
cd "$WORKTREE" || exit 1
export CLAUDE_CODE_EFFORT_LEVEL=medium
{
  echo "=== Claude start: $(date -Is) ==="
  echo "Branch: $(git branch --show-current)"
  # Auto mode uses a safety classifier; no blanket bypass permission.
  claude --print --permission-mode auto --model opus --effort medium --max-turns 45 "$(cat prompts/CLAUDE_FIRST_SHIFT.md)"
  CODE=$?
  echo "=== Claude exit code $CODE: $(date -Is) ==="
  echo "=== Current git status ==="
  git status --short
  echo "=== Current commit ==="
  git log -1 --oneline
  echo "Session finished; you can inspect logs."
} 2>&1 | tee "$LOG"
