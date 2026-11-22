#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  TOP="$(git rev-parse --show-toplevel)"
  [[ "$TOP" == "$ROOT" ]] || { echo "This starter is inside a different repository ($TOP). Move it to a standalone folder." >&2; exit 1; }
  echo "Already inside a Git repo: $TOP"
else
  git init -b main
  git add .
  git commit -m "chore: initial Crownwind Rust workspace and agent contracts"
fi
if [[ "$(git branch --show-current)" != main ]]; then
  echo 'Expected current branch main; stop to inspect.' >&2
  exit 1
fi
if [[ -f Cargo.lock && -n "$(git status --porcelain -- Cargo.lock)" ]]; then
  git add Cargo.lock
  git commit -m "build: lock Rust dependencies"
fi
printf '%s\n' 'Local repo ready. Next: bash scripts/publish-github.sh (optional), then launch-agents.sh'
