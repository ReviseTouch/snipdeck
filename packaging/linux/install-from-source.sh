#!/bin/sh
# Build Snipdeck from this checkout and install it the way a release installs:
# into ~/.local, no root, with the desktop entry, the icon and the GNOME Shell
# extension. Run from anywhere; --uninstall is passed on to install.sh.
#
# Every file is staged afresh. Copying only the new binary into an old staging
# folder installed the old extension beside it — one that predated GNOME 51 —
# and the shell disabled it at the next login, taking Super+Alt+drag with it.
#
# CARGO picks the build command, e.g. CARGO="derle cargo" to queue the build.

set -eu

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
STAGE="$ROOT/dist/snipdeck"
EXT_UUID="snipdeck@revisetouch.com"

if [ "${1:-}" = "--uninstall" ]; then
    exec "$ROOT/packaging/linux/install.sh" --uninstall
fi

${CARGO:-cargo} build --release --manifest-path "$ROOT/Cargo.toml"

rm -rf "$STAGE"
mkdir -p "$STAGE"
cp "$ROOT/target/release/snipdeck" "$ROOT/packaging/linux/com.revisetouch.Snipdeck.desktop" \
   "$ROOT/packaging/linux/install.sh" "$STAGE/"
cp "$ROOT/assets/preview_256.png" "$STAGE/snipdeck.png"
cp -r "$ROOT/packaging/gnome-extension/$EXT_UUID" "$STAGE/"
cp -r "$ROOT/packaging/kwin-script" "$STAGE/kwin-script"

exec "$STAGE/install.sh" "$@"
