//! The GNOME extension's way in: snips taken with Super+Alt+drag.
//!
//! On Windows a snip starts from a global mouse hook — hold Win, drag. Wayland
//! grants no client such a hook, and GNOME would not let a Super+drag through
//! anyway: Super is Mutter's window-move modifier, so the press becomes a
//! window drag before any client hears of it. The extension can see the press
//! because it runs inside the compositor. It draws the selection, takes the
//! pixels with the shell's own screenshot code — no portal, no "sharing your
//! screen" indicator, no shutter — and hands the finished region over here.
//!
//! What crosses the bus is a PNG, its top-left corner in physical screen pixels
//! (the space the rest of the app captures in) and the two modifiers that pick
//! the routing. From there it is an ordinary snip: see
//! `AppState::handle_region_snip`.
//!
//! Owning the bus name doubles as the extension's "is Snipdeck running" signal.
//! It only claims Super+Alt while this name has an owner, so with the app
//! closed the chord goes back to being a window move. The other end is
//! `SnipGesture` in `packaging/gnome-extension/snipdeck@revisetouch.com`.

use std::sync::OnceLock;

use crossbeam_channel::{unbounded, Receiver, Sender};

/// Must match `BUS_NAME`, `OBJECT_PATH` and `BUS_INTERFACE` in the extension.
const BUS_NAME: &str = "com.revisetouch.Snipdeck";
const OBJECT_PATH: &str = "/com/revisetouch/Snipdeck";

/// Held for the life of the process: dropping it releases the name, and the
/// extension would stop claiming Super+Alt.
static CONNECTION: OnceLock<zbus::Connection> = OnceLock::new();

/// A region the extension selected and captured.
pub struct RegionSnip {
    pub image: image::RgbaImage,
    /// Top-left corner, in physical screen pixels.
    pub x: i32,
    pub y: i32,
    pub ctrl: bool,
    pub shift: bool,
}

struct Bridge {
    tx: Sender<RegionSnip>,
}

#[zbus::interface(name = "com.revisetouch.Snipdeck1")]
impl Bridge {
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
        self.tx
            .send(RegionSnip {
                image,
                x,
                y,
                ctrl,
                shift,
            })
            .map_err(|_| zbus::fdo::Error::Failed("Snipdeck is shutting down".into()))?;
        crate::pump::wake();
        Ok(())
    }
}

/// Claim the bus name and start serving.
///
/// `None` off GNOME, where no extension will ever call. On GNOME the receiver
/// comes back at once and the bus is joined on a thread of its own, so start-up
/// does not wait on D-Bus; if that fails — no session bus, the name already
/// taken — the app carries on without the gesture, which is also how it runs
/// when the extension is not installed.
pub fn start() -> Option<Receiver<RegionSnip>> {
    if crate::window_env::caps().compositor != crate::window_env::Compositor::Gnome {
        return None;
    }

    let (tx, rx) = unbounded();
    let spawned = std::thread::Builder::new()
        .name("gnome-bridge".into())
        .spawn(move || {
            let built = pollster::block_on(async {
                zbus::connection::Builder::session()?
                    .name(BUS_NAME)?
                    .serve_at(OBJECT_PATH, Bridge { tx })?
                    .build()
                    .await
            });
            match built {
                Ok(connection) => {
                    log::info!("gnome bridge: serving {BUS_NAME}");
                    let _ = CONNECTION.set(connection);
                }
                Err(e) => {
                    log::warn!("gnome bridge unavailable ({e}); Super+Alt+drag will not snip")
                }
            }
        });
    if let Err(e) = spawned {
        log::warn!("gnome bridge thread not started ({e}); Super+Alt+drag will not snip");
        return None;
    }
    Some(rx)
}
