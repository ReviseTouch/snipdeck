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
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{anyhow, Context as _};
use image::RgbaImage;

/// One frame, already in the layout `image` wants.
struct Frame {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

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
/// timer in `pipewire_once`.
const FIRST_FRAME_DEADLINE: Duration = Duration::from_secs(10);

/// Is this path worth trying at all?
///
/// Only on a Wayland session. Under X11 xcap talks to the X server directly,
/// which is already fast and does not involve a portal.
pub fn usable() -> bool {
    crate::window_env::caps().backend == crate::window_env::Backend::Wayland
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
    let (session, fd, node_id, position) = pollster::block_on(negotiate())?;
    log::debug!("screencast: node {node_id} at {position:?}");
    let outcome = pipewire_once(fd, node_id, position, tx);

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
    use ashpd::desktop::{
        screencast::{CursorMode, Screencast, SelectSourcesOptions, SourceType},
        PersistMode,
    };

    let proxy = Screencast::new().await.context("opening the ScreenCast portal")?;
    let session = proxy
        .create_session(Default::default())
        .await
        .context("creating a screencast session")?;

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
        .select_sources(&session, options)
        .await
        .context("selecting the monitor to cast")?;

    let response = proxy
        .start(&session, None, Default::default())
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
        .open_pipe_wire_remote(&session, Default::default())
        .await
        .context("opening the PipeWire remote")?;

    // The session is handed back rather than dropped: it has to outlive the
    // stream, and then be closed explicitly. See `one_shot`.
    Ok((session, fd, node_id, position))
}

/// The PipeWire half: connect, negotiate a format, take one frame, quit.
fn pipewire_once(
    fd: std::os::fd::OwnedFd,
    node_id: u32,
    position: (i32, i32),
    tx: &crossbeam_channel::Sender<anyhow::Result<(i32, i32, Frame)>>,
) -> anyhow::Result<()> {
    use pipewire as pw;
    use pw::spa;

    pw::init();

    let mainloop = pw::main_loop::MainLoopRc::new(None).context("pipewire main loop")?;
    let context = pw::context::ContextRc::new(&mainloop, None).context("pipewire context")?;
    let core = context
        .connect_fd_rc(fd, None)
        .context("connecting to the PipeWire remote")?;

    let stream = pw::stream::StreamRc::new(
        core,
        "snipdeck-capture",
        pw::properties::properties! {
            *pw::keys::MEDIA_TYPE => "Video",
            *pw::keys::MEDIA_CATEGORY => "Capture",
            *pw::keys::MEDIA_ROLE => "Screen",
        },
    )
    .context("creating the capture stream")?;

    // Filled in by `param_changed`, read by `process`.
    let format = Arc::new(Mutex::new(spa::param::video::VideoInfoRaw::default()));
    let format_cb = format.clone();
    let sender = Arc::new(Mutex::new(Some(tx.clone())));
    let quit = mainloop.clone();
    let sender_state = sender.clone();
    let quit_state = mainloop.clone();
    // Cloned here rather than at the timer below: `process` takes `sender` by
    // move when the listener is registered.
    let sender_timer = sender.clone();

    let _listener = stream
        .add_local_listener_with_user_data(())
        .state_changed(move |_, _, old, new| {
            log::debug!("screencast stream state: {old:?} -> {new:?}");
            // A stream that has errored will never produce a frame. Waiting out
            // the caller's timeout after that is not patience, it is a hang:
            // thirty seconds of an application that looks frozen, and then a
            // fallback that could have run immediately.
            if let pw::stream::StreamState::Error(msg) = &new {
                if let Some(tx) = sender_state.lock().unwrap().take() {
                    let _ = tx.try_send(Err(anyhow!("the capture stream failed: {msg}")));
                }
                quit_state.quit();
            }
        })
        .param_changed(move |_, _, id, param| {
            let Some(param) = param else { return };
            if id != spa::param::ParamType::Format.as_raw() {
                return;
            }
            let Ok((media_type, media_subtype)) = spa::param::format_utils::parse_format(param)
            else {
                return;
            };
            if media_type != spa::param::format::MediaType::Video
                || media_subtype != spa::param::format::MediaSubtype::Raw
            {
                return;
            }
            let mut f = format_cb.lock().unwrap();
            if f.parse(param).is_ok() {
                log::debug!(
                    "screencast format: {}x{} {:?}",
                    f.size().width,
                    f.size().height,
                    f.format()
                );
            }
        })
        .process(move |stream, _| {
            // Nothing left to answer: the frame has been sent and the loop is
            // on its way out. Buffers still arriving are drained and dropped.
            if sender.lock().unwrap().is_none() {
                let _ = stream.dequeue_buffer();
                return;
            }
            let Some(mut buffer) = stream.dequeue_buffer() else {
                return;
            };
            let datas = buffer.datas_mut();
            let Some(data) = datas.first_mut() else { return };
            let info = *format.lock().unwrap();
            let (w, h) = (info.size().width, info.size().height);
            if w == 0 || h == 0 {
                return;
            }
            let chunk_stride = data.chunk().stride().max(0) as usize;
            // A zero-size chunk is the compositor saying "nothing new"; it is
            // not a frame, and treating it as one is how a capture comes back
            // blank.
            if data.chunk().size() == 0 {
                return;
            }
            let Some(src) = data.data() else { return };

            let stride = if chunk_stride > 0 {
                chunk_stride
            } else {
                w as usize * 4
            };
            let row_bytes = w as usize * 4;
            let mut rgba = vec![0u8; row_bytes * h as usize];
            let swap = matches!(
                info.format(),
                spa::param::video::VideoFormat::BGRx | spa::param::video::VideoFormat::BGRA
            );
            for row in 0..h as usize {
                let s = row * stride;
                if s + row_bytes > src.len() {
                    break;
                }
                let srow = &src[s..s + row_bytes];
                let drow = &mut rgba[row * row_bytes..(row + 1) * row_bytes];
                for (px_s, px_d) in srow.chunks_exact(4).zip(drow.chunks_exact_mut(4)) {
                    if swap {
                        // BGRx/BGRA -> RGBA.
                        px_d[0] = px_s[2];
                        px_d[1] = px_s[1];
                        px_d[2] = px_s[0];
                    } else {
                        px_d[0] = px_s[0];
                        px_d[1] = px_s[1];
                        px_d[2] = px_s[2];
                    }
                    // Force alpha: `x` is undefined padding, and carrying it
                    // through leaves the snip transparent in patches.
                    px_d[3] = 255;
                }
            }

            if let Some(tx) = sender.lock().unwrap().take() {
                let _ = tx.try_send(Ok((
                    position.0,
                    position.1,
                    Frame {
                        width: w,
                        height: h,
                        rgba,
                    },
                )));
            }
            // One frame is all this session was for.
            quit.quit();
        })
        .register()
        .context("registering the stream listener")?;

    let obj = spa::pod::object!(
        spa::utils::SpaTypes::ObjectParamFormat,
        spa::param::ParamType::EnumFormat,
        spa::pod::property!(
            spa::param::format::FormatProperties::MediaType,
            Id,
            spa::param::format::MediaType::Video
        ),
        spa::pod::property!(
            spa::param::format::FormatProperties::MediaSubtype,
            Id,
            spa::param::format::MediaSubtype::Raw
        ),
        spa::pod::property!(
            spa::param::format::FormatProperties::VideoFormat,
            Choice,
            Enum,
            Id,
            spa::param::video::VideoFormat::BGRx,
            spa::param::video::VideoFormat::RGBx,
            spa::param::video::VideoFormat::BGRA,
            spa::param::video::VideoFormat::RGBA
        ),
        spa::pod::property!(
            spa::param::format::FormatProperties::VideoSize,
            Choice,
            Range,
            Rectangle,
            spa::utils::Rectangle {
                width: 1920,
                height: 1080
            },
            spa::utils::Rectangle {
                width: 1,
                height: 1
            },
            spa::utils::Rectangle {
                width: 8192,
                height: 8192
            }
        ),
        spa::pod::property!(
            spa::param::format::FormatProperties::VideoFramerate,
            Choice,
            Range,
            Fraction,
            spa::utils::Fraction { num: 60, denom: 1 },
            // Minimum 0/1, NOT 1/1. A screencast source advertises a variable
            // rate — it sends on damage, not on a clock — and demanding at
            // least one frame a second leaves no format in common with it:
            // the stream goes straight to Error("no more input formats").
            spa::utils::Fraction { num: 0, denom: 1 },
            spa::utils::Fraction { num: 60, denom: 1 }
        ),
    );

    let values: Vec<u8> = spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &spa::pod::Value::Object(obj),
    )
    .context("serialising the format request")?
    .0
    .into_inner();
    let mut params = [spa::pod::Pod::from_bytes(&values)
        .ok_or_else(|| anyhow!("format pod did not round-trip"))?];

    stream
        .connect(
            spa::utils::Direction::Input,
            Some(node_id),
            pw::stream::StreamFlags::AUTOCONNECT | pw::stream::StreamFlags::MAP_BUFFERS,
            &mut params,
        )
        .context("connecting the capture stream")?;

    // A deadline INSIDE the loop, not merely around it.
    //
    // `mainloop.run()` returns when `process` has a frame or `state_changed`
    // reports an error. A stream that does neither leaves this thread running
    // after `capture()` has already given up on it — and a screencast source
    // sends on damage rather than on a clock, so "neither" is a real state, not
    // a hypothetical one. The cost is not the idle thread: `one_shot` never
    // reaches its `session.close()`, so the cast stays open and the desktop's
    // screen-sharing indicator stays lit, and every retry stacks another one.
    // That is exactly the accumulation the per-snip design exists to prevent,
    // and without this timer it survived in the one path nobody watches.
    let quit_timer = mainloop.clone();
    let timer = mainloop.loop_().add_timer(move |_| {
        if let Some(tx) = sender_timer.lock().unwrap().take() {
            let _ = tx.try_send(Err(anyhow!(
                "the compositor sent no frame within {}s",
                FIRST_FRAME_DEADLINE.as_secs()
            )));
        }
        quit_timer.quit();
    });
    timer
        .update_timer(Some(FIRST_FRAME_DEADLINE), None)
        .into_result()
        .map_err(|e| anyhow!("arming the first-frame deadline: {e:?}"))?;

    // Returns once `process` has a frame, the stream reports an error, or the
    // timer above fires. Every one of those paths unwinds to `one_shot`, which
    // closes the portal session.
    mainloop.run();
    Ok(())
}
