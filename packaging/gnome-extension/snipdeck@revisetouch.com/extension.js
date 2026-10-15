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
 *
 * Super+Alt+drag
 * --------------
 * The one thing that does go over D-Bus, because it runs the other way: the
 * extension starting a snip. On Windows a snip is Win+drag, read by a global
 * mouse hook. Wayland has no global hook, and on GNOME Super+drag is Mutter's
 * window move besides — so `SnipGesture` takes Super+Alt+drag from inside the
 * shell, captures the region itself and hands the pixels to the app. Details
 * on the class.
 */

import Clutter from 'gi://Clutter';
import Cogl from 'gi://Cogl';
import GLib from 'gi://GLib';
import Gio from 'gi://Gio';
import Meta from 'gi://Meta';
import Shell from 'gi://Shell';
import St from 'gi://St';

import * as AltTab from 'resource:///org/gnome/shell/ui/altTab.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as Workspace from 'resource:///org/gnome/shell/ui/workspace.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

// The shell's own screenshot UI promisifies the first two at start-up and
// `_promisify` ignores a second request, so these only matter if the shell
// ever stops importing that module eagerly.
Gio._promisify(Shell.Screenshot.prototype, 'screenshot_stage_to_content');
Gio._promisify(Shell.Screenshot, 'composite_to_stream');
Gio._promisify(Gio.DBusConnection.prototype, 'call');

/**
 * WM_CLASS of every Snipdeck window, lower-cased: the application id, and the
 * bare name it had before — a running older Snipdeck is still recognised.
 */
const WM_CLASSES = new Set(['com.revisetouch.snipdeck', 'snipdeck']);
/** Desktop entries the shell may have tied Snipdeck's windows to. */
const APP_IDS = new Set(['com.revisetouch.Snipdeck.desktop', 'snipdeck.desktop']);

/** Title prefix that marks a floating snip, as opposed to the gallery window. */
const SNIP_TITLE = 'SnipdeckFloating';

/**
 * Windows that open ON a snip, and so have to sit above the snips too.
 *
 * Raising the snips alone put them in a layer of their own, and these stayed
 * in the ordinary one below it: a snip's right-click menu opened underneath
 * any other snip it overlapped, half its items out of reach. Raised, not
 * placed and not stuck — each one is shown where it is needed and goes away.
 */
const ABOVE_TITLES = new Set([
    'SnipdeckContextMenu',
    'SnipdeckColorResult',
    'SnipdeckTranslate',
    'SnipdeckUpload',
]);

/**
 * Word in the title asking for this snip to be left in the window lists.
 *
 * The app's "show floating snips in the taskbar" setting, carried across. On
 * Windows that setting becomes WS_EX_TOOLWINDOW; here it decides whether a snip
 * appears in the overview and the switcher. Absent means hide, which keeps the
 * default the same as it has always been and makes an older build — one whose
 * titles never carry the word — behave exactly as before.
 */
const LISTED = 'listed';

/**
 * Snipdeck's name on the session bus, owned for exactly as long as the app
 * runs. That makes it the "is Snipdeck there" signal as well as the address:
 * Super+Alt is only claimed while it has an owner, so with the app closed the
 * chord is a window move again. The other end is src/bus_service.rs.
 */
const BUS_NAME = 'com.revisetouch.Snipdeck';
const OBJECT_PATH = '/com/revisetouch/Snipdeck';
const BUS_INTERFACE = 'com.revisetouch.Snipdeck1';

/**
 * Super can arrive as the real Mod4 bit, the virtual Super bit or both,
 * depending on the keymap; either counts.
 */
const SUPER = Clutter.ModifierType.SUPER_MASK | Clutter.ModifierType.MOD4_MASK;
const ALT = Clutter.ModifierType.MOD1_MASK;

/** The app's selection look, from ui/overlay.slint. */
const BORDER = '#ff8a3d';
const DIM = 'rgba(0, 0, 0, 0.376)';

/**
 * Narrower or shorter than this, in physical pixels, is a click rather than a
 * snip. The app drops such a region too (src/app/input.rs, at Release); this
 * just spares it the PNG.
 */
const MIN_SNIP = 4;

/** How long a selection border may wait for the snip window that replaces it. */
const HANDOFF_MS = 2000;

/**
 * The selection outline from ui/overlay.slint: orange, white, orange, a pixel
 * each, the white for contrast on any background. Sized by `sizeBorder`.
 */
function makeBorder() {
    const outer = new St.Widget({style: `border: 1px solid ${BORDER};`});
    const white = new St.Widget({style: 'border: 1px solid white;', x: 1, y: 1});
    white.add_child(new St.Widget({style: `border: 1px solid ${BORDER};`, x: 1, y: 1}));
    outer.add_child(white);
    return outer;
}

/** Fit a `makeBorder` outline around a `w`×`h` area; the outer pixel sits just outside it. */
function sizeBorder(outer, w, h) {
    outer.set_size(w + 2, h + 2);
    const white = outer.get_first_child();
    white.set_size(w, h);
    white.get_first_child().set_size(Math.max(w - 2, 0), Math.max(h - 2, 0));
}

/**
 * Crosshair while armed, as the app's own overlay shows. Newer shells set
 * cursors per actor; older ones have one global cursor on the display.
 */
function setCrosshair(actor, on) {
    if (actor.set_cursor_type)
        actor.set_cursor_type(on ? Clutter.CursorType.CROSSHAIR : Clutter.CursorType.INHERIT);
    else
        global.display.set_cursor(on ? Meta.Cursor.CROSSHAIR : Meta.Cursor.DEFAULT);
}

/**
 * Super+Alt+drag to snip.
 *
 * Why here
 * --------
 * Snipdeck's capture gesture on Windows is Win+drag, read by a global mouse
 * hook. Wayland grants no client a global hook, so on GNOME the gesture has to
 * be read by the compositor — which this extension is part of. Alt is added
 * because plain Super+drag is Mutter's window move, and that should keep
 * working.
 *
 * Why arm on the keys, not on the click
 * -------------------------------------
 * A press over a window never reaches the shell while Super is held: Mutter
 * turns it into a window drag first — raising and focusing the window, then
 * starting the move — and nothing on that path can be vetoed from JavaScript.
 * What diverts it is a stage grab: while the shell holds one, Mutter routes
 * every pointer event to the shell and none to windows (src/core/events.c,
 * `stage_has_grab`). So the grab is taken the moment Super and Alt are both
 * down and dropped the moment either is released, before any click can land.
 * The keymap's `state-changed` signal fires on modifier changes and nothing
 * else, so between snips this costs nothing: no polling, no per-event handler.
 * Keybindings still run — Mutter matches them before it looks at grabs.
 *
 * What the snip contains
 * ----------------------
 * The grab has a price: the window under the pointer is told the pointer left,
 * so hover states and tooltips go away — exactly the things one snips. So the
 * stage is painted to a texture first, before the grab exists. That frame is
 * what the first drag cuts from, and it is shown under the selection for the
 * length of the drag, so what is selected is what is kept. A further drag with
 * the keys still held paints afresh at its own press. The paint is synchronous
 * — `screenshot_stage_to_content` renders inside the call — so the frame is
 * the one on screen when the keys went down, open menus included.
 *
 * What crosses to the app
 * -----------------------
 * The region as a PNG, cut by the shell's own screenshot code — no portal, no
 * "sharing your screen" indicator, no shutter — plus its top-left corner in the
 * app's physical-pixel convention and the Ctrl/Shift that pick the routing.
 * Alt is the trigger here, not a choice, so it is not passed on.
 */
class SnipGesture {
    enable() {
        this._appRunning = false;
        /** While Super+Alt are held: `{actor, grab, shot, fresh}`. */
        this._armed = null;
        /** While the button is down. */
        this._drag = null;
        /** The border left standing until the snip window replaces it. */
        this._handoff = null;

        this._watchId = Gio.bus_watch_name(
            Gio.BusType.SESSION, BUS_NAME, Gio.BusNameWatcherFlags.NONE,
            () => {
                this._appRunning = true;
                this._sync();
            },
            () => {
                this._appRunning = false;
                this._sync();
            });

        const backend = global.stage.context?.get_backend() ?? Clutter.get_default_backend();
        this._keymap = backend.get_default_seat().get_keymap();
        this._keymapId = this._keymap.connect('state-changed', () => this._sync());
    }

    disable() {
        if (this._watchId) {
            Gio.bus_unwatch_name(this._watchId);
            this._watchId = 0;
        }
        if (this._keymapId) {
            this._keymap.disconnect(this._keymapId);
            this._keymapId = 0;
        }
        this._endDrag();
        this._disarm();
        this._endHandoff();
    }

    /** A snip window has painted its first frame; the border standing in for it can go. */
    snipShown() {
        this._endHandoff();
    }

    /**
     * Armed while Super+Alt are held and the app is there to receive the snip,
     * disarmed otherwise — but never under a drag, which ends on its release.
     */
    _sync() {
        const [, , mods] = global.get_pointer();
        if (this._appRunning && (mods & SUPER) && (mods & ALT))
            this._arm();
        else if (!this._drag)
            this._disarm();
    }

    _arm() {
        if (this._armed)
            return;
        // Only from the plain desktop. A grab already held — the overview, a
        // shell menu, the screenshot UI, a modal dialog — owns the pointer for
        // its own reasons.
        if (Main.actionMode !== Shell.ActionMode.NORMAL || global.stage.get_grab_actor())
            return;

        // Before the grab: see "What the snip contains" above.
        this._endHandoff();
        const shot = this._paintStage();

        const actor = new Clutter.Actor({
            reactive: true,
            width: global.stage.width,
            height: global.stage.height,
        });
        Main.layoutManager.uiGroup.add_child(actor);
        actor.connect('button-press-event', (_actor, event) => this._onPress(event));
        actor.connect('motion-event', (_actor, event) => this._onMotion(event));
        actor.connect('button-release-event', (_actor, event) => this._onRelease(event));
        actor.connect('key-press-event', (_actor, event) => this._onKey(event));
        setCrosshair(actor, true);

        this._armed = {actor, grab: global.stage.grab(actor), shot, fresh: true};
    }

    _disarm() {
        if (!this._armed)
            return;
        const {actor, grab} = this._armed;
        this._armed = null;
        grab.dismiss();
        setCrosshair(actor, false);
        actor.destroy();
    }

    /**
     * Paint the whole stage, now. Resolves to `[content, scale]`, or to null if
     * the paint failed — never rejects, so a frame nobody ends up using cannot
     * surface as an unhandled rejection.
     *
     * A fresh `Shell.Screenshot` each time: the object keeps per-operation
     * state, and a press can ask for a frame while the arm's is still settling.
     */
    _paintStage() {
        return new Shell.Screenshot().screenshot_stage_to_content().catch(e => {
            logError(e, 'snipdeck: could not paint the stage');
            return null;
        });
    }

    /**
     * Belt and braces for `state-changed`. A grab that outlived its keys would
     * take the pointer from every window on the desktop, so any event that
     * finds Super+Alt released ends it — the next mouse move at the latest.
     */
    _keysStillHeld(event) {
        const mods = event.get_state();
        if (this._appRunning && (mods & SUPER) && (mods & ALT))
            return true;
        this._disarm();
        return false;
    }

    _onPress(event) {
        if (this._drag || !this._keysStillHeld(event) ||
            event.get_button() !== Clutter.BUTTON_PRIMARY)
            return Clutter.EVENT_STOP;

        // The frame painted when the keys went down, unless a drag already used
        // it — then a new one, painted now. A border still standing in for the
        // previous snip goes first, or it would be in the picture.
        let shot = this._armed.shot;
        if (!this._armed.fresh) {
            this._endHandoff();
            shot = this._paintStage();
        }
        this._armed.shot = null;
        this._armed.fresh = false;

        const display = global.display;
        const monitor = display.get_current_monitor();
        const state = event.get_state();
        const [x, y] = event.get_coords();

        this._drag = {
            x0: x, y0: y, x1: x, y1: y,
            // One monitor per snip, as on Windows: the app places and sizes a
            // snip in that monitor's scale, so a region spanning two scales
            // has no single right answer.
            geometry: display.get_monitor_geometry(monitor),
            scale: display.get_monitor_scale(monitor),
            ctrl: (state & Clutter.ModifierType.CONTROL_MASK) !== 0,
            shift: (state & Clutter.ModifierType.SHIFT_MASK) !== 0,
            shot,
            view: this._buildView(shot),
        };
        // A fullscreen window may be scanned out directly, past the stage, and
        // the selection would be painted where nobody sees it.
        global.compositor.disable_unredirect();
        this._layout();
        return Clutter.EVENT_STOP;
    }

    _onMotion(event) {
        if (this._drag) {
            const [x, y] = event.get_coords();
            this._moveTo(x, y);
        } else {
            this._keysStillHeld(event);
        }
        return Clutter.EVENT_STOP;
    }

    _onRelease(event) {
        if (!this._drag || event.get_button() !== Clutter.BUTTON_PRIMARY)
            return Clutter.EVENT_STOP;

        const [x, y] = event.get_coords();
        this._moveTo(x, y);
        const drag = this._endDrag();

        const region = this._region(drag);
        if (region) {
            // Only a floating snip replaces the border; the Shift routings
            // leave nothing on screen to wait for.
            if (!drag.shift)
                this._beginHandoff(region);
            this._deliver(drag, region).catch(e => {
                this._endHandoff();
                logError(e, 'snipdeck: snip not delivered');
            });
        }

        // Keys still held: armed for the next one, as with Win held on
        // Windows. Keys let go during the drag: done.
        this._sync();
        return Clutter.EVENT_STOP;
    }

    _onKey(event) {
        if (event.get_key_symbol() === Clutter.KEY_Escape && this._drag) {
            this._endDrag();
            this._sync();
        }
        return Clutter.EVENT_STOP;
    }

    _moveTo(x, y) {
        const d = this._drag;
        const g = d.geometry;
        d.x1 = Math.max(g.x, Math.min(x, g.x + g.width));
        d.y1 = Math.max(g.y, Math.min(y, g.y + g.height));
        this._layout();
    }

    /** Tear down the drag's visuals and hand back what it selected. */
    _endDrag() {
        const drag = this._drag;
        if (!drag)
            return null;
        this._drag = null;
        drag.view.root.destroy();
        global.compositor.enable_unredirect();
        return drag;
    }

    /**
     * The frozen frame under a dimmed screen, the selection outline and its
     * size readout. Lives inside the grab actor, so none of it can take events.
     */
    _buildView(shot) {
        const root = new Clutter.Actor({width: global.stage.width, height: global.stage.height});

        const backdrop = new Clutter.Actor({width: global.stage.width, height: global.stage.height});
        root.add_child(backdrop);
        let alive = true;
        root.connect('destroy', () => {
            alive = false;
        });
        shot.then(result => {
            if (alive && result)
                backdrop.set_content(result[0]);
        });

        const dim = [0, 1, 2, 3].map(() => new St.Widget({style: `background-color: ${DIM};`}));
        dim.forEach(panel => root.add_child(panel));

        const border = makeBorder();
        root.add_child(border);

        const size = new St.Label({
            style: 'background-color: rgba(0, 0, 0, 0.8); color: white; ' +
                'font-size: 12px; border-radius: 4px; padding: 3px 8px;',
        });
        root.add_child(size);

        this._armed.actor.add_child(root);
        return {root, dim, border, size};
    }

    _layout() {
        const d = this._drag;
        const {x, y, width, height} = d.geometry;
        const l = Math.min(d.x0, d.x1);
        const t = Math.min(d.y0, d.y1);
        const r = Math.max(d.x0, d.x1);
        const b = Math.max(d.y0, d.y1);

        // Four panels around the selection rather than one mask over it, as in
        // ui/overlay.slint, so the selected area stays untouched.
        const [above, below, left, right] = d.view.dim;
        above.set_position(x, y);
        above.set_size(width, t - y);
        below.set_position(x, b);
        below.set_size(width, y + height - b);
        left.set_position(x, t);
        left.set_size(l - x, b - t);
        right.set_position(r, t);
        right.set_size(x + width - r, b - t);

        const visible = r > l && b > t;
        d.view.border.visible = visible;
        d.view.size.visible = visible;
        if (!visible)
            return;

        sizeBorder(d.view.border, r - l, b - t);
        d.view.border.set_position(l - 1, t - 1);

        // Physical pixels: the size the snip will actually be.
        d.view.size.text =
            `${Math.round((r - l) * d.scale)} × ${Math.round((b - t) * d.scale)}`;
        d.view.size.set_position(
            Math.min(r + 6, x + width - 88),
            Math.min(b + 6, y + height - 24));
    }

    /**
     * The selection in logical stage pixels (`l t r b`) and its top-left corner
     * in the app's physical ones (`px py`); null if it is too small to keep.
     *
     * The app's physical space is logical × that monitor's scale, origin
     * included — what `_toLogical` undoes when the floating snip comes back.
     */
    _region(drag) {
        const l = Math.min(drag.x0, drag.x1);
        const t = Math.min(drag.y0, drag.y1);
        const r = Math.max(drag.x0, drag.x1);
        const b = Math.max(drag.y0, drag.y1);
        const s = drag.scale;
        const px = Math.round(l * s);
        const py = Math.round(t * s);
        if (Math.round(r * s) - px < MIN_SNIP || Math.round(b * s) - py < MIN_SNIP)
            return null;
        return {l, t, r, b, px, py};
    }

    /**
     * The w x h region at (x, y) of `texture`, drawn over opaque black into a
     * texture of its own: every pixel ends fully opaque, with the colour the
     * screen showed. The frame's pixels are premultiplied, so "over black"
     * keeps them as they are and only fixes the alpha. Null if Cogl refuses,
     * and the caller cuts from the frame as before.
     */
    _flatten(texture, x, y, w, h) {
        try {
            const ctx = global.stage.context.get_backend().get_cogl_context();
            const target = Cogl.Texture2D.new_with_size(ctx, w, h);
            const fb = Cogl.Offscreen.new_with_texture(target);
            fb.allocate();
            fb.clear4f(Cogl.BufferBit.COLOR, 0, 0, 0, 1);
            const pipeline = Cogl.Pipeline.new(ctx);
            pipeline.set_layer_texture(0, texture);
            pipeline.set_blend('RGBA = ADD(SRC_COLOR, DST_COLOR*(1-SRC_COLOR[A]))');
            const tw = texture.get_width();
            const th = texture.get_height();
            fb.draw_textured_rectangle(pipeline, -1, 1, 1, -1,
                x / tw, y / th, (x + w) / tw, (y + h) / th);
            // Cogl batches the draw; the PNG encoder reads the texture
            // directly and would find it empty.
            fb.finish();
            return target;
        } catch (e) {
            logError(e, 'snipdeck: could not flatten the snip; using the frame as is');
            return null;
        }
    }

    /** Cut the region out of the frame, encode it, and send it to the app. */
    async _deliver(drag, region) {
        const result = await drag.shot;
        if (!result)
            throw new Error('no frame to cut the snip from');
        const [content, scale] = result;
        const texture = content.get_texture();

        // The frame has one scale for the whole stage — the largest of any
        // monitor — which need not be this monitor's, so the cut is made in
        // the frame's own pixels. The app sizes the snip from the PNG, not
        // from anything sent alongside it.
        const x = Math.round(region.l * scale);
        const y = Math.round(region.t * scale);
        const w = Math.min(Math.round(region.r * scale), texture.get_width()) - x;
        const h = Math.min(Math.round(region.b * scale), texture.get_height()) - y;

        // Over an opaque ground first. Some windows' buffers carry an alpha
        // channel the compositor ignores on screen (the window is declared
        // opaque) but the stage paint copies; turned into a PNG, those
        // pixels came out transparent with their colour discarded — snips of
        // such windows were white, grey or see-through.
        const opaque = this._flatten(texture, x, y, w, h);
        const stream = Gio.MemoryOutputStream.new_resizable();
        await Shell.Screenshot.composite_to_stream(
            opaque ?? texture, opaque ? 0 : x, opaque ? 0 : y, w, h, scale, null, 0, 0, 1, stream);
        stream.close(null);

        const png = GLib.Variant.new_from_bytes(
            new GLib.VariantType('ay'), stream.steal_as_bytes(), true);
        await Gio.DBus.session.call(
            BUS_NAME, OBJECT_PATH, BUS_INTERFACE, 'SnipRegion',
            GLib.Variant.new_tuple([
                png,
                GLib.Variant.new_int32(region.px),
                GLib.Variant.new_int32(region.py),
                GLib.Variant.new_boolean(drag.ctrl),
                GLib.Variant.new_boolean(drag.shift),
            ]),
            null, Gio.DBusCallFlags.NO_AUTO_START, 10000, null);
    }

    /**
     * Leave the selection's outline up until the floating snip is on screen.
     *
     * The app's own overlay does the same, for the same reason (src/app/input.rs,
     * at Release): without it the region blinks empty for the length of a PNG
     * encode, a bus round trip and a window map before the snip appears.
     */
    _beginHandoff({l, t, r, b}) {
        this._endHandoff();
        const actor = makeBorder();
        sizeBorder(actor, r - l, b - t);
        actor.set_position(l - 1, t - 1);
        Main.layoutManager.uiGroup.add_child(actor);

        const timeoutId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, HANDOFF_MS, () => {
            this._handoff.timeoutId = 0;
            this._endHandoff();
            return GLib.SOURCE_REMOVE;
        });
        this._handoff = {actor, timeoutId};
    }

    _endHandoff() {
        if (!this._handoff)
            return;
        const {actor, timeoutId} = this._handoff;
        this._handoff = null;
        if (timeoutId)
            GLib.source_remove(timeoutId);
        actor.destroy();
    }
}

export default class SnipdeckExtension extends Extension {
    enable() {
        // One line at start-up, so a session can be asked which build of
        // this file it is actually running. GNOME Shell caches the loaded
        // module, and on Wayland it cannot re-import one without a new
        // session — without this marker there is no way to tell a fix that
        // did not work from a fix that was never loaded.
        log('snipdeck: enable() match=title placement=first-frame lists=switcher+overview+running listed-flag=on gesture=super-alt-drag');

        this._handles = [];
        this._pending = new Set();
        this._placing = new Set();

        this._gesture = new SnipGesture();
        this._gesture.enable();

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

        this._patchWindowLists();
    }

    /**
     * Keep snips out of the two places the shell lists windows.
     *
     * Mutter exposes only `is_skip_taskbar`, a read-only query derived from the
     * window type — the flag cannot be set by the application OR by an
     * extension. What can be done is filter the lists themselves, which live in
     * GNOME Shell's own JavaScript.
     *
     * Each patch is taken separately and guarded separately: a future Shell may
     * rename or restructure either one, and losing a filter is a cosmetic
     * regression while breaking the switcher or the overview is not.
     */
    _patchWindowLists() {
        this._patchWindowSwitcher();
        this._patchOverview();
        this._patchRunningApps();
    }

    /**
     * The dock, the dash and Alt+Tab's application list.
     *
     * Each floating snip is a real window, and the shell lists an app as
     * running for as long as it has one: with the gallery hidden in the tray
     * and a snip on screen, Snipdeck sat in the dock with a running dot. All
     * three ask `Shell.AppSystem.get_running()`, so Snipdeck is left out of
     * that answer while every window it has is a snip or one of the windows
     * opened on a snip. The gallery open, or snips asked to be listed, and it
     * is back.
     */
    _patchRunningApps() {
        const proto = Shell.AppSystem?.prototype;
        if (!proto?.get_running) {
            log('snipdeck: running apps not patchable on this Shell version');
            return;
        }

        const quiet = app => {
            try {
                const windows = app.get_windows();
                // Snipdeck only: no other app's dock entry is ours to touch.
                const ours = APP_IDS.has(app.get_id()) ||
                    windows.every(w => WM_CLASSES.has((w.get_wm_class() ?? '').toLowerCase()));
                if (!ours || windows.length === 0)
                    return false;
                // Untitled or not yet mapped counts as quiet too: a snip is
                // created first and titled a moment later, and the hidden
                // start builds the gallery unmapped and drops it the next
                // turn — counted in between, each flashed the app into the
                // dock and out again.
                return windows.every(w => {
                    const title = w.get_title() ?? '';
                    const r = w.get_frame_rect();
                    return title === '' || r.width === 0 || r.height === 0 ||
                        this._isSnipWindow(w) || ABOVE_TITLES.has(title);
                });
            } catch (_) {
                return false;
            }
        };
        this._origGetRunning = proto.get_running;
        const orig = this._origGetRunning;
        proto.get_running = function () {
            return orig.call(this).filter(app => !quiet(app));
        };

        // The dock redraws when an app starts or stops, and opening the
        // gallery while snips are on screen does neither: the app was
        // running all along. Tell it when the answer above changes.
        const appSystem = Shell.AppSystem.get_default();
        this._quietApps = new Map();
        const recheck = () => {
            for (const app of orig.call(appSystem)) {
                const now = quiet(app);
                const id = app.get_id();
                if (this._quietApps.has(id) && this._quietApps.get(id) !== now)
                    appSystem.emit('app-state-changed', app);
                this._quietApps.set(id, now);
            }
        };
        // What the dock last heard is what the answer was when the shell said
        // the state changed — record it then. Recorded only by the recheck, an
        // app the shell announced while quiet and first rechecked once it no
        // longer was would have been left out of the dock with nothing to
        // correct it.
        this._handles.push([appSystem, appSystem.connect('app-state-changed',
            (_s, app) => this._quietApps.set(app.get_id(), quiet(app)))]);
        // A window joins the app unmapped and untitled, so the answer can
        // change later than that: when it is mapped, and when it goes.
        const tracker = Shell.WindowTracker.get_default();
        this._handles.push([tracker, tracker.connect('tracked-windows-changed', recheck)]);
        const wm = global.window_manager;
        this._handles.push([wm, wm.connect('map', recheck)]);
        this._handles.push([wm, wm.connect('destroy', recheck)]);
    }

    /**
     * Alt+` and Alt+Tab's per-app expansion.
     *
     * Only the *window* switcher is touched. The application switcher still
     * shows Snipdeck once, which is correct: the gallery is a real window the
     * user wants to reach. It is the dozen pinned snips that should not each be
     * a stop on the way there.
     */
    _patchWindowSwitcher() {
        const proto = AltTab.WindowSwitcherPopup?.prototype;
        if (!proto?._getWindowList) {
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

    /**
     * The Activities overview — what Super shows.
     *
     * A separate code path from the switcher, and the one people actually hit:
     * pressing Super threw every pinned snip into the window picker alongside
     * real applications. `Workspace._isOverviewWindow` is the single question
     * the overview asks about each window, and it asks it for no other reason,
     * so answering "no" for a snip removes it and nothing else.
     *
     * Safe for these windows specifically: a window rejected here is dropped
     * outright only when it has no transient parent, and snips have none —
     * `window_tamer` is a no-op off Windows. A rejected *transient* would
     * instead be re-attached to its ancestor as a dialog, which is not a shape
     * any snip takes.
     */
    _patchOverview() {
        const proto = Workspace.Workspace?.prototype;
        if (!proto?._isOverviewWindow) {
            log('snipdeck: overview not patchable on this Shell version');
            return;
        }

        const isSnip = w => this._isSnipWindow(w);
        this._origIsOverviewWindow = proto._isOverviewWindow;
        const orig = this._origIsOverviewWindow;
        proto._isOverviewWindow = function (window) {
            return isSnip(window) ? false : orig.call(this, window);
        };
    }

    /**
     * Should this window be kept out of the switcher and the overview?
     *
     * A snip, unless its title asks to be listed. Placement and stacking are
     * applied either way — being visible in the overview is not a reason to
     * stop pinning a snip where it was captured.
     */
    _isSnipWindow(window) {
        try {
            const title = window.get_title() ?? '';
            if (!title.startsWith(SNIP_TITLE))
                return false;
            return !this._wantsListing(title);
        } catch (_) {
            return false;
        }
    }

    /** `SnipdeckFloating listed 100,200` -> true; `SnipdeckFloating 100,200` -> false. */
    _wantsListing(title) {
        return title.slice(SNIP_TITLE.length).trimStart().startsWith(LISTED);
    }

    disable() {
        // First: it may be holding a stage grab, and a grab left behind takes
        // the pointer away from every window on the desktop.
        this._gesture?.disable();
        this._gesture = null;

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
        if (this._origIsOverviewWindow) {
            Workspace.Workspace.prototype._isOverviewWindow =
                this._origIsOverviewWindow;
            this._origIsOverviewWindow = null;
        }
        if (this._origGetRunning) {
            Shell.AppSystem.prototype.get_running = this._origGetRunning;
            this._origGetRunning = null;
        }
        this._quietApps = null;

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
            return !!wmClass && WM_CLASSES.has(wmClass.toLowerCase());
        } catch (_) {
            return false;
        }
    }

    /**
     * Apply the treatment if this is a titled snip window.
     * Returns true once handled, so callers can stop waiting.
     */
    _adopt(window) {
        if (!window)
            return false;

        // Before `_isOurs`, which a Wayland window without a snip's title
        // never passes.
        if (ABOVE_TITLES.has(window.get_title() ?? '')) {
            this._raise(window);
            this._applyWhenPlaced(window);
            return true;
        }

        if (!this._isOurs(window))
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

    /**
     * Into the snips' layer, and to the top of it.
     *
     * The menu covers the screen (see `context_menu.rs`), and GNOME maximises
     * a window that size on its first map. Mutter keeps a maximised window in
     * the normal layer whatever its "above" state says, so `make_above` was
     * accepted and did nothing. Measured: the menu reported above=true,
     * maximized=true, layer NORMAL, under the snips in TOP. So the maximise is
     * undone and the window given the same work-area rectangle by hand — the
     * menu finds the pointer in its own surface and sizes itself to what it
     * gets, so the panel still opens at the cursor and stays on screen.
     */
    _raise(window) {
        if (this._isMaximized(window)) {
            this._unmaximize(window);
            const area = window.get_work_area_current_monitor();
            window.move_resize_frame(true, area.x, area.y, area.width, area.height);
        }
        // Off and on: Mutter recomputes the layer only when the state changes,
        // and this window was already marked above while it was maximised.
        window.unmake_above();
        window.make_above();
        window.raise();
    }

    /** GNOME 49 replaced the per-direction flags with a yes/no. */
    _isMaximized(window) {
        try {
            if (typeof window.is_maximized === 'function')
                return window.is_maximized();
            return window.get_maximized() !== 0;
        } catch (_) {
            return false;
        }
    }

    _unmaximize(window) {
        try {
            window.unmaximize();
        } catch (_) {
            // GNOME 45–48: the directions are an argument.
            try {
                window.unmaximize(Meta.MaximizeFlags.BOTH);
            } catch (e) {
                log(`snipdeck: could not unmaximize: ${e.message}`);
            }
        }
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
            if (title && title.startsWith(SNIP_TITLE)) {
                this._apply(window, title);
                // A Super+Alt snip's outline has been waiting for exactly this.
                this._gesture?.snipShown();
            } else if (ABOVE_TITLES.has(title ?? '')) {
                this._raise(window);
            }
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
