#!/bin/bash
# Builds "NYXI Striker.app" from this checkout with Tauri and puts it in
# /Applications, for this Mac's own processor.
#
#   tools/make_mac_app.sh            # into /Applications
#   tools/make_mac_app.sh ~/Applications
#
# Needs Rust and Node.js/npm. Written for the bash 3.2 that comes with macOS.
set -euo pipefail

if [ "$(uname -s)" != "Darwin" ]; then
    echo "make_mac_app.sh makes a Mac app, so it runs on a Mac." >&2
    exit 1
fi
for tool in cargo npm; do
    command -v "$tool" >/dev/null || { echo "$tool not found; install Rust (rustup) and Node.js first" >&2; exit 1; }
done

root="$(cd "$(dirname "$0")/.." && pwd -P)"
dest="${1:-/Applications}"

cd "$root/app"
npm ci --no-audit --no-fund
npx tauri build --bundles app

mkdir -p "$dest"
# NYXI.app is the name older versions installed under.
rm -rf "$dest/NYXI.app" "$dest/NYXI Striker.app"
ditto "$root/target/release/bundle/macos/NYXI Striker.app" "$dest/NYXI Striker.app"
# So Finder and Launchpad notice the new version and its icon.
touch "$dest/NYXI Striker.app"

echo "NYXI Striker is in $dest/NYXI Striker.app"
