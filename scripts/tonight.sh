#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
printf '%s\n' 'Crownwind Desktop workflow: this script NO LONGER launches Claude/Codex CLIs.'
printf '%s\n' 'It prepares Rust and worktrees. Then open each folder in its Desktop app.'
exec bash "$ROOT/scripts/desktop-setup.sh"
