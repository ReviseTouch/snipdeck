#!/bin/bash
# One headless GNOME Shell session with Snipdeck in it, driven by a plan.
#
#   tools/ui-harness/run.sh            # replays $WORK/plan.json
#
# Runs its own session bus and its own XDG dirs under $WORK, so nothing here
# touches the desktop it is started from — not its settings, not its running
# Snipdeck. The ui driver extension (uidriver@local) reads $WORK/plan.json,
# acts, takes stage screenshots into $WORK/shots and logs every step to
# $WORK/shell.log as "ui: …". tools/ui-harness/shots.py builds on this.
#
# Environment:
#   WORK          working dir (default ~/.cache/snipdeck-ui-harness)
#   APP           the binary (default target/debug/snipdeck)
#   VMON          virtual monitor, WxH in logical pixels (default 1920x1200)
#   SCALE         monitor scale, 1 or 2 (default 1)
#   REAL_DISPLAY  X display the app reads screen geometry from; the headless
#                 shell's own XWayland reports a nonsense size
#   APP_ARGS      extra arguments for the app, e.g. --autostart
#   SEEDS         PNGs snipped into the gallery at start (gallery only)
#   FLOATS        "file.png:x:y …" — PNGs snipped as floating snips
#   WATCHER=1     fake a tray host, so the window's X hides instead of quits
#   SECOND=n      launch a second instance n seconds in (asks to show the window)
#   BG            flat desktop colour instead of the wallpaper, e.g. '#0d1116'

set -u

# Its own session bus and its own runtime dir: the desktop's Snipdeck holds
# the single-instance lock in the real one, and the copy under test would
# exit at once. The path stays short — a Wayland socket path over 108 bytes
# makes the shell abort.
if [ -z "${UI_HARNESS_INNER:-}" ]; then
  RT="/run/user/$(id -u)/sdh-$$"
  mkdir -m 700 -p "$RT"
  XDG_RUNTIME_DIR="$RT" UI_HARNESS_INNER=1 timeout "${TIMEOUT:-150}" dbus-run-session -- "$0" "$@"
  rc=$?
  # The document and gvfs portals mount FUSE filesystems in it.
  for m in "$RT/doc" "$RT/gvfs"; do fusermount3 -u "$m" 2>/dev/null || fusermount -u "$m" 2>/dev/null; done
  rm -rf "$RT"
  exit $rc
fi

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
WORK="${WORK:-${XDG_CACHE_HOME:-$HOME/.cache}/snipdeck-ui-harness}"
APP="${APP:-$ROOT/target/debug/snipdeck}"
D=snipdeck-ui-0
mkdir -p "$WORK/shots" "$WORK/data/gnome-shell/extensions" "$WORK/config/glib-2.0/settings" \
         "$WORK/appdata" "$WORK/appconfig/Snipdeck" "$WORK/data/applications"

# The extensions under test, fresh every run: the driver and Snipdeck's own.
EXT="$WORK/data/gnome-shell/extensions"
rm -rf "$EXT/uidriver@local" "$EXT/snipdeck@revisetouch.com"
cp -r "$HERE/uidriver@local" "$ROOT/packaging/gnome-extension/snipdeck@revisetouch.com" "$EXT/"
# A shell newer than the driver's metadata would disable it as out of date.
SHELL_MAJOR=$(gnome-shell --version | grep -o '[0-9]*' | head -1)
python3 - "$EXT" "$SHELL_MAJOR" <<'EOF'
import json, sys, os
ext, major = sys.argv[1], sys.argv[2]
for uuid in ("uidriver@local", "snipdeck@revisetouch.com"):
    p = os.path.join(ext, uuid, "metadata.json")
    m = json.load(open(p))
    if major not in m["shell-version"]:
        m["shell-version"].append(major)
    json.dump(m, open(p, "w"))
EOF
# The desktop entry groups Snipdeck's windows under one app, as installed.
sed "s|^Exec=.*|Exec=$APP|" "$ROOT/packaging/linux/snipdeck.desktop" > "$WORK/data/applications/snipdeck.desktop"
# BG: a flat desktop colour instead of the wallpaper, e.g. BG='#0d1116'.
KF="$WORK/config/glib-2.0/settings/keyfile"
cat > "$KF" <<EOF
[org/gnome/shell]
enabled-extensions=['uidriver@local', 'snipdeck@revisetouch.com']
disable-user-extensions=false
welcome-dialog-last-shown-version='999'

[org/gnome/desktop/interface]
enable-hot-corners=false
EOF
if [ -n "${BG:-}" ]; then
  cat >> "$KF" <<EOF

[org/gnome/desktop/background]
picture-options='none'
color-shading-type='solid'
primary-color='$BG'
EOF
fi

# SCALE=2: the monitor is VMON in logical pixels and twice that physically,
# as on a HiDPI laptop, so screenshots come out at twice the resolution and
# every coordinate a plan uses stays the same.
VMON="${VMON:-1920x1200}"
SCALE="${SCALE:-1}"
PW=$(( ${VMON%x*} * SCALE )); PH=$(( ${VMON#*x} * SCALE ))
if [ "$SCALE" != 1 ]; then
  cat > "$WORK/config/monitors.xml" <<XML
<monitors version="2">
  <configuration>
    <layoutmode>logical</layoutmode>
    <logicalmonitor>
      <x>0</x><y>0</y><scale>$SCALE</scale><primary>yes</primary>
      <monitor>
        <monitorspec><connector>Meta-0</connector><vendor>MetaVendor</vendor><product>MetaVirtualMonitor</product><serial>0x00</serial></monitorspec>
        <mode><width>$PW</width><height>$PH</height><rate>60.000</rate></mode>
      </monitor>
    </logicalmonitor>
  </configuration>
</monitors>
XML
else
  rm -f "$WORK/config/monitors.xml"
fi
XDG_DATA_HOME="$WORK/data" XDG_CONFIG_HOME="$WORK/config" GSETTINGS_BACKEND=keyfile UI_DIR="$WORK" \
  gnome-shell --headless --virtual-monitor "${PW}x${PH}" --wayland-display "$D" > "$WORK/shell.log" 2>&1 &
SH=$!
sleep 3
[ -n "${WATCHER:-}" ] && { gjs -m "$HERE/watcher.mjs" > /dev/null 2>&1 & sleep 0.5; }
HX=$(grep -o 'Using public X11 display :[0-9]*' "$WORK/shell.log" | grep -o ':[0-9]*$' | head -1)
DISPLAY="${REAL_DISPLAY:-${HX:-:99}}" XDG_DATA_HOME="$WORK/appdata" XDG_CONFIG_HOME="$WORK/appconfig" \
  XDG_CURRENT_DESKTOP=GNOME WAYLAND_DISPLAY="$D" RUST_LOG=${RUST_LOG:-info,zbus=warn,tracing=warn} \
  "$APP" ${APP_ARGS:-} > "$WORK/app.log" 2>&1 &
AP=$!
for _ in $(seq 1 40); do
  gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus \
    --method org.freedesktop.DBus.NameHasOwner com.revisetouch.Snipdeck 2>/dev/null | grep -q true && break
  sleep 0.25
done
if [ -n "${SEEDS:-}" ]; then
  for s in $SEEDS; do gjs -m "$HERE/call.mjs" "$s" 120 120 false true >> "$WORK/app.log" 2>&1; sleep 0.3; done
fi
if [ -n "${FLOATS:-}" ]; then
  for f in $FLOATS; do
    IFS=: read -r fp fx fy <<< "$f"
    gjs -m "$HERE/call.mjs" "$fp" "$fx" "$fy" false false >> "$WORK/app.log" 2>&1; sleep 0.6
  done
fi
if [ -n "${SECOND:-}" ]; then
  sleep "${SECOND}"
  DISPLAY="${REAL_DISPLAY:-${HX:-:99}}" XDG_DATA_HOME="$WORK/appdata" XDG_CONFIG_HOME="$WORK/appconfig" \
    XDG_CURRENT_DESKTOP=GNOME WAYLAND_DISPLAY="$D" timeout 10 "$APP" > "$WORK/app2.log" 2>&1
fi
for _ in $(seq 1 240); do grep -q 'ui: done\|ui: FAILED' "$WORK/shell.log" && break; sleep 0.5; done
kill $AP 2>/dev/null; kill $SH 2>/dev/null; wait 2>/dev/null
