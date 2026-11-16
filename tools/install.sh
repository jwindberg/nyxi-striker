#!/bin/bash
# Build NYXI Striker and install it for this user, into ~/.cargo/bin as
# nyxi-striker. Rerun to upgrade. Works from any folder: tools/install.sh,
# ../tools/install.sh, and so on. For the app menu, run
# tools/install_desktop.sh next; on a Mac, tools/make_mac_app.sh makes the app.
# First it offers to install what building needs: Rust, and on Debian-family
# Linux (Raspberry Pi OS, Ubuntu, Mint) the packages below. Node.js must be
# 20.19 or later.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd -P)"
PACKAGES=(build-essential pkg-config curl libwebkit2gtk-4.1-dev libdbus-1-dev)
# Node.js may come from elsewhere than apt (NodeSource, nvm); only ask apt
# for it when there's none.
command -v node >/dev/null || PACKAGES+=(nodejs npm)

# What this app needs before it builds, and offers to install what's
# missing: Rust (with rustup) everywhere, and on Debian-family Linux
# (Raspberry Pi OS, Ubuntu, Mint) the system packages in PACKAGES, with apt.
ask() {
    # Yes unless answered no; no when there's no one to ask.
    [ -t 0 ] || return 1
    printf '%s [Y/n] ' "$1"
    read -r answer
    case "$answer" in [Nn]*) return 1 ;; *) return 0 ;; esac
}

if [ "$(uname -s)" = Linux ] && command -v dpkg-query >/dev/null && command -v apt-get >/dev/null; then
    missing=()
    for pkg in "${PACKAGES[@]}"; do
        dpkg-query -W -f='${Status}' "$pkg" 2>/dev/null | grep -q "install ok installed" || missing+=("$pkg")
    done
    if [ ${#missing[@]} -gt 0 ]; then
        echo "Building needs these packages, which aren't installed: ${missing[*]}"
        if ask "Install them now (with sudo apt)?"; then
            sudo apt-get update
            sudo apt-get install -y "${missing[@]}"
        else
            echo "Install them with: sudo apt install ${missing[*]}" >&2
            exit 1
        fi
    fi
fi

# On a Mac, building needs Apple's command line tools (the linker).
if [ "$(uname -s)" = Darwin ] && ! xcode-select -p >/dev/null 2>&1; then
    echo "Building needs Apple's command line tools; macOS will offer to install them." >&2
    xcode-select --install || true
    echo "Run tools/install.sh again once they're installed." >&2
    exit 1
fi

# rustup puts cargo in ~/.cargo/bin, which a new shell may not have on its PATH yet.
cargo_bin="${CARGO_HOME:-$HOME/.cargo}/bin"
if ! command -v cargo >/dev/null && [ -x "$cargo_bin/cargo" ]; then
    export PATH="$cargo_bin:$PATH"
fi
if ! command -v cargo >/dev/null; then
    echo "Rust isn't installed."
    if ask "Install it now with rustup (from https://rustup.rs)?"; then
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        export PATH="$cargo_bin:$PATH"
    else
        echo "Install Rust from https://rustup.rs first." >&2
        exit 1
    fi
fi

# The front end's build (Vite) needs Node.js 20.19 or later.
if ! command -v node >/dev/null || ! command -v npm >/dev/null; then
    echo "Node.js and npm aren't installed; on Debian-family Linux they're the nodejs and npm packages." >&2
    exit 1
fi
node_version=$(node -p 'process.versions.node')
if ! node -e 'const [a, b] = process.versions.node.split(".").map(Number); process.exit(a > 20 || (a === 20 && b >= 19) ? 0 : 1)'; then
    echo "Node.js $node_version is too old; building needs 20.19 or later (https://nodejs.org)." >&2
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
