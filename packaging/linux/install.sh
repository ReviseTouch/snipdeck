#!/bin/sh
# Install Snipdeck for the current user.
#
# Everything goes under ~/.local, so no root and nothing outside your home
# directory. Run ./install.sh --uninstall to undo it.
#
# The .desktop entry is not decoration. On Wayland, global shortcuts go through
# xdg-desktop-portal, which since 1.21 refuses a session from a client with no
# application id — and an unsandboxed app only gets one when the desktop
# launched it from a .desktop entry (the portal reads it out of the systemd
# scope name). Start the binary straight from a terminal and every shortcut
# silently fails to bind. So: install this, then launch Snipdeck from your
# applications menu.

set -eu

BIN_DIR="${XDG_BIN_HOME:-$HOME/.local/bin}"
APP_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
ICON_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/256x256/apps"
SRC="$(cd "$(dirname "$0")" && pwd)"

if [ "${1:-}" = "--uninstall" ]; then
    rm -f "$BIN_DIR/snipdeck" "$APP_DIR/snipdeck.desktop" "$ICON_DIR/snipdeck.png"
    command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APP_DIR" || true
    echo "Snipdeck removed. Your snips in ~/.local/share/Snipdeck were left alone."
    exit 0
fi

mkdir -p "$BIN_DIR" "$APP_DIR" "$ICON_DIR"

install -m 755 "$SRC/snipdeck" "$BIN_DIR/snipdeck"
[ -f "$SRC/snipdeck.png" ] && install -m 644 "$SRC/snipdeck.png" "$ICON_DIR/snipdeck.png"

# Point Exec at the installed binary. The *filename* is what the portal turns
# into the application id, so it has to stay snipdeck.desktop.
sed "s|^Exec=.*|Exec=$BIN_DIR/snipdeck|" "$SRC/snipdeck.desktop" > "$APP_DIR/snipdeck.desktop"
chmod 644 "$APP_DIR/snipdeck.desktop"

command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APP_DIR" || true

echo "Snipdeck installed to $BIN_DIR/snipdeck"
echo

case "${XDG_SESSION_TYPE:-}" in
wayland)
    echo "Wayland session detected."
    echo "  * Launch Snipdeck from the applications menu, NOT from a terminal —"
    echo "    global shortcuts will not bind otherwise."
    echo "  * The first shortcut binding shows a system dialog; approve it once."
    ;;
esac

if [ -z "${XDG_DATA_HOME:-}" ] && ! echo "$PATH" | grep -q "$BIN_DIR"; then
    echo "  * $BIN_DIR is not on your PATH; the menu entry works regardless."
fi

cat <<'EOF'

Optional runtime dependencies:
  tray icon           libayatana-appindicator  (plus the AppIndicator
                      extension on GNOME, which has no built-in tray)
  "Share" actions     xdotool
  OCR                 tesseract + the language data you want
                      (absent, OCR is skipped rather than failing)
EOF
