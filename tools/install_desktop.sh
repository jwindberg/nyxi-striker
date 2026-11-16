#!/bin/bash
# Puts NYXI Striker in the desktop's app menu, for this user: its menu entry
# and icons, pointing at the program tools/install.sh installed. For Linux
# and other systems with freedesktop menus; rerun it after installing a new
# version. On a Mac, use tools/make_mac_app.sh.
set -euo pipefail

if [ "$(uname -s)" = "Darwin" ]; then
    echo "On a Mac, use tools/make_mac_app.sh instead." >&2
    exit 1
fi

root="$(cd "$(dirname "$0")/.." && pwd -P)"
id=com.jwindberg.nyxi
# The menu may not search ~/.cargo/bin, so the entry names the program's path.
bin=$(command -v nyxi-striker || true)
if [ -z "$bin" ] && [ -x "${CARGO_HOME:-$HOME/.cargo}/bin/nyxi-striker" ]; then
    bin="${CARGO_HOME:-$HOME/.cargo}/bin/nyxi-striker"
fi
if [ -z "$bin" ]; then
    echo "nyxi-striker isn't installed yet; run tools/install.sh first." >&2
    exit 1
fi

data="${XDG_DATA_HOME:-$HOME/.local/share}"
install -D -m 644 "$root/app/src-tauri/icons/128x128.png" "$data/icons/hicolor/128x128/apps/$id.png"
install -D -m 644 "$root/app/src-tauri/icons/128x128@2x.png" "$data/icons/hicolor/256x256/apps/$id.png"
mkdir -p "$data/applications"
cat > "$data/applications/$id.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=NYXI Striker
Comment=Lighting, screen and settings for the NYXI Striker keyboard
Exec=$bin
Icon=$id
Terminal=false
StartupWMClass=nyxi-striker
Categories=Utility;
DESKTOP

# So menus notice the new entry and icon right away.
if command -v update-desktop-database >/dev/null; then
    update-desktop-database "$data/applications" || true
fi
if command -v gtk-update-icon-cache >/dev/null; then
    gtk-update-icon-cache -q -t "$data/icons/hicolor" || true
fi
echo "NYXI Striker is in the app menu, running $bin."
