/*
 * Snipdeck GNOME Shell extension.
 *
 * Snipdeck's whole point is that a capture stays on screen, where it was taken,
 * above whatever you are working in. On Wayland an application cannot do that
 * for itself: it may not place its own windows, may not ask to be raised above
 * others, and is not even told where it currently is. Those are compositor
 * decisions, and GNOME offers no protocol to influence them — no wlr-layer-shell,
 * no equivalent.
 *
 * Code running *inside* GNOME Shell has no such limits. This extension is the
 * smallest possible piece of that: it watches for Snipdeck's snip windows and
 * applies the two things the app cannot.
 *
 * The app is not required to have this installed. Without it Snipdeck behaves
 * like any other Wayland client — snips open wherever the compositor decides and
 * sit in the ordinary stacking order. With it, they behave as designed.
 *
 * How placement is communicated
 * -----------------------------
 * Through the window title, which for these windows is never user-visible (they
 * are borderless). Snipdeck sets it to:
 *
 *     SnipdeckFloating <x>,<y>
 *
 * with x/y in physical screen pixels — the space the app captures in. Mutter
 * places windows in logical coordinates, so `_toLogical` converts before the
 * move. A title with no coordinates is still recognised and raised — only the
 * move is skipped. Deliberately not D-Bus: a title is readable the instant the
 * window appears, needs no service to be running, no name to be owned, and no
 * ordering between app and extension start-up.
 */

import * as AltTab from 'resource:///org/gnome/shell/ui/altTab.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

/** WM_CLASS of every Snipdeck window. */
const WM_CLASS = 'snipdeck';

/** Title prefix that marks a floating snip, as opposed to the gallery window. */
const SNIP_TITLE = 'SnipdeckFloating';

export default class SnipdeckExtension extends Extension {
    enable() {
        // One line at start-up, so a session can be asked which build of
        // this file it is actually running. GNOME Shell caches the loaded
        // module, and on Wayland it cannot re-import one without a new
        // session — without this marker there is no way to tell a fix that
        // did not work from a fix that was never loaded.
        log('snipdeck: enable() match=title placement=first-frame coords=logical');

        this._handles = [];
        this._pending = new Set();
        this._placing = new Set();

        this._handles.push([
            global.display,
            global.display.connect('window-created', (_display, window) => {
                this._onWindowCreated(window);
            }),
        ]);

        // Anything already on screen when the extension is enabled — after a
        // login, the app usually starts first.
        for (const actor of global.get_window_actors())
            this._adopt(actor.meta_window);

        this._patchWindowSwitcher();
    }

    /**
     * Keep snips out of the window switcher.
     *
     * Mutter has no `set_skip_taskbar` — only `is_skip_taskbar`, a read-only
     * query derived from the window type — so the flag cannot be set by the
     * application OR by an extension. What can be done is filter the list the
     * switcher builds, which lives in GNOME Shell's own JavaScript.
     *
     * Only the *window* switcher (Alt+`, and Alt+Tab's per-app expansion) is
     * touched. The application switcher still shows Snipdeck once, which is
     * correct: the gallery is a real window the user wants to reach. It is the
     * dozen pinned snips that should not each be a stop on the way there.
     */
    _patchWindowSwitcher() {
        const proto = AltTab.WindowSwitcherPopup?.prototype;
        if (!proto?._getWindowList) {
            // A future Shell may rename or restructure this. Losing the filter
            // is a cosmetic regression; breaking the switcher is not, so leave
            // it alone rather than guess.
            log('snipdeck: window switcher not patchable on this Shell version');
            return;
        }

        const isSnip = w => this._isSnipWindow(w);
        this._origGetWindowList = proto._getWindowList;
        const orig = this._origGetWindowList;
        proto._getWindowList = function () {
            return orig.call(this).filter(w => !isSnip(w));
        };
    }

    _isSnipWindow(window) {
        try {
            return (
                this._isOurs(window) &&
                (window.get_title() ?? '').startsWith(SNIP_TITLE)
            );
        } catch (_) {
            return false;
        }
    }

    disable() {
        for (const [obj, id] of this._handles ?? [])
            obj.disconnect(id);
        this._handles = null;

        // Restore the switcher unconditionally. A monkey patch left behind
        // after disable is how an extension breaks a desktop it is no longer
        // even running in.
        if (this._origGetWindowList) {
            AltTab.WindowSwitcherPopup.prototype._getWindowList =
                this._origGetWindowList;
            this._origGetWindowList = null;
        }

        // Windows keep whatever we applied; undoing it on disable would drop
        // every snip behind the focused window, which reads as the app breaking
        // rather than the extension stopping.
        this._pending?.clear();
        this._pending = null;
        this._placing?.clear();
        this._placing = null;
    }

    _onWindowCreated(window) {
        // The title is often still empty at window-created; Snipdeck sets it
        // immediately afterwards. Watch for it rather than sampling once and
        // giving up.
        if (this._adopt(window))
            return;

        // Deliberately NOT gated on `_isOurs` here. At `window-created` a snip
        // is usually untitled, and `wm_class` cannot be relied on to identify
        // it in the meantime, so there is nothing trustworthy to filter on yet.
        // Watching every new window until it is titled costs one signal per
        // window and lets `_adopt` be the only thing that decides.

        // `titleWatch` is the one source of truth for whether that handler is
        // still connected. Disconnecting an id twice is not an exception in
        // GJS — it is a GLib warning written straight to the journal, which no
        // `try` can catch — so it has to be tracked rather than attempted and
        // forgiven.
        let titleWatch = window.connect('notify::title', () => {
            if (this._adopt(window)) {
                window.disconnect(titleWatch);
                titleWatch = 0;
                this._pending?.delete(window);
            }
        });
        this._pending.add(window);

        // Stop listening if the window goes away before it is ever titled.
        // These per-window handlers outlive `disable()`, which is why the sets
        // they touch are reached optionally.
        const unmanaged = window.connect('unmanaged', () => {
            if (titleWatch) {
                window.disconnect(titleWatch);
                titleWatch = 0;
            }
            window.disconnect(unmanaged);
            this._pending?.delete(window);
        });
    }

    /**
     * Is this one of ours?
     *
     * The TITLE is the marker, not `wm_class`. `meta_window_get_wm_class()`
     * reports the X11 WM_CLASS property, and a Wayland toplevel need not have
     * one — GNOME Shell matches applications through `ShellWindowTracker`, not
     * this. Requiring it was a gate that could never open on Wayland, which is
     * why nothing was ever adopted: not the placement, not `make_above`, not
     * the switcher filter.
     *
     * `SnipdeckFloating` is specific enough to stand on its own, and the app
     * only ever puts it on windows it wants treated this way. `wm_class` is
     * still accepted when present, so the X11 path keeps a second signal.
     */
    _isOurs(window) {
        try {
            if ((window.get_title() ?? '').startsWith(SNIP_TITLE))
                return true;
            const wmClass = window.get_wm_class();
            return !!wmClass && wmClass.toLowerCase() === WM_CLASS;
        } catch (_) {
            return false;
        }
    }

    /**
     * Apply the treatment if this is a titled snip window.
     * Returns true once handled, so callers can stop waiting.
     */
    _adopt(window) {
        if (!window || !this._isOurs(window))
            return false;

        const title = window.get_title();
        if (!title || !title.startsWith(SNIP_TITLE))
            return false;

        log(`snipdeck: adopting ${JSON.stringify(title)}`);

        // Once now, because a window adopted from `enable()` is already placed
        // and this is all it needs...
        this._apply(window, title);

        // ...and once more after Mutter has had its say. Everything above runs
        // from `window-created` or `notify::title`, both of which fire BEFORE
        // the compositor places the window. With `center-new-windows` on — the
        // GNOME default — Mutter then centres it and the move is simply undone,
        // which is why a snip landed dead centre rather than merely off by a
        // bit. `first-frame` is the first moment the window is mapped, placed
        // and painted, so a move made there is the one that survives.
        //
        // Same trap as the sizing fix in 0153733: on this compositor, saying it
        // early is not saying it at all.
        this._applyWhenPlaced(window);

        return true;
    }

    /** Above, on every workspace, and where it was captured. */
    _apply(window, title) {
        // Above everything. The single reason this extension exists.
        window.make_above();

        // Snips are reference material for whatever you are doing, so they
        // belong on every workspace rather than the one they happened to be
        // captured on.
        window.stick();

        const at = this._parsePosition(title);
        if (at) {
            // `move_frame` places the frame rather than the client area, which
            // is what the app's coordinates describe — these windows are
            // borderless, so the two coincide, but asking for the frame is
            // correct if that ever changes.
            const to = this._toLogical(at.x, at.y);
            window.move_frame(true, to.x, to.y);
        }
    }

    /**
     * Re-apply once the compositor has finished placing the window.
     *
     * `first-frame` fires exactly once per actor, so this cannot turn into a
     * fight with the user: a snip dragged later stays where it was dropped.
     * A window that has already painted by the time we get here — anything
     * adopted during `enable()` — never fires it again, and does not need to,
     * because the immediate pass in `_adopt` already landed on a placed window.
     */
    _applyWhenPlaced(window) {
        const actor = window.get_compositor_private();
        if (!actor || this._placing?.has(window))
            return;

        this._placing?.add(window);
        const id = actor.connect('first-frame', () => {
            actor.disconnect(id);
            this._placing?.delete(window);

            const title = window.get_title();
            if (title && title.startsWith(SNIP_TITLE))
                this._apply(window, title);
        });
    }

    /**
     * Physical screen pixels -> Mutter's logical coordinates.
     *
     * The title carries physical pixels because that is the space the app
     * captures in. Mutter lays windows out in logical space, and on a scaled
     * display the two differ by exactly the scale factor: a 3200x2000 panel at
     * 1.667 is a 1920x1200 desktop. Handing physical pixels straight to
     * `move_frame` put every snip 1.667x too far right and down, and pushed
     * anything captured past logical width off the screen entirely, where
     * Mutter clamped it to the edge.
     *
     * The monitor is found by trying each one's scale and keeping the first
     * result that lands inside that monitor's logical geometry — exact for a
     * single display, and for several that share a scale. Monitors with
     * *different* scales are not solvable from here: the physical origin the
     * app uses for the second monitor comes from its own capture stack and is
     * not something Mutter exposes. That case falls back to the primary
     * monitor's scale and may still be wrong.
     */
    _toLogical(x, y) {
        const display = global.display;

        for (let i = 0; i < display.get_n_monitors(); i++) {
            const scale = display.get_monitor_scale(i);
            const lx = Math.round(x / scale);
            const ly = Math.round(y / scale);
            const g = display.get_monitor_geometry(i);

            if (lx >= g.x && lx < g.x + g.width &&
                ly >= g.y && ly < g.y + g.height)
                return {x: lx, y: ly};
        }

        const scale = display.get_monitor_scale(display.get_primary_monitor());
        return {x: Math.round(x / scale), y: Math.round(y / scale)};
    }

    /** "SnipdeckFloating 1234,567" -> {x: 1234, y: 567}; null if absent. */
    _parsePosition(title) {
        const m = /(-?\d+)\s*,\s*(-?\d+)\s*$/.exec(title);
        if (!m)
            return null;
        const x = Number.parseInt(m[1], 10);
        const y = Number.parseInt(m[2], 10);
        return Number.isFinite(x) && Number.isFinite(y) ? {x, y} : null;
    }
}
