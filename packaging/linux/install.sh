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
EXT_UUID="snipdeck@revisetouch.com"
EXT_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/gnome-shell/extensions/$EXT_UUID"
SRC="$(cd "$(dirname "$0")" && pwd)"

# Is this a GNOME session? Only there is the shell extension worth installing —
# every other desktop either grants the window control outright (X11) or offers
# wlr-layer-shell (KDE, wlroots), and would just be left with dead files.
is_gnome() {
    case ":${XDG_CURRENT_DESKTOP:-}:" in
    *:GNOME:* | *GNOME-*) return 0 ;;
    esac
    [ -n "${GNOME_SHELL_SESSION_MODE:-}" ]
}

if [ "${1:-}" = "--uninstall" ]; then
    rm -f "$BIN_DIR/snipdeck" "$APP_DIR/snipdeck.desktop" "$ICON_DIR/snipdeck.png"
    command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APP_DIR" || true
    if [ -d "$EXT_DIR" ]; then
        command -v gnome-extensions >/dev/null 2>&1 && gnome-extensions disable "$EXT_UUID" 2>/dev/null || true
        rm -rf "$EXT_DIR"
        echo "GNOME Shell extension removed (log out and back in to unload it)."
    fi
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

# The GNOME Shell extension. Not cosmetic on a GNOME Wayland session: placing a
# snip where it was captured and keeping it above other windows are compositor
# decisions there, and Mutter offers no protocol for a client to ask. Code
# inside the shell can do both, so that is where those two live. Snipdeck runs
# without it — the snips just land wherever Mutter puts them.
EXT_INSTALLED=no
if [ -d "$SRC/$EXT_UUID" ] && is_gnome; then
    mkdir -p "$EXT_DIR"
    install -m 644 "$SRC/$EXT_UUID/extension.js" "$SRC/$EXT_UUID/metadata.json" "$EXT_DIR/"
    # Enabling only writes a gsetting, so it works on a shell that has not
    # loaded the extension yet — it takes effect on the next shell start.
    command -v gnome-extensions >/dev/null 2>&1 && gnome-extensions enable "$EXT_UUID" 2>/dev/null || true
    EXT_INSTALLED=yes
fi

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

if [ "$EXT_INSTALLED" = yes ]; then
    echo "  * GNOME Shell extension installed and enabled ($EXT_UUID)."
    case "${XDG_SESSION_TYPE:-}" in
    wayland)
        # No way around this: GNOME Shell cannot be restarted in place on
        # Wayland, because it *is* the compositor holding every client's session.
        echo "    Log out and back in to load it — until then snips will not be"
        echo "    placed or kept on top."
        ;;
    *)
        echo "    Press Alt+F2, type 'r' and hit Enter to load it now."
        ;;
    esac
elif [ -d "$SRC/$EXT_UUID" ]; then
    echo "  * GNOME Shell extension NOT installed: this is not a GNOME session,"
    echo "    and it is only needed there."
fi

if [ -z "${XDG_DATA_HOME:-}" ] && ! echo "$PATH" | grep -q "$BIN_DIR"; then
    echo "  * $BIN_DIR is not on your PATH; the menu entry works regardless."
fi

cat <<'EOF'

Optional runtime dependencies:
  tray icon           libayatana-appindicator  (plus the AppIndicator
                      extension on GNOME, which has no built-in tray)
  "Share" actions     xdg-utils  (xdg-email / xdg-open)
  OCR                 tesseract + the language data you want
                      (absent, OCR is skipped rather than failing)
EOF
