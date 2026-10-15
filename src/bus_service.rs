//! Snipdeck on the session bus: `com.revisetouch.Snipdeck`, Linux only.
//!
//! Two callers, two methods:
//!
//! - **`SnipRegion`** — the GNOME Shell extension's way in, for snips taken
//!   with Super+Alt+drag. On Windows a snip starts from a global mouse hook —
//!   hold Win, drag. Wayland grants no client such a hook, and GNOME would not
//!   let a Super+drag through anyway: Super is Mutter's window-move modifier,
//!   so the press becomes a window drag before any client hears of it. The
//!   extension can see the press because it runs inside the compositor. It
//!   draws the selection, takes the pixels with the shell's own screenshot code
//!   — no portal, no "sharing your screen" indicator, no shutter — and hands
//!   the finished region over here: a PNG, its top-left corner in physical
//!   screen pixels (the space the rest of the app captures in) and the two
//!   modifiers that pick the routing. From there it is an ordinary snip: see
//!   `AppState::handle_region_snip`.
//!
//! - **`Show`** — a second launch asking the running Snipdeck to bring its
//!   window back. The single-instance guard used to make that second launch
//!   exit without a word, and with the window hidden and no tray on the panel
//!   the app was then running and unreachable: launching it again from the
//!   app grid did nothing at all.
//!
//! Owning the name doubles as the extension's "is Snipdeck running" signal: it
//! only claims Super+Alt while the name has an owner, so with the app closed
//! the chord goes back to being a window move. The extension's side is
//! `SnipGesture` in `packaging/gnome-extension/snipdeck@revisetouch.com`.

use std::sync::OnceLock;
use std::time::Duration;

use crossbeam_channel::{unbounded, Receiver, Sender};

/// Must match `BUS_NAME`, `OBJECT_PATH` and `BUS_INTERFACE` in the extension.
const BUS_NAME: &str = "com.revisetouch.Snipdeck";
const OBJECT_PATH: &str = "/com/revisetouch/Snipdeck";
const BUS_INTERFACE: &str = "com.revisetouch.Snipdeck1";

/// Held for the life of the process: dropping it releases the name, and the
/// extension would stop claiming Super+Alt.
static CONNECTION: OnceLock<zbus::Connection> = OnceLock::new();

/// What arrives over the bus for the UI thread.
pub enum BusEvent {
    /// A region the extension selected and captured.
    RegionSnip(RegionSnip),
    /// Bring the main window back (a second launch asked).
    Show,
    /// A shortcut's action, asked by `snipdeck --action <name>`.
    Hotkey(crate::hotkeys::HotkeyAction),
}

pub struct RegionSnip {
    pub image: image::RgbaImage,
    /// Top-left corner, in physical screen pixels.
    pub x: i32,
    pub y: i32,
    pub ctrl: bool,
    pub shift: bool,
}

struct Service {
    tx: Sender<BusEvent>,
}

impl Service {
    fn deliver(&self, event: BusEvent) -> zbus::fdo::Result<()> {
        self.tx
            .send(event)
            .map_err(|_| zbus::fdo::Error::Failed("Snipdeck is shutting down".into()))?;
        crate::pump::wake();
        Ok(())
    }
}

#[zbus::interface(name = "com.revisetouch.Snipdeck1")]
impl Service {
    /// One finished Super+Alt+drag.
    ///
    /// Decoded here, on the bus thread, so a large region costs the UI thread
    /// nothing. Answers once the snip is queued, not once it is saved: the
    /// extension has nothing to do with the outcome, and a reply held back
    /// until the gallery write would only keep its call open longer.
    async fn snip_region(
        &self,
        png: Vec<u8>,
        x: i32,
        y: i32,
        ctrl: bool,
        shift: bool,
    ) -> zbus::fdo::Result<()> {
        let image = image::load_from_memory_with_format(&png, image::ImageFormat::Png)
            .map_err(|e| zbus::fdo::Error::InvalidArgs(format!("not a PNG: {e}")))?
            .into_rgba8();
        log::info!(
            "region snip from the GNOME extension: {}x{} at ({x},{y}) ctrl={ctrl} shift={shift}",
            image.width(),
            image.height()
        );
        self.deliver(BusEvent::RegionSnip(RegionSnip {
            image,
            x,
            y,
            ctrl,
            shift,
        }))
    }

    /// Bring the main window to the front.
    async fn show(&self) -> zbus::fdo::Result<()> {
        log::info!("asked over the bus to show the main window");
        self.deliver(BusEvent::Show)
    }

    /// Do what a global shortcut does: "snip", "snip-to-clipboard",
    /// "clipboard-only", "toggle-visibility" or "gallery".
    ///
    /// For the desktops whose portal has no global shortcuts — sway, river and
    /// the other wlroots compositors — where a key is bound in the
    /// compositor's own configuration to `snipdeck --action snip`.
    async fn activate(&self, action: String) -> zbus::fdo::Result<()> {
        let hotkey = crate::hotkeys::HotkeyAction::from_name(&action)
            .ok_or_else(|| zbus::fdo::Error::InvalidArgs(format!("no action {action:?}")))?;
        log::info!("asked over the bus for {action}");
        self.deliver(BusEvent::Hotkey(hotkey))
    }
}

/// Claim the bus name and start serving.
///
/// The receiver comes back at once and the bus is joined on a thread of its
/// own, so start-up does not wait on D-Bus. If that fails — no session bus,
/// the name already taken — the app carries on without the Super+Alt gesture
/// and without being raised by a second launch.
pub fn start() -> Receiver<BusEvent> {
    let (tx, rx) = unbounded();
    let spawned = std::thread::Builder::new()
        .name("bus-service".into())
        .spawn(move || {
            let built = pollster::block_on(async {
                zbus::connection::Builder::session()?
                    .name(BUS_NAME)?
                    .serve_at(OBJECT_PATH, Service { tx })?
                    .build()
                    .await
            });
            match built {
                Ok(connection) => {
                    log::info!("bus service: serving {BUS_NAME}");
                    let _ = CONNECTION.set(connection);
                }
                Err(e) => log::warn!("bus service unavailable ({e})"),
            }
        });
    if let Err(e) = spawned {
        log::warn!("bus service thread not started ({e})");
    }
    rx
}

/// From a second launch: ask the running Snipdeck to show its window. True if
/// it answered. Bounded, so a running instance that has hung cannot hang the
/// launch that is about to exit anyway.
pub fn show_running_instance() -> bool {
    call_running("Show", None)
}

/// Ask the running Snipdeck for a shortcut's action; false if none answered.
pub fn activate_running_instance(action: &str) -> bool {
    call_running("Activate", Some(action.to_string()))
}

fn call_running(method: &'static str, arg: Option<String>) -> bool {
    let (tx, rx) = crossbeam_channel::bounded(1);
    let _ = std::thread::Builder::new()
        .name("bus-call".into())
        .spawn(move || {
            let answered = pollster::block_on(async {
                let conn = zbus::Connection::session().await.ok()?;
                let call = match &arg {
                    Some(a) => {
                        conn.call_method(Some(BUS_NAME), OBJECT_PATH, Some(BUS_INTERFACE), method, &(a,))
                            .await
                    }
                    None => {
                        conn.call_method(Some(BUS_NAME), OBJECT_PATH, Some(BUS_INTERFACE), method, &())
                            .await
                    }
                };
                call.map_err(|e| log::warn!("{method} on the running Snipdeck failed: {e}")).ok()
            })
            .is_some();
            let _ = tx.send(answered);
        });
    rx.recv_timeout(Duration::from_secs(3)).unwrap_or(false)
}
