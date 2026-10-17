#!/bin/bash
# Launch the official NYXI (Electron) app on Linux with the Linux iot_driver replacement.
# The app talks to the driver over gRPC-web on 127.0.0.1:3814; on Linux the app does not
# start a driver itself, so we start it here and stop it when the app exits.
set -u
ROOT="$(cd "$(dirname "$(readlink -f "$0")")" && pwd)"
DRIVER="$ROOT/nyxi-driver/target/release/iot_driver"
ELECTRON="$ROOT/electron-linux/node_modules/electron/dist/electron"
APP="$ROOT/extracted_asar"

started_driver=""
if ! ss -ltn 2>/dev/null | grep -q '127.0.0.1:3814'; then
    (cd "$ROOT/nyxi-driver" && exec "$DRIVER") >/dev/null 2>&1 &
    started_driver=$!
    trap '[ -n "$started_driver" ] && kill "$started_driver" 2>/dev/null' EXIT
    for _ in $(seq 50); do
        ss -ltn 2>/dev/null | grep -q '127.0.0.1:3814' && break
        sleep 0.1
    done
fi

cd "$APP" && "$ELECTRON" --no-sandbox . "$@"
