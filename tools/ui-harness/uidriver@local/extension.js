// Replays plan.json against the headless shell: clicks relative to a window,
// keys, waits and stage screenshots. Logs each step as "ui: ...".
import Clutter from 'gi://Clutter';
import GLib from 'gi://GLib';
import Gio from 'gi://Gio';
import Shell from 'gi://Shell';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

Gio._promisify(Shell.Screenshot.prototype, 'screenshot_stage_to_content');
Gio._promisify(Shell.Screenshot, 'composite_to_stream');
Gio._promisify(Shell.Screenshot.prototype, 'pick_color');

const DIR = GLib.getenv('UI_DIR');
const sleep = ms => new Promise(r => GLib.timeout_add(GLib.PRIORITY_DEFAULT, ms, () => { r(); return GLib.SOURCE_REMOVE; }));
const now = () => GLib.get_monotonic_time();

Gio._promisify(Gio.DBusConnection.prototype, 'call');

/** A method of the shell's screencast service, unpacked. */
async function screencast(method, params) {
    const reply = await Gio.DBus.session.call(
        'org.gnome.Shell.Screencast', '/org/gnome/Shell/Screencast',
        'org.gnome.Shell.Screencast', method, params, null,
        Gio.DBusCallFlags.NONE, 10000, null);
    return reply.deepUnpack();
}

function findWindow(title) {
    // Several windows can share a title (a hidden 0x0 menu surface beside the
    // visible one); only a mapped window with a real size is a target.
    return global.get_window_actors()
        .filter(a => a.visible)
        .map(a => a.meta_window)
        // "Name*" matches by prefix: a floating snip's title carries its
        // position, which depends on the monitor's scale.
        .find(w => {
            const t = w.get_title() ?? '';
            const hit = title.endsWith('*') ? t.startsWith(title.slice(0, -1)) : t === title;
            return hit && w.get_frame_rect().width > 0;
        });
}

export default class UiDriver extends Extension {
    enable() { this._run().catch(e => logError(e, 'ui: FAILED')); }
    disable() {}

    async _run() {
        const plan = JSON.parse(new TextDecoder().decode(GLib.file_get_contents(`${DIR}/plan.json`)[1]));
        const seat = global.stage.context.get_backend().get_default_seat();
        const kbd = seat.create_virtual_device(Clutter.InputDeviceType.KEYBOARD_DEVICE);
        const ptr = seat.create_virtual_device(Clutter.InputDeviceType.POINTER_DEVICE);
        ptr.notify_absolute_motion(now(), 800, 500);
        for (const step of plan) {
            if (step.wait) await sleep(step.wait);
            if (step.hideOverview) Main.overview.hide();
            if (step.waitFor) {
                // Poll until the window exists (and, with minY, has settled below
                // the panel — the menu surface maps at y=0 and is moved after).
                let w = null;
                for (let t = 0; t < (step.timeout ?? 5000); t += 100) {
                    w = findWindow(step.waitFor);
                    if (w && w.get_frame_rect().y >= (step.minY ?? 0)) break;
                    w = null;
                    await sleep(100);
                }
                log(`ui: waitFor "${step.waitFor}" ${w ? 'ok' : 'TIMEOUT'}`);
                await sleep(300);
            }
            if (step.watchStates) {
                Shell.AppSystem.get_default().connect('app-state-changed', (_s, app) =>
                    log(`ui: state-changed ${app.get_id()} running=${Shell.AppSystem.get_default().get_running().map(a => a.get_id()).join(',')}`));
            }
            if (step.apps) {
                for (const app of Shell.AppSystem.get_default().get_running()) {
                    const ws = app.get_windows().map(w => `${w.get_title()}[${w.get_frame_rect().width}x${w.get_frame_rect().height}${w.is_skip_taskbar() ? ' skip' : ''}]`);
                    log(`ui: app "${app.get_id()}" state=${app.get_state()} icon=${app.get_icon()?.to_string?.() ?? '?'} windows=${ws.join(', ')}`);
                }
            }
            if (step.windows) {
                for (const a of global.get_window_actors()) {
                    const w = a.meta_window; const r = w.get_frame_rect();
                    log(`ui: window "${w.get_title()}" ${r.x},${r.y} ${r.width}x${r.height} layer=${w.get_layer()} above=${w.is_above()} type=${w.get_window_type()} focus=${w.has_focus()} max=${typeof w.get_maximized === "function" ? w.get_maximized() : (typeof w.is_maximized === "function" ? w.is_maximized() : "?")} fs=${w.is_fullscreen()}`);
                }
            }
            if (step.click) {
                const w = findWindow(step.click.win);
                if (!w) { log(`ui: no window "${step.click.win}"`); continue; }
                const r = w.get_frame_rect();
                const x = step.click.right !== undefined ? r.x + r.width - step.click.right : r.x + step.click.x;
                const y = step.click.bottom !== undefined ? r.y + r.height - step.click.bottom : r.y + step.click.y;
                ptr.notify_absolute_motion(now(), x, y);
                await sleep(150);
                const btn = step.click.button ?? Clutter.BUTTON_PRIMARY;
                ptr.notify_button(now(), btn, Clutter.ButtonState.PRESSED);
                await sleep(60);
                ptr.notify_button(now(), btn, Clutter.ButtonState.RELEASED);
                log(`ui: clicked ${x},${y} button ${btn}`);
            }
            if (step.probeClick) {
                // Click at absolute x, and y chosen by whether the probe pixel is
                // dark (a menu card) or not — for UI that lands in one of two places.
                const pc = step.probeClick;
                const [color] = await new Shell.Screenshot().pick_color(pc.probe[0], pc.probe[1]);
                const dark = color.red + color.green + color.blue < 160;
                const y = dark ? pc.ifDark : pc.otherwise;
                ptr.notify_absolute_motion(now(), pc.x, y);
                await sleep(150);
                ptr.notify_button(now(), Clutter.BUTTON_PRIMARY, Clutter.ButtonState.PRESSED);
                await sleep(60);
                ptr.notify_button(now(), Clutter.BUTTON_PRIMARY, Clutter.ButtonState.RELEASED);
                log(`ui: probeClick probe=${color.red},${color.green},${color.blue} dark=${dark} -> ${pc.x},${y}`);
            }
            if (step.drag) {
                const w = findWindow(step.drag.win);
                if (!w) { log(`ui: no window "${step.drag.win}"`); continue; }
                const r = w.get_frame_rect();
                const x0 = r.x + step.drag.x, y0 = r.y + step.drag.y;
                ptr.notify_absolute_motion(now(), x0, y0);
                await sleep(150);
                ptr.notify_button(now(), Clutter.BUTTON_PRIMARY, Clutter.ButtonState.PRESSED);
                for (let i = 1; i <= 12; i++) {
                    await sleep(25);
                    ptr.notify_absolute_motion(now(), x0 + step.drag.dx * i / 12, y0 + step.drag.dy * i / 12);
                }
                await sleep(80);
                ptr.notify_button(now(), Clutter.BUTTON_PRIMARY, Clutter.ButtonState.RELEASED);
                log(`ui: dragged ${x0},${y0} by ${step.drag.dx},${step.drag.dy}`);
            }
            if (step.move) {
                ptr.notify_absolute_motion(now(), step.move.x, step.move.y);
                log(`ui: moved ${step.move.x},${step.move.y}`);
            }
            if (step.key) {
                kbd.notify_keyval(now(), Clutter[`KEY_${step.key}`], Clutter.KeyState.PRESSED);
                await sleep(50);
                kbd.notify_keyval(now(), Clutter[`KEY_${step.key}`], Clutter.KeyState.RELEASED);
                log(`ui: key ${step.key}`);
            }
            // Held keys, a smooth pointer and the wheel: what a recording
            // needs to look like a person and not a teleport.
            // Where Wayland lets no client put itself: a window by title
            // (prefix with '*'), moved and sized in logical pixels.
            // Click a dialog's button until the dialog is gone: one click
            // can land before it takes input.
            if (step.dismiss) {
                const d = step.dismiss;
                let tries = 0;
                for (; tries < (d.tries ?? 6); tries++) {
                    const w = findWindow(d.win);
                    if (!w) break;
                    // Windows opened after it may cover it.
                    w.activate(now() / 1000);
                    await sleep(300);
                    const r = w.get_frame_rect();
                    ptr.notify_absolute_motion(now(), r.x + d.x, r.y + d.y);
                    await sleep(150);
                    ptr.notify_button(now(), Clutter.BUTTON_PRIMARY, Clutter.ButtonState.PRESSED);
                    await sleep(60);
                    ptr.notify_button(now(), Clutter.BUTTON_PRIMARY, Clutter.ButtonState.RELEASED);
                    await sleep(d.gap ?? 900);
                }
                log(`ui: dismissed "${d.win}" after ${tries} click(s): ${findWindow(d.win) ? 'STILL THERE' : 'gone'}`);
            }
            if (step.place) {
                const w = findWindow(step.place.win);
                if (!w) { log(`ui: no window "${step.place.win}"`); continue; }
                if (w.is_maximized?.()) w.unmaximize?.();
                w.move_resize_frame(true, step.place.x, step.place.y, step.place.w, step.place.h);
                if (step.place.raise) w.activate(now() / 1000);
                log(`ui: placed "${w.get_title()}"`);
            }
            if (step.keyDown || step.keyUp) {
                const name = step.keyDown ?? step.keyUp;
                kbd.notify_keyval(now(), Clutter[`KEY_${name}`],
                    step.keyDown ? Clutter.KeyState.PRESSED : Clutter.KeyState.RELEASED);
                log(`ui: key ${step.keyDown ? 'down' : 'up'} ${name}`);
            }
            if (step.glide) {
                // Ease in and out over `ms`, at about 60 events a second.
                const [x0, y0] = global.get_pointer();
                const {x, y} = step.glide;
                const ms = step.glide.ms ?? 600;
                const n = Math.max(2, Math.round(ms / 16));
                for (let i = 1; i <= n; i++) {
                    const t = i / n;
                    const e = t < 0.5 ? 2 * t * t : 1 - Math.pow(-2 * t + 2, 2) / 2;
                    ptr.notify_absolute_motion(now(), x0 + (x - x0) * e, y0 + (y - y0) * e);
                    await sleep(ms / n);
                }
                log(`ui: glided to ${x},${y}`);
            }
            if (step.button) {
                const btn = step.button.button ?? Clutter.BUTTON_PRIMARY;
                ptr.notify_button(now(), btn, step.button.press ? Clutter.ButtonState.PRESSED : Clutter.ButtonState.RELEASED);
                log(`ui: button ${btn} ${step.button.press ? 'pressed' : 'released'}`);
            }
            if (step.scroll) {
                const dir = step.scroll.up ? Clutter.ScrollDirection.UP : Clutter.ScrollDirection.DOWN;
                for (let i = 0; i < (step.scroll.n ?? 3); i++) {
                    ptr.notify_discrete_scroll(now(), dir, Clutter.ScrollSource.WHEEL);
                    await sleep(step.scroll.gap ?? 120);
                }
                log(`ui: scrolled ${step.scroll.up ? 'up' : 'down'} ${step.scroll.n ?? 3}`);
            }
            // The shell's own screencast of a logical-pixel area, to a file
            // in the shots dir. Needs RECORD=1 (PipeWire).
            if (step.record) {
                const r = step.record;
                const [ok, used] = await screencast('ScreencastArea', new GLib.Variant('(iiiisa{sv})', [
                    r.x, r.y, r.w, r.h, `${DIR}/shots/${r.file}`,
                    {
                        'framerate': new GLib.Variant('i', r.fps ?? 30),
                        'draw-cursor': new GLib.Variant('b', r.cursor ?? true),
                    },
                ]));
                log(`ui: recording ${ok} ${used}`);
            }
            if (step.stopRecord) {
                const [ok] = await screencast('StopScreencast', null);
                await sleep(1500);
                log(`ui: recording stopped ${ok}`);
            }
            if (step.type) {
                // One keysym per character. Only what the session's keymap
                // (US) can produce gets through: "ü" does, "ı" does not.
                for (const ch of step.type) {
                    const sym = Clutter.unicode_to_keysym(ch.codePointAt(0));
                    kbd.notify_keyval(now(), sym, Clutter.KeyState.PRESSED);
                    await sleep(30);
                    kbd.notify_keyval(now(), sym, Clutter.KeyState.RELEASED);
                    await sleep(60);
                }
                log(`ui: typed ${step.type}`);
            }
            if (step.shot) {
                const [content, scale] = await new Shell.Screenshot().screenshot_stage_to_content();
                const tex = content.get_texture();
                const stream = Gio.MemoryOutputStream.new_resizable();
                await Shell.Screenshot.composite_to_stream(tex, 0, 0, -1, -1, scale, null, 0, 0, 1, stream);
                stream.close(null);
                GLib.file_set_contents(`${DIR}/shots/${step.shot}`, stream.steal_as_bytes().toArray());
                log(`ui: shot ${step.shot}`);
            }
        }
        log('ui: done');
    }
}
