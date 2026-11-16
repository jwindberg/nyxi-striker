#!/bin/bash
# Build the app (app/) and install it for this user on macOS or Linux, in one
# step. Rerun to upgrade.
#   macOS -> "/Applications/NYXI Striker.app"   (tools/make_mac_app.sh)
#   Linux -> nyxi-striker in ~/.cargo/bin, plus a menu entry "NYXI Striker"
#            (tools/install.sh, then tools/install_desktop.sh)
# On Windows use install-app.ps1.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd -P)"
if [ "$(uname)" = Darwin ]; then
    exec "$ROOT/tools/make_mac_app.sh"
fi
"$ROOT/tools/install.sh"
"$ROOT/tools/install_desktop.sh"
echo "Start \"NYXI Striker\" from your app menu, or run: nyxi-striker"
