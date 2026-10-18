#!/bin/bash
# Prepare NYXI's official app to run on this machine (macOS or Linux).
# The app is NYXI's code, so it is not in this repo. This installs a matching Electron for this
# platform into electron-local/ and puts NYXI's app inside it, so it runs as an installed app.
# The app comes from one of:
#   - the private repo jwindberg/nyxi-official-app (default), cloned or updated into extracted_asar/
#   - NYXI's Windows installer: its resources/app.asar and app.asar.unpacked/ are copied in as their
#     installer lays them out, and a readable copy is unpacked into extracted_asar/ for reference.
# Nothing from the installer is executed. Then run ./nyxi-app.sh.
#
# Usage: ./setup-official-app.sh [path/to/NYXI_SETUP.exe]
# Needs: Node.js/npm; with an installer also 7-Zip (macOS: brew install sevenzip, Linux: sudo apt install 7zip).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd -P)"
INSTALLER="${1:-}"
APP_REPO=git@github.com:jwindberg/nyxi-official-app.git
ELECTRON_VERSION=22.3.27   # the version NYXI's app was built against
DIST="$ROOT/electron-local/node_modules/electron/dist"

command -v npm >/dev/null || { echo "npm not found; install Node.js first" >&2; exit 1; }
if [ -n "$INSTALLER" ]; then
    SEVENZIP="$(command -v 7zz || command -v 7z || command -v 7za || true)"
    [ -n "$SEVENZIP" ] || { echo "7-Zip not found (macOS: brew install sevenzip, Linux: sudo apt install 7zip)" >&2; exit 1; }
fi

npm install --prefix "$ROOT/electron-local" --no-audit --no-fund "electron@$ELECTRON_VERSION"

if [ "$(uname)" = Darwin ]; then
    RESOURCES="$DIST/Electron.app/Contents/Resources"
    ELECTRON="$DIST/Electron.app/Contents/MacOS/Electron"
else
    RESOURCES="$DIST/resources"
    ELECTRON="$DIST/electron"
fi
rm -rf "$RESOURCES/app.asar" "$RESOURCES/app.asar.unpacked" "$RESOURCES/app"

if [ -n "$INSTALLER" ]; then
    WORK="$(mktemp -d)"
    trap 'rm -rf "$WORK"' EXIT
    # The NSIS installer holds the packaged app as app-64.7z and/or app-32.7z.
    "$SEVENZIP" x -y -o"$WORK/installer" "$INSTALLER" >/dev/null
    PKG="$(find "$WORK/installer" -name 'app-*.7z' | sort -r | head -1)"
    [ -n "$PKG" ] || { echo "no app-*.7z inside $INSTALLER; is it the NYXI installer?" >&2; exit 1; }
    "$SEVENZIP" x -y -o"$WORK/app" "$PKG" >/dev/null
    ASAR="$(find "$WORK/app" -name app.asar | head -1)"

    cp -R "$ASAR" "$ASAR.unpacked" "$RESOURCES/"

    if [ ! -e "$ROOT/extracted_asar" ]; then
        # asar also pulls in the app.asar.unpacked/ files that sit beside it.
        npx --yes @electron/asar extract "$ASAR" "$ROOT/extracted_asar"
    fi
else
    if [ -d "$ROOT/extracted_asar/.git" ]; then
        git -C "$ROOT/extracted_asar" pull --ff-only
    elif [ -e "$ROOT/extracted_asar" ]; then
        echo "extracted_asar/ exists but is not a clone of $APP_REPO; move it aside and rerun" >&2
        exit 1
    else
        git clone "$APP_REPO" "$ROOT/extracted_asar"
    fi
    # Electron runs resources/app/ the same way as resources/app.asar. The app loads its pages from
    # resources/app.asar.unpacked/, so that points at the same files.
    ln -s "$ROOT/extracted_asar" "$RESOURCES/app"
    ln -s app "$RESOURCES/app.asar.unpacked"
fi

# The app only loads its bundled pages when Electron reports app.isPackaged, which is false
# whenever the executable is named "electron"; otherwise it tries NYXI's dev server on
# localhost:8082. A copy under the app's own name runs it as an installed app.
cp "$ELECTRON" "$(dirname "$ELECTRON")/NYXI Driver"

echo "Done. Start the app with ./nyxi-app.sh"
