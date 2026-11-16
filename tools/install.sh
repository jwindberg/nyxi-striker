#!/bin/bash
# Build NYXI Striker and install it for this user, into ~/.cargo/bin as
# nyxi-striker. Rerun to upgrade. Works from any folder: tools/install.sh,
# ../tools/install.sh, and so on. For the app menu, run
# tools/install_desktop.sh next; on a Mac, tools/make_mac_app.sh makes the app.
# Needs Rust and Node.js/npm; on Linux also WebKitGTK
# (sudo apt install libwebkit2gtk-4.1-dev build-essential).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd -P)"

for tool in cargo npm; do
    command -v "$tool" >/dev/null || { echo "$tool not found; install Rust (rustup) and Node.js first" >&2; exit 1; }
done
if [ "$(uname)" != Darwin ] && ! pkg-config --exists webkit2gtk-4.1 2>/dev/null; then
    echo "WebKitGTK not found; run: sudo apt install libwebkit2gtk-4.1-dev build-essential" >&2
    exit 1
fi

# Tauri builds the pages into the program, so the program is all there is to install.
cd "$ROOT/app"
npm ci --no-audit --no-fund
npx tauri build --no-bundle

bin="${CARGO_HOME:-$HOME/.cargo}/bin/nyxi-striker"
mkdir -p "$(dirname "$bin")"
install -m 755 "$ROOT/target/release/nyxi-app" "$bin"
# Earlier versions installed into ~/.local/bin, which may come first on the PATH.
rm -f "$HOME/.local/bin/nyxi-striker"
echo "Installed $bin"

if [ "$(uname)" != Darwin ] && [ ! -e /etc/udev/rules.d/99-nyxi.rules ]; then
    echo
    echo "One-time step so the app can reach the keyboard without root:"
    echo "  echo 'SUBSYSTEM==\"hidraw\", ATTRS{idVendor}==\"3151\", MODE=\"0666\"' | sudo tee /etc/udev/rules.d/99-nyxi.rules"
    echo "  sudo udevadm control --reload && sudo udevadm trigger"
fi
