// A stand-in StatusNotifierWatcher that reports a tray host, so Snipdeck
// believes it has a tray: its window's X then hides instead of quitting, as
// on a desktop with the AppIndicator extension.
import GLib from 'gi://GLib';
import Gio from 'gi://Gio';

const XML = `<node><interface name="org.kde.StatusNotifierWatcher">
  <method name="RegisterStatusNotifierItem"><arg type="s" direction="in"/></method>
  <method name="RegisterStatusNotifierHost"><arg type="s" direction="in"/></method>
  <property name="RegisteredStatusNotifierItems" type="as" access="read"/>
  <property name="IsStatusNotifierHostRegistered" type="b" access="read"/>
  <property name="ProtocolVersion" type="i" access="read"/>
  <signal name="StatusNotifierItemRegistered"><arg type="s"/></signal>
  <signal name="StatusNotifierHostRegistered"/>
</interface></node>`;

const items = [];
const impl = Gio.DBusExportedObject.wrapJSObject(XML, {
    RegisterStatusNotifierItem(service) { items.push(service); },
    RegisterStatusNotifierHost() {},
    get RegisteredStatusNotifierItems() { return items; },
    get IsStatusNotifierHostRegistered() { return true; },
    get ProtocolVersion() { return 0; },
});
impl.export(Gio.DBus.session, '/StatusNotifierWatcher');
Gio.bus_own_name(Gio.BusType.SESSION, 'org.kde.StatusNotifierWatcher',
    Gio.BusNameOwnerFlags.NONE, null, null, null);
new GLib.MainLoop(null, false).run();
