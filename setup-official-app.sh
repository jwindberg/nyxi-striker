#!/bin/bash
# Prepare NYXI's official app to run on this machine (macOS or Linux).
# The app is NYXI's code, so it is not in this repo. This installs a matching Electron for this
# platform into electron-local/ and puts NYXI's app files (resources/app.asar and
# app.asar.unpacked/ from their Windows installer) inside it, laid out as their installer does.
# Nothing from the installer is executed. It also unpacks a readable copy into extracted_asar/
# for reference. Then run ./nyxi-app.sh.
#
# Usage: ./setup-official-app.sh path/to/NYXI_SETUP.exe
# Needs: 7-Zip (macOS: brew install sevenzip, Linux: sudo apt install 7zip) and Node.js/npm.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd -P)"
INSTALLER="${1:?usage: $0 path/to/NYXI_SETUP.exe}"
ELECTRON_VERSION=22.3.27   # the version NYXI's app was built against
DIST="$ROOT/electron-local/node_modules/electron/dist"

SEVENZIP="$(command -v 7zz || command -v 7z || command -v 7za || true)"
[ -n "$SEVENZIP" ] || { echo "7-Zip not found (macOS: brew install sevenzip, Linux: sudo apt install 7zip)" >&2; exit 1; }
command -v npm >/dev/null || { echo "npm not found; install Node.js first" >&2; exit 1; }

npm install --prefix "$ROOT/electron-local" --no-audit --no-fund "electron@$ELECTRON_VERSION"

if [ "$(uname)" = Darwin ]; then
    RESOURCES="$DIST/Electron.app/Contents/Resources"
    ELECTRON="$DIST/Electron.app/Contents/MacOS/Electron"
else
    RESOURCES="$DIST/resources"
    ELECTRON="$DIST/electron"
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
# The NSIS installer holds the packaged app as app-64.7z and/or app-32.7z.
"$SEVENZIP" x -y -o"$WORK/installer" "$INSTALLER" >/dev/null
PKG="$(find "$WORK/installer" -name 'app-*.7z' | sort -r | head -1)"
[ -n "$PKG" ] || { echo "no app-*.7z inside $INSTALLER; is it the NYXI installer?" >&2; exit 1; }
"$SEVENZIP" x -y -o"$WORK/app" "$PKG" >/dev/null
ASAR="$(find "$WORK/app" -name app.asar | head -1)"

rm -rf "$RESOURCES/app.asar" "$RESOURCES/app.asar.unpacked"
cp -R "$ASAR" "$ASAR.unpacked" "$RESOURCES/"

# The app only loads its bundled pages when Electron reports app.isPackaged, which is false
# whenever the executable is named "electron"; otherwise it tries NYXI's dev server on
# localhost:8082. A copy under the app's own name runs it as an installed app.
cp "$ELECTRON" "$(dirname "$ELECTRON")/NYXI Driver"

if [ ! -e "$ROOT/extracted_asar" ]; then
    # asar also pulls in the app.asar.unpacked/ files that sit beside it.
    npx --yes @electron/asar extract "$ASAR" "$ROOT/extracted_asar"
fi
echo "Done. Start the app with ./nyxi-app.sh"
