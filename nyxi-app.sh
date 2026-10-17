#!/bin/bash
# Launch the official NYXI (Electron) app on macOS or Linux with our iot_driver replacement.
# The app talks to the driver over gRPC-web on 127.0.0.1:3814 and never starts a driver itself,
# so we start it here (building it first if needed) and stop it when the app exits.
# First-time setup: ./setup-official-app.sh path/to/NYXI_SETUP.exe
set -u
ROOT="$(cd "$(dirname "$0")" && pwd -P)"
DRIVER="$ROOT/nyxi-driver/target/release/iot_driver"
APP="$ROOT/extracted_asar"
if [ "$(uname)" = Darwin ]; then
    ELECTRON="$ROOT/electron-local/node_modules/electron/dist/Electron.app/Contents/MacOS/Electron"
else
    ELECTRON="$ROOT/electron-local/node_modules/electron/dist/electron"
fi

if [ ! -d "$APP" ] || [ ! -x "$ELECTRON" ]; then
    echo "NYXI app not set up; run: ./setup-official-app.sh path/to/NYXI_SETUP.exe" >&2
    exit 1
fi

driver_up() { (exec 3<>/dev/tcp/127.0.0.1/3814) 2>/dev/null; }

if [ ! -x "$DRIVER" ]; then
    echo "Building iot_driver (first run only)..."
    # Local workaround for Linux machines without libudev-dev installed.
    LIBUDEV="$ROOT/libudev-local/usr"
    if [ -d "$LIBUDEV" ]; then
        export CFLAGS="-I$LIBUDEV/include"
        export PKG_CONFIG_PATH="$LIBUDEV/lib/x86_64-linux-gnu/pkgconfig"
        export RUSTFLAGS="-L $LIBUDEV/lib/x86_64-linux-gnu"
    fi
    (cd "$ROOT/nyxi-driver" && cargo build --release --bin iot_driver) || exit 1
fi

started_driver=""
app_pid=""
cleanup() {
    [ -n "$app_pid" ] && kill "$app_pid" 2>/dev/null
    [ -n "$started_driver" ] && kill "$started_driver" 2>/dev/null
}
trap cleanup EXIT
trap 'exit 130' INT TERM HUP

if ! driver_up; then
    (cd "$ROOT/nyxi-driver" && exec "$DRIVER") >/dev/null 2>&1 &
    started_driver=$!
    for _ in $(seq 50); do
        driver_up && break
        sleep 0.1
    done
fi

# On macOS, closing the window leaves the app running; quit it (Cmd+Q) to stop the driver too.
cd "$APP" || exit 1
"$ELECTRON" --no-sandbox . "$@" &
app_pid=$!
wait "$app_pid"
