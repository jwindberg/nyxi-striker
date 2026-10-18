#!/bin/bash
# Package NYXI's official app with our iot_driver into an installable bundle for this machine:
#   macOS -> release/NYXI-Driver-mac-<arch>.dmg       (open it, drag "NYXI Driver" to Applications)
#   Linux -> release/NYXI-Driver-linux-<arch>.tar.gz  (unpack it, run ./install.sh)
# Each platform must be packaged on that platform, since the driver is compiled natively.
# The bundle contains NYXI's code: personal use only, never publish it.
# Needs Rust, Node.js/npm and the app in extracted_asar/ (run ./setup-official-app.sh first).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd -P)"
APP="$ROOT/extracted_asar"
OUT="$ROOT/release"
DRIVER="$ROOT/nyxi-driver/target/release/iot_driver"
ELECTRON_VERSION=22.3.27   # the version NYXI's app was built against
# The app only loads its bundled pages (app.isPackaged) when the executable is not named "electron".
NAME="NYXI Driver"

[ -d "$APP/dist" ] || { echo "NYXI app missing; run ./setup-official-app.sh first" >&2; exit 1; }
case "$(uname -m)" in
    x86_64) ARCH=x64 ;;
    arm64|aarch64) ARCH=arm64 ;;
    *) echo "unsupported architecture $(uname -m)" >&2; exit 1 ;;
esac
if [ "$(uname)" = Darwin ]; then PLATFORM=darwin; else PLATFORM=linux; fi

# Local workaround for Linux machines without libudev-dev installed.
LIBUDEV="$ROOT/libudev-local/usr"
if [ -d "$LIBUDEV" ]; then
    export CFLAGS="-I$LIBUDEV/include"
    export PKG_CONFIG_PATH="$LIBUDEV/lib/x86_64-linux-gnu/pkgconfig"
    export RUSTFLAGS="-L $LIBUDEV/lib/x86_64-linux-gnu"
fi
(cd "$ROOT/nyxi-driver" && cargo build --release --bin iot_driver)

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# NYXI's app files, plus our driver at the app root: on macOS the app copies <appPath>/iot_driver
# into its data folder on first launch and runs it from there.
rsync -a --exclude .git --exclude README.md "$APP/" "$WORK/app/"
cp "$DRIVER" "$WORK/app/iot_driver"

ICON_ARGS=()
if [ "$PLATFORM" = darwin ]; then
    ICONSET="$WORK/icon.iconset"
    mkdir "$ICONSET"
    for s in 16 32 128 256 512; do
        sips -z $s $s "$APP/company/company_NYXI/icon.png" --out "$ICONSET/icon_${s}x${s}.png" >/dev/null
        sips -z $((s * 2)) $((s * 2)) "$APP/company/company_NYXI/icon.png" --out "$ICONSET/icon_${s}x${s}@2x.png" >/dev/null
    done
    iconutil -c icns "$ICONSET" -o "$WORK/icon.icns"
    ICON_ARGS=(--icon "$WORK/icon.icns")
fi

# --no-prune: the app's node_modules is already its production set (Electron itself excluded).
npx --yes @electron/packager@18 "$WORK/app" "$NAME" \
    --platform="$PLATFORM" --arch="$ARCH" --electron-version="$ELECTRON_VERSION" \
    --out "$WORK/pkg" --no-prune --overwrite ${ICON_ARGS[@]+"${ICON_ARGS[@]}"}
BUILT="$WORK/pkg/$NAME-$PLATFORM-$ARCH"
mkdir -p "$OUT"
# The packaged app loads its pages (and upgrades its driver) from resources/app.asar.unpacked/, as
# NYXI's installer lays it out; without it the hidden window never loads and so never shows.
if [ "$PLATFORM" = darwin ]; then RES="$BUILT/$NAME.app/Contents/Resources"; else RES="$BUILT/resources"; fi
ln -s app "$RES/app.asar.unpacked"

if [ "$PLATFORM" = darwin ]; then
    # Re-seal the bundle after adding files (ad-hoc; enough for a locally installed app).
    codesign --force --deep --sign - "$BUILT/$NAME.app"
    mkdir "$WORK/dmg"
    mv "$BUILT/$NAME.app" "$WORK/dmg/"
    ln -s /Applications "$WORK/dmg/Applications"
    DMG="$OUT/NYXI-Driver-mac-$ARCH.dmg"
    hdiutil create -volname "$NAME" -srcfolder "$WORK/dmg" -ov -format UDZO "$DMG" >/dev/null
    echo "Built $DMG"
    exit 0
fi

# Linux: the app never starts a driver itself, so a launcher starts ours alongside it.
cp "$APP/company/company_NYXI/icon.png" "$BUILT/icon.png"
cat > "$BUILT/nyxi-app" <<'EOF'
#!/bin/bash
# Start the NYXI driver (unless one is already running) and the app; stop the driver on exit.
DIR="$(cd "$(dirname "$(readlink -f "$0")")" && pwd)"
driver_up() { (exec 3<>/dev/tcp/127.0.0.1/3814) 2>/dev/null; }
driver_pid=""
app_pid=""
cleanup() {
    [ -n "$app_pid" ] && kill "$app_pid" 2>/dev/null
    [ -n "$driver_pid" ] && kill "$driver_pid" 2>/dev/null
}
trap cleanup EXIT
trap 'exit 130' INT TERM HUP
if ! driver_up; then
    (cd /tmp && exec "$DIR/resources/app/iot_driver") >/dev/null 2>&1 &
    driver_pid=$!
    for _ in $(seq 50); do
        driver_up && break
        sleep 0.1
    done
fi
"$DIR/NYXI Driver" --no-sandbox "$@" &
app_pid=$!
wait "$app_pid"
EOF
cat > "$BUILT/install.sh" <<'EOF'
#!/bin/bash
# Install the NYXI app for this user: files in ~/.local/share/nyxi-driver, a menu entry,
# and the command nyxi-app. Rerun to upgrade.
set -euo pipefail
SRC="$(cd "$(dirname "$0")" && pwd)"
DEST="$HOME/.local/share/nyxi-driver"
rm -rf "$DEST"
mkdir -p "$DEST" "$HOME/.local/bin" "$HOME/.local/share/applications"
cp -R "$SRC/." "$DEST/"
rm -f "$DEST/install.sh"
ln -sf "$DEST/nyxi-app" "$HOME/.local/bin/nyxi-app"
cat > "$HOME/.local/share/applications/nyxi-driver.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=NYXI Driver
Comment=Configure the NYXI CA01 Striker keyboard
Exec="$DEST/nyxi-app"
Icon=$DEST/icon.png
Categories=Utility;Settings;
DESKTOP
echo "Installed. Start \"NYXI Driver\" from your app menu, or run: nyxi-app"
if [ ! -e /etc/udev/rules.d/99-nyxi.rules ]; then
    echo
    echo "One-time step so the app can reach the keyboard without root:"
    echo "  echo 'SUBSYSTEM==\"hidraw\", ATTRS{idVendor}==\"3151\", MODE=\"0666\"' | sudo tee /etc/udev/rules.d/99-nyxi.rules"
    echo "  sudo udevadm control --reload && sudo udevadm trigger"
fi
EOF
chmod +x "$BUILT/nyxi-app" "$BUILT/install.sh"
mv "$BUILT" "$WORK/nyxi-driver-linux-$ARCH"
TGZ="$OUT/NYXI-Driver-linux-$ARCH.tar.gz"
tar -C "$WORK" -czf "$TGZ" "nyxi-driver-linux-$ARCH"
echo "Built $TGZ"
