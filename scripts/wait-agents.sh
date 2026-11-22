#!/usr/bin/env bash
set -euo pipefail
# Keep the inhibitor alive only while at least one bounded agent session is running.
while tmux has-session -t crownwind-claude 2>/dev/null || tmux has-session -t crownwind-codex 2>/dev/null; do
  sleep 20
done
