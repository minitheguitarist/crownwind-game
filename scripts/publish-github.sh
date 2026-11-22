#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
command -v gh >/dev/null || { echo "Missing gh CLI" >&2; exit 1; }
gh auth status >/dev/null || { echo "Run gh auth login first." >&2; exit 1; }
EXPECTED="minitheguitarist"
CURRENT="$(gh api user --jq '.login')"
[[ "$CURRENT" == "$EXPECTED" ]] || { echo "Logged in as $CURRENT, expected $EXPECTED; refusing to create org/other user repo." >&2; exit 1; }
[[ "$(git branch --show-current)" == main ]] || { echo 'Expected main branch.' >&2; exit 1; }
[[ -z "$(git status --porcelain)" ]] || { echo 'Commit local changes before publishing.' >&2; exit 1; }
if gh repo view "$EXPECTED/crownwind-game" >/dev/null 2>&1; then
  echo "Repo exists: https://github.com/$EXPECTED/crownwind-game"
  if ! git remote get-url origin >/dev/null 2>&1; then git remote add origin "https://github.com/$EXPECTED/crownwind-game.git"; fi
  git push -u origin main
else
  gh repo create "$EXPECTED/crownwind-game" --public --source "$ROOT" --remote origin --push --description "Original 2D pixel-art Rust/Bevy platform-adventure (working title)"
fi
echo "Published: https://github.com/$EXPECTED/crownwind-game"
