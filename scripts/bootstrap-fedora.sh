#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ ! -f /etc/fedora-release ]]; then
  echo "This bootstrap targets Fedora (Fedora 45 recommended)." >&2
  exit 1
fi
printf '%s\n' '[1/4] Installing Fedora development packages (one sudo authorization)...'
sudo dnf install -y git gh tmux gcc gcc-c++ pkgconf-pkg-config libX11-devel alsa-lib-devel systemd-devel wayland-devel libxkbcommon-devel rustup
printf '%s\n' '[2/4] Installing stable Rust and components for current user...'
# Fedora's rustup RPM installs rustup-init; rustup itself appears only after initialization.
export PATH="$HOME/.cargo/bin:$PATH"
if ! command -v rustup >/dev/null 2>&1; then
  rustup-init -y --profile minimal --component rustfmt,clippy
fi
if [[ -f "$HOME/.cargo/env" ]]; then
  # shellcheck source=/dev/null
  source "$HOME/.cargo/env"
fi
rustup toolchain install stable --profile minimal --component rustfmt --component clippy
rustup default stable
printf '%s\n' '[3/4] Checking Rust and fetching predeclared dependencies...'
cargo --version
(cd "$ROOT" && cargo fetch)
printf '%s\n' '[4/4] Checking Git tools (Desktop AI apps do not require local agent CLIs)...'
for c in gh git; do
  if command -v "$c" >/dev/null; then
    printf '  OK: %s -> %s\n' "$c" "$(command -v "$c")"
  else
    printf '  MISSING: %s (install/sign in manually before launch)\n' "$c"
  fi
done
printf '\nDone. Next: bash scripts/desktop-setup.sh\n'
