#!/bin/sh
# Install Snipdeck for the current user.
#
# Everything goes under ~/.local, so no root and nothing outside your home
# directory. Run ./install.sh --uninstall to undo it.
#
# The .desktop entry is not decoration. On Wayland, global shortcuts go through
# xdg-desktop-portal, which since 1.21 refuses a session from a client with no
# application id, and the id it accepts is this entry's name. See the note at
# the end of com.revisetouch.Snipdeck.desktop.

set -eu

BIN_DIR="${XDG_BIN_HOME:-$HOME/.local/bin}"
APP_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
ICON_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/256x256/apps"
EXT_UUID="snipdeck@revisetouch.com"
EXT_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/gnome-shell/extensions/$EXT_UUID"
KWIN_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/kwin/scripts/snipdeck"
SRC="$(cd "$(dirname "$0")" && pwd)"
DESKTOP=com.revisetouch.Snipdeck.desktop
# The names this entry had before: removed on install and uninstall, or a
# menu would list Snipdeck twice and a login would start it twice.
OLD_DESKTOP=snipdeck.desktop

is_kde() {
    case ":${XDG_CURRENT_DESKTOP:-}:" in
    *:KDE:*) return 0 ;;
    esac
    return 1
}

# KWin loads scripts when told to reconfigure; no need to log out.
kwin_reconfigure() {
    dbus-send --session --type=method_call --dest=org.kde.KWin /KWin org.kde.KWin.reconfigure \
        >/dev/null 2>&1 || true
}

kwin_enable() {
    for tool in kwriteconfig6 kwriteconfig5; do
        if command -v "$tool" >/dev/null 2>&1; then
            "$tool" --file kwinrc --group Plugins --key snipdeckEnabled "$1"
            kwin_reconfigure
            return 0
        fi
    done
    return 1
}

# Is this a GNOME session? Only there is the shell extension worth installing,
# as the KWin script only on KDE; anywhere else they would be dead files.
is_gnome() {
    case ":${XDG_CURRENT_DESKTOP:-}:" in
    *:GNOME:* | *GNOME-*) return 0 ;;
    esac
    [ -n "${GNOME_SHELL_SESSION_MODE:-}" ]
}

if [ "${1:-}" = "--uninstall" ]; then
    # The login autostart entry too, if "Launch at login" was on: left behind,
    # it ran a binary that no longer exists at every login.
    rm -f "$BIN_DIR/snipdeck" "$APP_DIR/$DESKTOP" "$APP_DIR/$OLD_DESKTOP" "$ICON_DIR/snipdeck.png" \
        "${XDG_CONFIG_HOME:-$HOME/.config}/autostart/$DESKTOP" \
        "${XDG_CONFIG_HOME:-$HOME/.config}/autostart/$OLD_DESKTOP"
    command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APP_DIR" || true
    if [ -d "$EXT_DIR" ]; then
        command -v gnome-extensions >/dev/null 2>&1 && gnome-extensions disable "$EXT_UUID" 2>/dev/null || true
        rm -rf "$EXT_DIR"
        echo "GNOME Shell extension removed (log out and back in to unload it)."
    fi
    if [ -d "$KWIN_DIR" ]; then
        rm -rf "$KWIN_DIR"
        kwin_reconfigure
        echo "KWin script removed."
    fi
    echo "Snipdeck removed. Your snips in ~/.local/share/Snipdeck were left alone."
    exit 0
fi

mkdir -p "$BIN_DIR" "$APP_DIR" "$ICON_DIR"

install -m 755 "$SRC/snipdeck" "$BIN_DIR/snipdeck"
[ -f "$SRC/snipdeck.png" ] && install -m 644 "$SRC/snipdeck.png" "$ICON_DIR/snipdeck.png"

# Point Exec at the installed binary. The *filename* is the application id,
# so it keeps the name it has here.
rm -f "$APP_DIR/$OLD_DESKTOP"
sed "s|^Exec=.*|Exec=$BIN_DIR/snipdeck|" "$SRC/$DESKTOP" > "$APP_DIR/$DESKTOP"
chmod 644 "$APP_DIR/$DESKTOP"

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

# The KWin script: the same job on KDE Plasma. A Wayland client cannot keep
# a window above the others there either; a KWin script can, and loads at once.
KWIN_INSTALLED=no
if [ -d "$SRC/kwin-script/snipdeck" ] && is_kde; then
    mkdir -p "$KWIN_DIR/contents/code"
    install -m 644 "$SRC/kwin-script/snipdeck/metadata.json" "$KWIN_DIR/"
    install -m 644 "$SRC/kwin-script/snipdeck/contents/code/main.js" "$KWIN_DIR/contents/code/"
    kwin_enable true && KWIN_INSTALLED=yes
fi

echo "Snipdeck installed to $BIN_DIR/snipdeck"
echo

case "${XDG_SESSION_TYPE:-}" in
wayland)
    echo "Wayland session detected."
    echo "  * Launch Snipdeck from the applications menu. From a terminal, global"
    echo "    shortcuts need xdg-desktop-portal 1.19 or later."
    echo "  * The first shortcut binding shows a system dialog; approve it once."
    ;;
esac

if [ "$EXT_INSTALLED" = yes ]; then
    echo "  * GNOME Shell extension installed and enabled ($EXT_UUID)."
    echo "    With Snipdeck running, hold Super+Alt and drag to snip a region"
    echo "    straight from the screen."
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
elif [ "$KWIN_INSTALLED" = yes ]; then
    echo "  * KWin script installed and enabled: snips stay above other windows."
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
