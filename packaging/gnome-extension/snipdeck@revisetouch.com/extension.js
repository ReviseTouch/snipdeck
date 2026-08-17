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
 * with x/y in physical screen pixels. A title with no coordinates is still
 * recognised and raised — only the move is skipped. Deliberately not D-Bus: a
 * title is readable the instant the window appears, needs no service to be
 * running, no name to be owned, and no ordering between app and extension
 * start-up.
 */

import * as AltTab from 'resource:///org/gnome/shell/ui/altTab.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

/** WM_CLASS of every Snipdeck window. */
const WM_CLASS = 'snipdeck';

/** Title prefix that marks a floating snip, as opposed to the gallery window. */
const SNIP_TITLE = 'SnipdeckFloating';

export default class SnipdeckExtension extends Extension {
    enable() {
        this._handles = [];
        this._pending = new Set();

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
    }

    _onWindowCreated(window) {
        // The title is often still empty at window-created; Snipdeck sets it
        // immediately afterwards. Watch for it rather than sampling once and
        // giving up.
        if (this._adopt(window))
            return;

        if (!this._isOurs(window))
            return;

        const id = window.connect('notify::title', () => {
            if (this._adopt(window)) {
                window.disconnect(id);
                this._pending.delete(window);
            }
        });
        this._pending.add(window);

        // Stop listening if the window goes away before it is ever titled.
        const unmanaged = window.connect('unmanaged', () => {
            try {
                window.disconnect(id);
            } catch (_) {}
            window.disconnect(unmanaged);
            this._pending.delete(window);
        });
    }

    _isOurs(window) {
        const wmClass = window.get_wm_class();
        return !!wmClass && wmClass.toLowerCase() === WM_CLASS;
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
            window.move_frame(true, at.x, at.y);
        }

        return true;
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
