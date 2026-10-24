#!/bin/bash
# Build the new app (app/) and install it for this user on macOS or Linux. Rerun to upgrade.
#   macOS -> "/Applications/NYXI Striker.app"
#   Linux -> ~/.local/bin/nyxi-striker, plus a menu entry "NYXI Striker"
# On Windows use install-app.ps1.
# Needs Rust and Node.js/npm; on Linux also WebKitGTK (sudo apt install libwebkit2gtk-4.1-dev build-essential).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd -P)"
ID=com.jwindberg.nyxi

for tool in cargo npm; do
    command -v "$tool" >/dev/null || { echo "$tool not found; install Rust (rustup) and Node.js first" >&2; exit 1; }
done
if [ "$(uname)" != Darwin ] && ! pkg-config --exists webkit2gtk-4.1 2>/dev/null; then
    echo "WebKitGTK not found; run: sudo apt install libwebkit2gtk-4.1-dev build-essential" >&2
    exit 1
fi

cd "$ROOT/app"
npm ci --no-audit --no-fund

if [ "$(uname)" = Darwin ]; then
    npx tauri build --bundles app
    # NYXI.app is the name older versions installed under.
    rm -rf /Applications/NYXI.app "/Applications/NYXI Striker.app"
    cp -R "$ROOT/target/release/bundle/macos/NYXI Striker.app" /Applications/
    echo "Installed \"/Applications/NYXI Striker.app\""
    exit 0
fi

# Linux: the release binary has the pages built in, so it is all we need to install.
npx tauri build --no-bundle
BIN="$HOME/.local/bin/nyxi-striker"
ICONS="$HOME/.local/share/icons/hicolor"
mkdir -p "$HOME/.local/bin" "$HOME/.local/share/applications" "$ICONS/128x128/apps" "$ICONS/256x256/apps"
install -m 755 "$ROOT/target/release/nyxi-app" "$BIN"
install -m 644 "$ROOT/app/src-tauri/icons/128x128.png" "$ICONS/128x128/apps/$ID.png"
install -m 644 "$ROOT/app/src-tauri/icons/128x128@2x.png" "$ICONS/256x256/apps/$ID.png"
cat > "$HOME/.local/share/applications/$ID.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=NYXI Striker
Comment=Lighting, screen and settings for the NYXI Striker keyboard
Exec=$BIN
Icon=$ID
Terminal=false
StartupWMClass=nyxi-striker
Categories=Utility;
EOF
echo "Installed. Start \"NYXI Striker\" from your app menu, or run: nyxi-striker"
if [ ! -e /etc/udev/rules.d/99-nyxi.rules ]; then
    echo
    echo "One-time step so the app can reach the keyboard without root:"
    echo "  echo 'SUBSYSTEM==\"hidraw\", ATTRS{idVendor}==\"3151\", MODE=\"0666\"' | sudo tee /etc/udev/rules.d/99-nyxi.rules"
    echo "  sudo udevadm control --reload && sudo udevadm trigger"
fi
