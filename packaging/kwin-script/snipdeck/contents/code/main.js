// Snipdeck's floating snips on KDE Plasma: the GNOME extension's job, for KWin.
//
// A Wayland client cannot ask to stay above other windows, to be placed at
// given coordinates, or to stay out of the taskbar; the compositor decides all
// three. On GNOME the shell extension in packaging/gnome-extension does it
// from inside the compositor; this script is the same thing for KWin, which
// lets scripts set exactly those properties on any window.
//
// The app and this script agree through the window title, the one thing a
// client can always set (see src/floating.rs):
//
//   SnipdeckFloating 1234,567          a snip, captured at physical (1234, 567)
//   SnipdeckFloating listed 1234,567   the same, left in the taskbar
//   SnipdeckContextMenu, ...           a window opened on a snip: raised only
//
// Plasma 6 names the API windowAdded/windowList; Plasma 5 called them
// clientAdded/clientList. Both are taken.

const SNIP_TITLE = "SnipdeckFloating";
const LISTED = "listed";
const ABOVE_TITLES = [
    "SnipdeckContextMenu",
    "SnipdeckColorResult",
    "SnipdeckTranslate",
    "SnipdeckUpload",
];
const CLASSES = ["com.revisetouch.snipdeck", "snipdeck"];

function isOurs(window) {
    const cls = String(window.resourceClass || "").toLowerCase();
    const app = String(window.desktopFileName || "").toLowerCase();
    return CLASSES.indexOf(cls) >= 0 || CLASSES.indexOf(app) >= 0;
}

// "SnipdeckFloating 1234,567" -> {x: 1234, y: 567}; null if absent.
function parsePosition(title) {
    const m = /(-?\d+)\s*,\s*(-?\d+)\s*$/.exec(title);
    return m ? { x: parseInt(m[1], 10), y: parseInt(m[2], 10) } : null;
}

// Physical pixels, as the app captures, to KWin's logical layout: the first
// screen whose scale puts the point inside it. Exact for one screen and for
// several that share a scale; with mixed scales the primary's is a guess, as
// in the GNOME extension.
function toLogical(x, y) {
    const screens = workspace.screens || [];
    for (let i = 0; i < screens.length; i++) {
        const s = screens[i];
        const scale = s.devicePixelRatio || 1;
        const lx = Math.round(x / scale);
        const ly = Math.round(y / scale);
        const g = s.geometry;
        if (lx >= g.x && lx < g.x + g.width && ly >= g.y && ly < g.y + g.height)
            return { x: lx, y: ly };
    }
    const scale = screens.length ? screens[0].devicePixelRatio || 1 : 1;
    return { x: Math.round(x / scale), y: Math.round(y / scale) };
}

// Off and on, so the state is applied even where it was already set.
function raise(window) {
    window.keepAbove = false;
    window.keepAbove = true;
    if (typeof workspace.raiseWindow === "function")
        workspace.raiseWindow(window);
}

// Snips are positioned once, on adoption. A snip the user then drags stays
// where it was dropped: a title change from the app carries a new position
// only when the app moved it.
function adopt(window) {
    const title = String(window.caption || "");
    if (!isOurs(window))
        return;
    if (ABOVE_TITLES.indexOf(title) >= 0) {
        raise(window);
        return;
    }
    if (title.indexOf(SNIP_TITLE) !== 0)
        return;

    const listed = title.split(/\s+/).indexOf(LISTED) >= 0;
    window.keepAbove = true;
    window.onAllDesktops = true;
    window.skipTaskbar = !listed;
    window.skipPager = !listed;
    window.skipSwitcher = !listed;

    const at = parsePosition(title);
    if (at) {
        const to = toLogical(at.x, at.y);
        const g = window.frameGeometry;
        if (g.x !== to.x || g.y !== to.y)
            window.frameGeometry = { x: to.x, y: to.y, width: g.width, height: g.height };
    }
}

function watch(window) {
    adopt(window);
    // A snip is created first and titled a moment later.
    if (window.captionChanged)
        window.captionChanged.connect(function () { adopt(window); });
}

const added = workspace.windowAdded || workspace.clientAdded;
added.connect(watch);
const existing = workspace.windowList ? workspace.windowList() : workspace.clientList();
for (let i = 0; i < existing.length; i++)
    watch(existing[i]);
