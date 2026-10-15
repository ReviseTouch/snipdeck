//! Wayland screen capture over the ScreenCast portal and PipeWire.
//!
//! Why this exists, rather than letting xcap do it: xcap's Wayland path asks
//! GNOME for a screenshot *into a file*. Every capture encodes the whole screen
//! as a PNG — 3840x2400 on this display, because the portal rounds a fractional
//! monitor scale up — writes it to `/tmp`, reads it back and decodes it. That
//! measured five to nine seconds per snip and rang the shutter each time, which
//! is not a snipping tool.
//!
//! ScreenCast hands over the compositor's own buffers through PipeWire instead:
//! no encode, no disk, no sound.
//!
//! The stream is brought up per capture and torn down again. Holding it open
//! would make a capture a memcpy, but it also leaves the desktop's
//! "screen is being shared" indicator lit for as long as Snipdeck is running,
//! which for a tool that sits in the tray all day reads as a program watching
//! you. It is also fragile in a way that is worse than slow: stopping the share
//! from that indicator kills the stream, and a cached last-frame then gets
//! served for every capture afterwards — the screen silently frozen in the
//! past. One session per snip has neither problem.
//!
//! Two things this deliberately does NOT do:
//!
//!   - It does not replace xcap. `capture.rs` prefers this path and falls back,
//!     so a session that cannot be negotiated (no portal, permission refused, a
//!     compositor without ScreenCast) costs nothing but a log line.
//!   - It does not ask per capture. The portal's consent dialog appears once,
//!     and a restore token is persisted so it does not appear again on the next
//!     run. `PersistMode::ExplicitlyRevoked` is what makes the token durable —
//!     the user can still revoke it in their desktop's privacy settings, which
//!     is the only place that decision belongs.
//!
//! The indicator therefore appears for the moment a snip is taken and goes away
//! again, which is the truth: that is exactly when this process can see the
//! screen.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::{anyhow, Context as _};
use image::RgbaImage;

use crate::pipewire_dl::Frame;

/// Path the restore token lives at, so consent survives a restart.
fn token_path() -> std::path::PathBuf {
    crate::paths::data_dir().join("screencast.token")
}

fn read_token() -> Option<String> {
    let t = std::fs::read_to_string(token_path()).ok()?;
    let t = t.trim().to_string();
    (!t.is_empty()).then_some(t)
}

fn write_token(token: &str) {
    let path = token_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Err(e) = std::fs::write(&path, token) {
        log::debug!("could not persist the screencast restore token: {e}");
    }
}

/// How long a whole capture may take: portal handshake, stream, first frame.
/// Every capture after the first is expected to be a small fraction of this.
const CAPTURE_TIMEOUT: Duration = Duration::from_secs(30);

/// The same for the first capture, with no restore token yet: the portal shows
/// its consent dialog and waits for a human, who may well read it first. The
/// caller is a capture thread of its own (see `AppState::freeze_captured`),
/// so waiting holds up nothing else.
const CONSENT_TIMEOUT: Duration = Duration::from_secs(120);

/// Long enough for the portal's consent dialog to finish closing.
const CONSENT_DIALOG_CLOSE: Duration = Duration::from_millis(500);

/// Set for as long as a capture thread is alive.
///
/// A capture that gave up waiting leaves its thread — and the portal's
/// consent dialog — still running. Starting another stacked a second dialog
/// on the first, and another on each further try.
static IN_FLIGHT: AtomicBool = AtomicBool::new(false);

/// Clears `IN_FLIGHT` when the capture thread ends, however it ends.
struct InFlight;

impl Drop for InFlight {
    fn drop(&mut self) {
        IN_FLIGHT.store(false, Ordering::Release);
    }
}

/// How long the PipeWire loop waits for its first frame once the stream is up.
///
/// Deliberately much shorter than `CAPTURE_TIMEOUT`: by the time the stream
/// exists the portal has already answered, so the only thing left to wait for
/// is the compositor sending a buffer. This has to fire *before* the caller
/// gives up, because the caller giving up does not stop this thread — see the
/// timer in `pipewire_dl::capture_frame`.
const FIRST_FRAME_DEADLINE: Duration = Duration::from_secs(10);

/// Is this path worth trying at all?
///
/// Only on a Wayland session — under X11 xcap talks to the X server directly,
/// which is already fast and does not involve a portal — and only where
/// libpipewire can be loaded (see `pipewire_dl`).
pub fn usable() -> bool {
    crate::window_env::caps().backend == crate::window_env::Backend::Wayland
        && crate::pipewire_dl::available()
}

/// Did this capture fail because the user declined the sharing dialog?
pub fn declined_by_user(e: &anyhow::Error) -> bool {
    e.chain().any(|cause| {
        matches!(
            cause.downcast_ref::<ashpd::Error>(),
            Some(ashpd::Error::Response(ashpd::desktop::ResponseError::Cancelled))
        )
    })
}

/// Grab the current screen.
///
/// Brings a ScreenCast session up, takes the first frame the compositor sends,
/// and tears everything down again. The first call may show the portal's
/// consent dialog; after that the restore token answers it.
pub fn capture() -> anyhow::Result<(i32, i32, RgbaImage)> {
    if IN_FLIGHT.swap(true, Ordering::AcqRel) {
        return Err(anyhow!(
            "still waiting for the screen-sharing dialog from the last attempt"
        ));
    }
    let guard = InFlight;
    let timeout = if read_token().is_some() {
        CAPTURE_TIMEOUT
    } else {
        CONSENT_TIMEOUT
    };
    let (tx, rx) = crossbeam_channel::bounded::<anyhow::Result<(i32, i32, Frame)>>(1);

    std::thread::Builder::new()
        .name("screencast".into())
        .spawn(move || {
            let _guard = guard;
            let outcome = one_shot(&tx);
            // Only lands if the loop fell over before sending anything.
            if let Err(e) = outcome {
                let _ = tx.try_send(Err(e));
            }
        })
        .context("spawning the screencast thread")?;

    let (x, y, frame) = rx
        .recv_timeout(timeout)
        .map_err(|_| anyhow!("the screencast portal did not answer in time"))??;

    let img = RgbaImage::from_raw(frame.width, frame.height, frame.rgba)
        .ok_or_else(|| anyhow!("frame buffer did not match its own dimensions"))?;
    Ok((x, y, img))
}

/// Thread body: portal handshake, one frame, done.
fn one_shot(tx: &crossbeam_channel::Sender<anyhow::Result<(i32, i32, Frame)>>) -> anyhow::Result<()> {
    let consent_asked = read_token().is_none();
    let (session, fd, node_id, position) = pollster::block_on(negotiate())?;
    log::debug!("screencast: node {node_id} at {position:?}");
    if consent_asked {
        // The consent dialog has only just been answered and is still on its
        // way out; a stream started now sends a first frame with the dialog in
        // it, and the very first snip after granting access was a picture of
        // the permission prompt. Started once it has gone, the stream's first
        // frame is the screen.
        std::thread::sleep(CONSENT_DIALOG_CLOSE);
    }
    let outcome = crate::pipewire_dl::capture_frame(fd, node_id, FIRST_FRAME_DEADLINE)
        .map(|frame| {
            let _ = tx.try_send(Ok((position.0, position.1, frame)));
        });

    // Close it, always. Dropping an ashpd `Session` does not end the portal
    // session — closing is an explicit call — and an unclosed cast is what
    // keeps the desktop's screen-sharing indicator lit long after the snip is
    // taken. Every capture opened one and left it, so they accumulated.
    if let Err(e) = pollster::block_on(session.close()) {
        log::debug!("could not close the screencast session: {e}");
    }

    outcome
}

/// The portal half: session, sources, start, and the PipeWire fd.
type CastSession = ashpd::desktop::Session<ashpd::desktop::screencast::Screencast>;

async fn negotiate() -> anyhow::Result<(CastSession, std::os::fd::OwnedFd, u32, (i32, i32))> {
    use ashpd::desktop::screencast::Screencast;

    crate::portal_hotkeys::register_app_id().await;
    let proxy = Screencast::new().await.context("opening the ScreenCast portal")?;
    let session = proxy
        .create_session(Default::default())
        .await
        .context("creating a screencast session")?;

    // Once the session exists every way out closes it. A source selection or
    // start that failed — a refused dialog included — used to drop it open,
    // and a portal session left open is a screen cast the desktop still
    // shows as running.
    match start_cast(&proxy, &session).await {
        // The session is handed back rather than dropped: it has to outlive
        // the stream, and then be closed explicitly. See `one_shot`.
        Ok((fd, node_id, position)) => Ok((session, fd, node_id, position)),
        Err(e) => {
            if let Err(close) = session.close().await {
                log::debug!("could not close the failed screencast session: {close}");
            }
            Err(e)
        }
    }
}

/// Sources, start, and the PipeWire fd, on a session already created.
async fn start_cast(
    proxy: &ashpd::desktop::screencast::Screencast,
    session: &CastSession,
) -> anyhow::Result<(std::os::fd::OwnedFd, u32, (i32, i32))> {
    use ashpd::desktop::{
        screencast::{CursorMode, SelectSourcesOptions, SourceType},
        PersistMode,
    };

    let token = read_token();
    // Spelled out rather than `.into()`: the setter takes an
    // `Into<Option<BitFlags<_>>>`, so a bare conversion is ambiguous between
    // the flags and the Option.
    let sources: ashpd::enumflags2::BitFlags<SourceType> = SourceType::Monitor.into();
    let mut options = SelectSourcesOptions::default()
        // The pointer is drawn by the app's own overlay; a second one baked
        // into the frame would be visibly wrong in the snip.
        .set_cursor_mode(CursorMode::Hidden)
        .set_sources(sources)
        .set_multiple(false)
        // Durable consent. Without it the dialog returns on every launch,
        // which for a tool reached by a hotkey is the same as not working.
        .set_persist_mode(PersistMode::ExplicitlyRevoked);
    if let Some(t) = token.as_deref() {
        options = options.set_restore_token(t);
    }

    proxy
        .select_sources(session, options)
        .await
        .context("selecting the monitor to cast")?;

    let response = proxy
        .start(session, None, Default::default())
        .await
        .context("starting the cast")?
        .response()
        .context("the user or the compositor refused the cast")?;

    if let Some(t) = response.restore_token() {
        write_token(t);
    }

    let stream = response
        .streams()
        .first()
        .ok_or_else(|| anyhow!("the portal started a cast with no streams"))?;
    let node_id = stream.pipe_wire_node_id();
    let position = stream.position().unwrap_or((0, 0));

    let fd = proxy
        .open_pipe_wire_remote(session, Default::default())
        .await
        .context("opening the PipeWire remote")?;

    Ok((fd, node_id, position))
}
