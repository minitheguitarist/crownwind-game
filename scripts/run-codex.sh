#!/usr/bin/env bash
set -uo pipefail
WORKTREE="$1"; LOG="$2"
cd "$WORKTREE" || exit 1
{
  echo "=== Codex start: $(date -Is) ==="
  echo "Branch: $(git branch --show-current)"
  timeout --foreground --signal=TERM --kill-after=30s 90m codex exec --model gpt-6.2-sol -c model_reasoning_effort=low --sandbox workspace-write --ask-for-approval never "$(cat prompts/CODEX_FIRST_SHIFT.md)"
  CODE=$?
  echo "=== Codex exit code $CODE: $(date -Is) ==="
  echo "=== Current git status ==="
  git status --short
  echo "=== Current commit ==="
  git log -1 --oneline
  echo "Session finished; you can inspect logs."
} 2>&1 | tee "$LOG"
