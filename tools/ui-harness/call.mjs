// Snip a PNG into the running Snipdeck as the GNOME extension does after a
// Super+Alt+drag: SnipRegion(png, x, y, ctrl, shift). shift=true keeps it to
// the gallery; shift=false also floats it on screen at (x, y).
//   gjs -m call.mjs file.png x y ctrl shift
import GLib from 'gi://GLib';
import Gio from 'gi://Gio';

Gio._promisify(Gio.DBusConnection.prototype, 'call');

const [path, x, y, ctrl, shift] = ARGV;
const bytes = Gio.File.new_for_path(path).load_bytes(null)[0];
const png = GLib.Variant.new_from_bytes(new GLib.VariantType('ay'), bytes, true);

const loop = new GLib.MainLoop(null, false);
Gio.DBus.session.call(
    'com.revisetouch.Snipdeck', '/com/revisetouch/Snipdeck',
    'com.revisetouch.Snipdeck1', 'SnipRegion',
    GLib.Variant.new_tuple([
        png,
        GLib.Variant.new_int32(Number(x)),
        GLib.Variant.new_int32(Number(y)),
        GLib.Variant.new_boolean(ctrl === 'true'),
        GLib.Variant.new_boolean(shift === 'true'),
    ]),
    null, Gio.DBusCallFlags.NO_AUTO_START, 10000, null)
    .then(reply => print(`reply: ${reply.print(true)}`))
    .catch(e => print(`error: ${e.message}`))
    .finally(() => loop.quit());
loop.run();
