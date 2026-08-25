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
//! no encode, no disk, no sound. The session is negotiated once and the stream
//! is left running, so a capture is a memcpy of the most recent frame.
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
//! Known cost, stated plainly: while the stream is up, the desktop shows its
//! "screen is being shared" indicator. That is the compositor telling the truth
//! about what this process can see, and suppressing it is neither possible nor
//! desirable. The stream is started on the first capture, not at launch, so an
//! idle Snipdeck does not light it.

use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context as _};
use image::RgbaImage;

/// One frame, already in the layout `image` wants.
struct Frame {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

/// Shared with the PipeWire thread: the most recent frame, or none yet.
type Latest = Arc<Mutex<Option<Frame>>>;

struct Capturer {
    latest: Latest,
    /// Where the stream says the monitor sits, in the capture's own pixels.
    position: (i32, i32),
    /// Kept alive for the process: dropping it tears the session down and the
    /// next capture would have to negotiate — and prompt — all over again.
    _thread: std::thread::JoinHandle<()>,
}

static CAPTURER: OnceLock<Option<Capturer>> = OnceLock::new();

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

/// Is this path worth trying at all?
///
/// Only on a Wayland session. Under X11 xcap talks to the X server directly,
/// which is already fast and does not involve a portal.
pub fn usable() -> bool {
    crate::window_env::caps().backend == crate::window_env::Backend::Wayland
}

/// Grab the current screen.
///
/// The first call negotiates the session — which may show the portal's consent
/// dialog — and waits for the first frame. Later calls copy whatever the
/// compositor last handed us.
pub fn capture() -> anyhow::Result<(i32, i32, RgbaImage)> {
    let cap = CAPTURER
        .get_or_init(|| match start() {
            Ok(c) => Some(c),
            Err(e) => {
                log::warn!("screencast capture unavailable ({e:#}); falling back to xcap");
                None
            }
        })
        .as_ref()
        .ok_or_else(|| anyhow!("screencast session was not established"))?;

    // The stream is free-running, so a frame is normally already waiting. The
    // wait matters only for the very first capture, and after the compositor
    // has had nothing to send (an idle screen produces no new buffers).
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(frame) = cap.latest.lock().unwrap().as_ref() {
            let img = RgbaImage::from_raw(frame.width, frame.height, frame.rgba.clone())
                .ok_or_else(|| anyhow!("frame buffer did not match its own dimensions"))?;
            return Ok((cap.position.0, cap.position.1, img));
        }
        if Instant::now() >= deadline {
            return Err(anyhow!("no frame arrived within 5s"));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Negotiate the portal session and bring the PipeWire stream up.
fn start() -> anyhow::Result<Capturer> {
    let (ready_tx, ready_rx) = crossbeam_channel::bounded::<anyhow::Result<(i32, i32)>>(1);
    let latest: Latest = Arc::new(Mutex::new(None));
    let latest_thread = latest.clone();

    let thread = std::thread::Builder::new()
        .name("screencast".into())
        .spawn(move || {
            if let Err(e) = run(latest_thread, ready_tx.clone()) {
                log::warn!("screencast stream stopped: {e:#}");
                let _ = ready_tx.try_send(Err(e));
            }
        })
        .context("spawning the screencast thread")?;

    // The consent dialog can sit unanswered; do not hang the app behind it.
    let position = match ready_rx.recv_timeout(Duration::from_secs(30)) {
        Ok(Ok(pos)) => pos,
        Ok(Err(e)) => return Err(e),
        Err(_) => return Err(anyhow!("portal did not answer within 30s")),
    };

    Ok(Capturer {
        latest,
        position,
        _thread: thread,
    })
}

/// Thread body: portal handshake, then the PipeWire loop, forever.
fn run(
    latest: Latest,
    ready_tx: crossbeam_channel::Sender<anyhow::Result<(i32, i32)>>,
) -> anyhow::Result<()> {
    let (fd, node_id, position) = pollster::block_on(negotiate())?;
    log::info!("screencast: node {node_id} at {position:?}");
    pipewire_loop(fd, node_id, latest, position, ready_tx)
}

/// The portal half: session, sources, start, and the PipeWire fd.
async fn negotiate() -> anyhow::Result<(std::os::fd::OwnedFd, u32, (i32, i32))> {
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

    // `session` is dropped here, and with ashpd that does NOT close the portal
    // session — closing is explicit. The cast stays up for the fd's lifetime.
    Ok((fd, node_id, position))
}

/// The PipeWire half: connect, negotiate a format, and copy every frame.
fn pipewire_loop(
    fd: std::os::fd::OwnedFd,
    node_id: u32,
    latest: Latest,
    position: (i32, i32),
    ready_tx: crossbeam_channel::Sender<anyhow::Result<(i32, i32)>>,
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

    // Format is filled in by `param_changed` and read by `process`.
    let format = Arc::new(Mutex::new(spa::param::video::VideoInfoRaw::default()));
    let format_cb = format.clone();
    let format_proc = format.clone();
    let ready = Arc::new(Mutex::new(Some(ready_tx)));
    let ready_proc = ready.clone();

    let _listener = stream
        .add_local_listener_with_user_data(())
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
                log::info!(
                    "screencast format: {}x{} {:?}",
                    f.size().width,
                    f.size().height,
                    f.format()
                );
            }
        })
        .process(move |stream, _| {
            let Some(mut buffer) = stream.dequeue_buffer() else {
                return;
            };
            let datas = buffer.datas_mut();
            let Some(data) = datas.first_mut() else { return };
            let info = *format_proc.lock().unwrap();
            let (w, h) = (info.size().width, info.size().height);
            if w == 0 || h == 0 {
                return;
            }
            let chunk_stride = data.chunk().stride().max(0) as usize;
            let Some(src) = data.data() else { return };

            let stride = if chunk_stride > 0 {
                chunk_stride
            } else {
                w as usize * 4
            };
            let mut rgba = vec![0u8; w as usize * h as usize * 4];
            let swap = matches!(
                info.format(),
                spa::param::video::VideoFormat::BGRx | spa::param::video::VideoFormat::BGRA
            );
            for row in 0..h as usize {
                let s = row * stride;
                let d = row * w as usize * 4;
                if s + w as usize * 4 > src.len() {
                    break;
                }
                let srow = &src[s..s + w as usize * 4];
                let drow = &mut rgba[d..d + w as usize * 4];
                if swap {
                    // BGRx/BGRA -> RGBA, and force the alpha byte: the
                    // compositor sends `x` as undefined padding, and leaving it
                    // makes the snip transparent in places.
                    for (px_s, px_d) in srow.chunks_exact(4).zip(drow.chunks_exact_mut(4)) {
                        px_d[0] = px_s[2];
                        px_d[1] = px_s[1];
                        px_d[2] = px_s[0];
                        px_d[3] = 255;
                    }
                } else {
                    for (px_s, px_d) in srow.chunks_exact(4).zip(drow.chunks_exact_mut(4)) {
                        px_d[0] = px_s[0];
                        px_d[1] = px_s[1];
                        px_d[2] = px_s[2];
                        px_d[3] = 255;
                    }
                }
            }

            *latest.lock().unwrap() = Some(Frame {
                width: w,
                height: h,
                rgba,
            });

            // The caller has been waiting on the first frame, not on the
            // session: a session that never produces one is no use.
            if let Some(tx) = ready_proc.lock().unwrap().take() {
                let _ = tx.try_send(Ok(position));
            }
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
            spa::utils::Fraction { num: 30, denom: 1 },
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

    // Runs until the process ends. The session and the stream stay up so a
    // capture is a copy rather than a negotiation.
    mainloop.run();
    Ok(())
}
