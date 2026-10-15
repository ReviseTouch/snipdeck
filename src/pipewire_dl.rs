//! PipeWire, loaded when it is first needed rather than linked.
//!
//! The ScreenCast capture needs very little of PipeWire: connect to the remote
//! the portal hands over, ask for one raw video stream, take one frame. The
//! `pipewire` crate it used to go through links libpipewire at build time and
//! generates its bindings from the build machine's SPA headers — and the
//! libspa it depends on uses SPA additions newer than PipeWire 0.3.48, the
//! version on Ubuntu 22.04, which the Linux release is built on for its old
//! glibc floor. The release build had not passed since PipeWire came in.
//!
//! Loaded at run time, `libpipewire-0.3.so.0` needs nothing at build time and
//! is whatever the user's system has, 0.3 or 1.x; a system without it loses
//! only the fast capture path (`wayland_screencast::usable`), not the app.
//!
//! Declared here is only what this file uses, all of it from the stable ABI:
//! exported `pw_*` functions, the leading fields of the structs the API hands
//! out, the loop-utils method table, and SPA's POD encoding, which is a
//! documented wire format.

use std::ffi::{c_char, c_int, c_void};
use std::os::fd::{IntoRawFd, OwnedFd};
use std::ptr::{null, null_mut};
use std::sync::OnceLock;
use std::time::Duration;

use anyhow::{anyhow, bail};

/// One frame, RGBA, tightly packed.
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Whether libpipewire could be loaded. Loads it on the first call.
pub fn available() -> bool {
    lib().is_some()
}

/// Take one frame from the PipeWire node `node_id` on the remote `fd` (from
/// the ScreenCast portal). Fails if the stream errors, or no frame arrives
/// within `deadline`.
pub fn capture_frame(fd: OwnedFd, node_id: u32, deadline: Duration) -> anyhow::Result<Frame> {
    let lib = lib().ok_or_else(|| anyhow!("PipeWire is not available on this system"))?;
    // SAFETY: every pointer below comes from libpipewire and is used on this
    // thread only, within the lifetime `Session` gives it; the callbacks get
    // `State`, boxed so it stays put until the stream is gone.
    unsafe {
        let mut s = Session::new(lib);
        s.main_loop = (lib.main_loop_new)(null());
        if s.main_loop.is_null() {
            bail!("creating the PipeWire main loop failed");
        }
        let pw_loop = (lib.main_loop_get_loop)(s.main_loop);
        s.context = (lib.context_new)(pw_loop, null_mut(), 0);
        if s.context.is_null() {
            bail!("creating the PipeWire context failed");
        }
        // Takes the fd, and closes it on disconnect or failure.
        s.core = (lib.context_connect_fd)(s.context, fd.into_raw_fd(), null_mut(), 0);
        if s.core.is_null() {
            bail!("connecting to the PipeWire remote failed");
        }
        let props = (lib.properties_new_string)(
            c"media.type=Video media.category=Capture media.role=Screen".as_ptr(),
        );
        s.stream = (lib.stream_new)(s.core, c"snipdeck-capture".as_ptr(), props);
        if s.stream.is_null() {
            bail!("creating the capture stream failed");
        }

        s.state.main_loop = s.main_loop;
        s.state.stream = s.stream;
        let state: *mut State = &mut *s.state;
        (lib.stream_add_listener)(s.stream, &mut *s.hook, &STREAM_EVENTS, state.cast());

        let pod = enum_format_pod();
        let mut params = [pod.as_ptr().cast::<c_void>()];
        let rc = (lib.stream_connect)(
            s.stream,
            SPA_DIRECTION_INPUT,
            node_id,
            PW_STREAM_FLAG_AUTOCONNECT | PW_STREAM_FLAG_MAP_BUFFERS,
            params.as_mut_ptr(),
            params.len() as u32,
        );
        if rc < 0 {
            bail!("connecting the capture stream failed ({rc})");
        }

        // A deadline inside the loop, not merely around it. A screencast
        // source sends on damage, not on a clock, so a stream that neither
        // delivers nor fails is a real state; without this the loop would run
        // on after the caller gave up, the portal session would never be
        // closed, and the desktop's screen-sharing indicator would stay lit.
        s.arm_timer(pw_loop, deadline)?;

        // Returns once `process` has a frame, the stream reports an error, or
        // the timer fires — each of them records an outcome and quits.
        (lib.main_loop_run)(s.main_loop);
        s.state
            .outcome
            .take()
            .unwrap_or_else(|| Err(anyhow!("the PipeWire loop ended without a frame")))
    }
}

// ---- The library -----------------------------------------------------------

struct Lib {
    main_loop_new: unsafe extern "C" fn(*const c_void) -> *mut c_void,
    main_loop_get_loop: unsafe extern "C" fn(*mut c_void) -> *mut PwLoop,
    main_loop_run: unsafe extern "C" fn(*mut c_void) -> c_int,
    main_loop_quit: unsafe extern "C" fn(*mut c_void) -> c_int,
    main_loop_destroy: unsafe extern "C" fn(*mut c_void),
    context_new: unsafe extern "C" fn(*mut PwLoop, *mut c_void, usize) -> *mut c_void,
    context_destroy: unsafe extern "C" fn(*mut c_void),
    context_connect_fd: unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, usize) -> *mut c_void,
    core_disconnect: unsafe extern "C" fn(*mut c_void) -> c_int,
    properties_new_string: unsafe extern "C" fn(*const c_char) -> *mut c_void,
    stream_new: unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_void) -> *mut c_void,
    stream_add_listener:
        unsafe extern "C" fn(*mut c_void, *mut SpaHook, *const StreamEvents, *mut c_void),
    stream_connect:
        unsafe extern "C" fn(*mut c_void, u32, u32, u32, *mut *const c_void, u32) -> c_int,
    stream_update_params: unsafe extern "C" fn(*mut c_void, *mut *const c_void, u32) -> c_int,
    stream_dequeue_buffer: unsafe extern "C" fn(*mut c_void) -> *mut PwBuffer,
    stream_queue_buffer: unsafe extern "C" fn(*mut c_void, *mut PwBuffer) -> c_int,
    stream_destroy: unsafe extern "C" fn(*mut c_void),
    /// Kept loaded for the life of the process: the pointers above are into it.
    _lib: libloading::Library,
}

fn lib() -> Option<&'static Lib> {
    static LIB: OnceLock<Option<Lib>> = OnceLock::new();
    LIB.get_or_init(|| match unsafe { Lib::load() } {
        Ok(lib) => Some(lib),
        Err(e) => {
            log::info!("PipeWire not loaded ({e}); screen capture takes the slower path");
            None
        }
    })
    .as_ref()
}

impl Lib {
    unsafe fn load() -> Result<Lib, libloading::Error> {
        let lib = libloading::Library::new("libpipewire-0.3.so.0")?;
        macro_rules! sym {
            ($name:literal) => {
                *lib.get(concat!($name, "\0").as_bytes())?
            };
        }
        let init: unsafe extern "C" fn(*mut c_int, *mut *mut *mut c_char) = sym!("pw_init");
        init(null_mut(), null_mut());
        Ok(Lib {
            main_loop_new: sym!("pw_main_loop_new"),
            main_loop_get_loop: sym!("pw_main_loop_get_loop"),
            main_loop_run: sym!("pw_main_loop_run"),
            main_loop_quit: sym!("pw_main_loop_quit"),
            main_loop_destroy: sym!("pw_main_loop_destroy"),
            context_new: sym!("pw_context_new"),
            context_destroy: sym!("pw_context_destroy"),
            context_connect_fd: sym!("pw_context_connect_fd"),
            core_disconnect: sym!("pw_core_disconnect"),
            properties_new_string: sym!("pw_properties_new_string"),
            stream_new: sym!("pw_stream_new"),
            stream_add_listener: sym!("pw_stream_add_listener"),
            stream_connect: sym!("pw_stream_connect"),
            stream_update_params: sym!("pw_stream_update_params"),
            stream_dequeue_buffer: sym!("pw_stream_dequeue_buffer"),
            stream_queue_buffer: sym!("pw_stream_queue_buffer"),
            stream_destroy: sym!("pw_stream_destroy"),
            _lib: lib,
        })
    }
}

// ---- ABI -------------------------------------------------------------------

const SPA_DIRECTION_INPUT: u32 = 0;
const PW_STREAM_FLAG_AUTOCONNECT: u32 = 1 << 0;
const PW_STREAM_FLAG_MAP_BUFFERS: u32 = 1 << 2;
const PW_STREAM_STATE_ERROR: c_int = -1;

#[repr(C)]
struct SpaList {
    next: *mut SpaList,
    prev: *mut SpaList,
}

#[repr(C)]
struct SpaCallbacks {
    funcs: *const c_void,
    data: *mut c_void,
}

/// Storage for a listener registration; libpipewire fills it in.
#[repr(C)]
struct SpaHook {
    link: SpaList,
    cb: SpaCallbacks,
    removed: *const c_void,
    priv_: *mut c_void,
}

#[repr(C)]
struct SpaInterface {
    type_: *const c_char,
    version: u32,
    cb: SpaCallbacks,
}

/// `struct pw_loop`, leading fields.
#[repr(C)]
struct PwLoop {
    system: *mut c_void,
    loop_: *mut c_void,
    control: *mut c_void,
    utils: *mut SpaInterface,
}

type TimerFn = unsafe extern "C" fn(data: *mut c_void, expirations: u64);

/// `struct spa_loop_utils_methods`, up to the last method used.
#[repr(C)]
struct LoopUtilsMethods {
    version: u32,
    add_io: *const c_void,
    update_io: *const c_void,
    add_idle: *const c_void,
    enable_idle: *const c_void,
    add_event: *const c_void,
    signal_event: *const c_void,
    add_timer: Option<unsafe extern "C" fn(*mut c_void, Option<TimerFn>, *mut c_void) -> *mut c_void>,
    update_timer: Option<
        unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            *mut libc::timespec,
            *mut libc::timespec,
            bool,
        ) -> c_int,
    >,
    add_signal: *const c_void,
    destroy_source: Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
}

/// `struct pw_stream_events`, version 0: the callbacks up to `drained`.
#[repr(C)]
struct StreamEvents {
    version: u32,
    destroy: Option<unsafe extern "C" fn(*mut c_void)>,
    state_changed: Option<unsafe extern "C" fn(*mut c_void, c_int, c_int, *const c_char)>,
    control_info: Option<unsafe extern "C" fn()>,
    io_changed: Option<unsafe extern "C" fn()>,
    param_changed: Option<unsafe extern "C" fn(*mut c_void, u32, *const SpaPod)>,
    add_buffer: Option<unsafe extern "C" fn()>,
    remove_buffer: Option<unsafe extern "C" fn()>,
    process: Option<unsafe extern "C" fn(*mut c_void)>,
    drained: Option<unsafe extern "C" fn()>,
}

static STREAM_EVENTS: StreamEvents = StreamEvents {
    version: 0,
    destroy: None,
    state_changed: Some(on_state_changed),
    control_info: None,
    io_changed: None,
    param_changed: Some(on_param_changed),
    add_buffer: None,
    remove_buffer: None,
    process: Some(on_process),
    drained: None,
};

/// `struct pw_buffer`, leading field.
#[repr(C)]
struct PwBuffer {
    buffer: *mut SpaBuffer,
}

#[repr(C)]
struct SpaBuffer {
    n_metas: u32,
    n_datas: u32,
    metas: *mut c_void,
    datas: *mut SpaData,
}

#[repr(C)]
struct SpaData {
    type_: u32,
    flags: u32,
    fd: i64,
    mapoffset: u32,
    maxsize: u32,
    data: *mut c_void,
    chunk: *mut SpaChunk,
}

#[repr(C)]
struct SpaChunk {
    offset: u32,
    size: u32,
    stride: i32,
    flags: i32,
}

#[repr(C)]
struct SpaPod {
    size: u32,
    type_: u32,
}

// ---- One capture -----------------------------------------------------------

/// What the callbacks share, on the loop's own thread.
struct State {
    lib: &'static Lib,
    main_loop: *mut c_void,
    stream: *mut c_void,
    format: Option<VideoFormat>,
    /// The first outcome wins; later callbacks leave it be.
    outcome: Option<anyhow::Result<Frame>>,
}

impl State {
    unsafe fn finish(&mut self, outcome: anyhow::Result<Frame>) {
        if self.outcome.is_none() {
            self.outcome = Some(outcome);
        }
        (self.lib.main_loop_quit)(self.main_loop);
    }
}

/// Everything a capture creates, torn down in reverse on drop, whichever way
/// the capture ends.
struct Session {
    lib: &'static Lib,
    main_loop: *mut c_void,
    context: *mut c_void,
    core: *mut c_void,
    stream: *mut c_void,
    timer: Option<(*const LoopUtilsMethods, *mut c_void, *mut c_void)>,
    hook: Box<SpaHook>,
    state: Box<State>,
}

impl Session {
    fn new(lib: &'static Lib) -> Session {
        Session {
            lib,
            main_loop: null_mut(),
            context: null_mut(),
            core: null_mut(),
            stream: null_mut(),
            timer: None,
            // SAFETY: a hook is plain pointers, all null before registration.
            hook: Box::new(unsafe { std::mem::zeroed() }),
            state: Box::new(State {
                lib,
                main_loop: null_mut(),
                stream: null_mut(),
                format: None,
                outcome: None,
            }),
        }
    }

    /// The loop-utils `add_timer`/`update_timer`, called through the method
    /// table as SPA's inline wrappers do.
    unsafe fn arm_timer(&mut self, pw_loop: *mut PwLoop, after: Duration) -> anyhow::Result<()> {
        let utils = (*pw_loop).utils;
        if utils.is_null() {
            bail!("the PipeWire loop has no utils");
        }
        let methods = (*utils).cb.funcs.cast::<LoopUtilsMethods>();
        let object = (*utils).cb.data;
        let (Some(add_timer), Some(update_timer)) = ((*methods).add_timer, (*methods).update_timer)
        else {
            bail!("the PipeWire loop cannot make timers");
        };
        let state: *mut State = &mut *self.state;
        let source = add_timer(object, Some(on_timer), state.cast());
        if source.is_null() {
            bail!("adding the first-frame timer failed");
        }
        self.timer = Some((methods, object, source));
        let mut value = libc::timespec {
            tv_sec: after.as_secs() as libc::time_t,
            tv_nsec: after.subsec_nanos() as libc::c_long,
        };
        if update_timer(object, source, &mut value, null_mut(), false) < 0 {
            bail!("arming the first-frame timer failed");
        }
        Ok(())
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // SAFETY: each pointer is null or the live object it was created as.
        unsafe {
            if let Some((methods, object, source)) = self.timer.take() {
                if let Some(destroy_source) = (*methods).destroy_source {
                    destroy_source(object, source);
                }
            }
            if !self.stream.is_null() {
                (self.lib.stream_destroy)(self.stream);
            }
            if !self.core.is_null() {
                (self.lib.core_disconnect)(self.core);
            }
            if !self.context.is_null() {
                (self.lib.context_destroy)(self.context);
            }
            if !self.main_loop.is_null() {
                (self.lib.main_loop_destroy)(self.main_loop);
            }
        }
    }
}

unsafe extern "C" fn on_state_changed(
    data: *mut c_void,
    _old: c_int,
    new: c_int,
    error: *const c_char,
) {
    let state = &mut *data.cast::<State>();
    log::debug!("screencast stream state -> {new}");
    // A stream that has errored will never produce a frame; waiting out the
    // deadline after that would only look like a hang.
    if new == PW_STREAM_STATE_ERROR {
        let msg = if error.is_null() {
            "unknown error".into()
        } else {
            std::ffi::CStr::from_ptr(error).to_string_lossy().into_owned()
        };
        state.finish(Err(anyhow!("the capture stream failed: {msg}")));
    }
}

unsafe extern "C" fn on_param_changed(data: *mut c_void, id: u32, param: *const SpaPod) {
    if id != SPA_PARAM_FORMAT || param.is_null() {
        return;
    }
    let state = &mut *data.cast::<State>();
    let len = 8 + (*param).size as usize;
    let bytes = std::slice::from_raw_parts(param.cast::<u8>(), len);
    if let Some(format) = parse_video_format(bytes) {
        log::debug!("screencast format: {}x{} format {}", format.width, format.height, format.format);
        state.format = Some(format);
        // Buffers in memory, not on the GPU. Left unsaid, the wlroots portal
        // (sway) handed over DMA-BUFs, mapped as if they were plain memory,
        // and reading the first row killed the app with SIGSEGV. GNOME and
        // KDE happened to choose memory on their own.
        let pod = buffers_pod();
        let mut params = [pod.as_ptr().cast::<c_void>()];
        let rc = (state.lib.stream_update_params)(state.stream, params.as_mut_ptr(), 1);
        if rc < 0 {
            log::debug!("screencast: asking for memory buffers failed ({rc})");
        }
    }
}

unsafe extern "C" fn on_process(data: *mut c_void) {
    let state = &mut *data.cast::<State>();
    let stream = state.stream;
    let buffer = (state.lib.stream_dequeue_buffer)(stream);
    if buffer.is_null() {
        return;
    }
    // Every dequeued buffer goes back, frame taken or not.
    let frame = if state.outcome.is_none() {
        read_frame(&*(*buffer).buffer, state.format)
    } else {
        None
    };
    (state.lib.stream_queue_buffer)(stream, buffer);
    if let Some(frame) = frame {
        // One frame is all this session was for.
        state.finish(Ok(frame));
    }
}

unsafe extern "C" fn on_timer(data: *mut c_void, _expirations: u64) {
    let state = &mut *data.cast::<State>();
    state.finish(Err(anyhow!("the compositor sent no frame in time")));
}

/// The first buffer's pixels as RGBA, if it holds a frame.
unsafe fn read_frame(buffer: &SpaBuffer, format: Option<VideoFormat>) -> Option<Frame> {
    let format = format?;
    if buffer.n_datas == 0 || buffer.datas.is_null() {
        return None;
    }
    let data = &*buffer.datas;
    if data.data.is_null() || data.chunk.is_null() {
        return None;
    }
    // Memory only: a DMA-BUF's mapping is not something to read row by row
    // (see `on_param_changed`), and a frame missed is better than a crash.
    if !matches!(data.type_, SPA_DATA_MEM_PTR | SPA_DATA_MEM_FD) {
        log::debug!("screencast: skipping a buffer of type {}", data.type_);
        return None;
    }
    let chunk = &*data.chunk;
    // A zero-size chunk is the compositor saying "nothing new"; it is not a
    // frame, and treating it as one is how a capture comes back blank.
    if chunk.size == 0 {
        return None;
    }
    log::debug!(
        "screencast buffer: type {} flags {:#x} fd {} mapoffset {} maxsize {} data {:p} chunk offset {} size {} stride {} flags {:#x}",
        data.type_, data.flags, data.fd, data.mapoffset, data.maxsize, data.data,
        chunk.offset, chunk.size, chunk.stride, chunk.flags
    );
    let maxsize = data.maxsize as usize;
    let offset = (chunk.offset as usize).checked_rem(maxsize).unwrap_or(0);
    // PipeWire maps a buffer with the access its flags grant, and the wlroots
    // portal (sway) marks its memfds mappable but not readable: the mapping
    // was PROT_NONE and the first pixel read was a SIGSEGV. Such a buffer is
    // mapped again here, read-only, from its fd.
    if data.flags & SPA_DATA_FLAG_READABLE == 0 {
        if data.type_ != SPA_DATA_MEM_FD || data.fd < 0 {
            log::debug!("screencast: buffer is not readable");
            return None;
        }
        let len = data.mapoffset as usize + maxsize;
        let map = libc::mmap(
            null_mut(),
            len,
            libc::PROT_READ,
            libc::MAP_SHARED,
            data.fd as c_int,
            0,
        );
        if map == libc::MAP_FAILED {
            log::debug!("screencast: mapping the buffer failed");
            return None;
        }
        let base = map.cast::<u8>().add(data.mapoffset as usize + offset);
        let frame = convert(std::slice::from_raw_parts(base, maxsize - offset), chunk.stride, format);
        libc::munmap(map, len);
        return frame;
    }
    let src = std::slice::from_raw_parts(data.data.cast::<u8>().add(offset), maxsize - offset);
    convert(src, chunk.stride, format)
}

/// Rows of BGRx/RGBx/BGRA/RGBA at `stride` to tightly packed, opaque RGBA.
fn convert(src: &[u8], stride: i32, format: VideoFormat) -> Option<Frame> {
    let (w, h) = (format.width as usize, format.height as usize);
    if w == 0 || h == 0 {
        return None;
    }
    let row_bytes = w * 4;
    let stride = if stride > 0 { stride as usize } else { row_bytes };
    let swap = matches!(format.format, VIDEO_FORMAT_BGRX | VIDEO_FORMAT_BGRA);
    let mut rgba = vec![0u8; row_bytes * h];
    for row in 0..h {
        let s = row * stride;
        if s + row_bytes > src.len() {
            break;
        }
        let srow = &src[s..s + row_bytes];
        let drow = &mut rgba[row * row_bytes..(row + 1) * row_bytes];
        for (px_s, px_d) in srow.chunks_exact(4).zip(drow.chunks_exact_mut(4)) {
            if swap {
                px_d[0] = px_s[2];
                px_d[1] = px_s[1];
                px_d[2] = px_s[0];
            } else {
                px_d[..3].copy_from_slice(&px_s[..3]);
            }
            // Force alpha: `x` is undefined padding, and carrying it through
            // leaves the snip transparent in patches.
            px_d[3] = 255;
        }
    }
    Some(Frame {
        width: format.width,
        height: format.height,
        rgba,
    })
}

// ---- SPA POD ---------------------------------------------------------------

const SPA_TYPE_ID: u32 = 3;
const SPA_TYPE_INT: u32 = 4;
const SPA_TYPE_OBJECT_PARAM_BUFFERS: u32 = 0x40004;
const SPA_PARAM_BUFFERS: u32 = 5;
const PARAM_BUFFERS_DATA_TYPE: u32 = 6;
const SPA_DATA_MEM_PTR: u32 = 1;
const SPA_DATA_MEM_FD: u32 = 2;
const SPA_DATA_FLAG_READABLE: u32 = 1 << 0;
const SPA_TYPE_RECTANGLE: u32 = 10;
const SPA_TYPE_FRACTION: u32 = 11;
const SPA_TYPE_OBJECT: u32 = 15;
const SPA_TYPE_CHOICE: u32 = 19;
const SPA_TYPE_OBJECT_FORMAT: u32 = 0x40003;
const SPA_PARAM_ENUM_FORMAT: u32 = 3;
const SPA_PARAM_FORMAT: u32 = 4;
const SPA_CHOICE_RANGE: u32 = 1;
const SPA_CHOICE_ENUM: u32 = 3;
const FORMAT_MEDIA_TYPE: u32 = 1;
const FORMAT_MEDIA_SUBTYPE: u32 = 2;
const FORMAT_VIDEO_FORMAT: u32 = 0x20001;
const FORMAT_VIDEO_SIZE: u32 = 0x20003;
const FORMAT_VIDEO_FRAMERATE: u32 = 0x20004;
const MEDIA_TYPE_VIDEO: u32 = 2;
const MEDIA_SUBTYPE_RAW: u32 = 1;
const VIDEO_FORMAT_RGBX: u32 = 7;
const VIDEO_FORMAT_BGRX: u32 = 8;
const VIDEO_FORMAT_RGBA: u32 = 11;
const VIDEO_FORMAT_BGRA: u32 = 12;

#[derive(Debug, Clone, Copy, PartialEq)]
struct VideoFormat {
    width: u32,
    height: u32,
    format: u32,
}

/// Writes SPA POD values; every value is padded to 8 bytes, as SPA requires.
#[derive(Default)]
struct PodWriter(Vec<u8>);

impl PodWriter {
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_ne_bytes());
    }

    fn pad(&mut self) {
        while self.0.len() % 8 != 0 {
            self.0.push(0);
        }
    }

    fn prop_id(&mut self, key: u32, id: u32) {
        self.u32(key);
        self.u32(0);
        self.u32(4);
        self.u32(SPA_TYPE_ID);
        self.u32(id);
        self.pad();
    }

    fn prop_int(&mut self, key: u32, v: u32) {
        self.u32(key);
        self.u32(0);
        self.u32(4);
        self.u32(SPA_TYPE_INT);
        self.u32(v);
        self.pad();
    }

    /// A choice property: `values` are the choice's values in order (for an
    /// enum the default first, then the alternatives; for a range default,
    /// min, max), each `child_size` bytes of `child_type`.
    fn prop_choice(&mut self, key: u32, choice: u32, child_type: u32, values: &[&[u32]]) {
        let child_size = values.first().map_or(0, |v| v.len() * 4) as u32;
        self.u32(key);
        self.u32(0);
        self.u32(16 + child_size * values.len() as u32);
        self.u32(SPA_TYPE_CHOICE);
        self.u32(choice);
        self.u32(0);
        self.u32(child_size);
        self.u32(child_type);
        for value in values {
            value.iter().for_each(|&v| self.u32(v));
        }
        self.pad();
    }

    /// The finished object: header, then `body`.
    fn object(object_type: u32, id: u32, body: impl FnOnce(&mut PodWriter)) -> Vec<u64> {
        let mut props = PodWriter::default();
        props.u32(object_type);
        props.u32(id);
        body(&mut props);
        let mut pod = PodWriter::default();
        pod.u32(props.0.len() as u32);
        pod.u32(SPA_TYPE_OBJECT);
        pod.0.extend_from_slice(&props.0);
        // In u64s: libpipewire reads it as aligned 32-bit fields.
        pod.0
            .chunks_exact(8)
            .map(|c| u64::from_ne_bytes(c.try_into().expect("8 bytes")))
            .collect()
    }
}

/// The buffers this capture can read: in memory, by pointer or by fd.
fn buffers_pod() -> Vec<u64> {
    PodWriter::object(SPA_TYPE_OBJECT_PARAM_BUFFERS, SPA_PARAM_BUFFERS, |w| {
        w.prop_int(PARAM_BUFFERS_DATA_TYPE, (1 << SPA_DATA_MEM_PTR) | (1 << SPA_DATA_MEM_FD));
    })
}

/// The formats this capture can take: raw video, 32-bit RGB in either order,
/// any size, any frame rate.
fn enum_format_pod() -> Vec<u64> {
    PodWriter::object(SPA_TYPE_OBJECT_FORMAT, SPA_PARAM_ENUM_FORMAT, |w| {
        w.prop_id(FORMAT_MEDIA_TYPE, MEDIA_TYPE_VIDEO);
        w.prop_id(FORMAT_MEDIA_SUBTYPE, MEDIA_SUBTYPE_RAW);
        w.prop_choice(
            FORMAT_VIDEO_FORMAT,
            SPA_CHOICE_ENUM,
            SPA_TYPE_ID,
            &[
                &[VIDEO_FORMAT_BGRX],
                &[VIDEO_FORMAT_RGBX],
                &[VIDEO_FORMAT_BGRA],
                &[VIDEO_FORMAT_RGBA],
            ],
        );
        w.prop_choice(
            FORMAT_VIDEO_SIZE,
            SPA_CHOICE_RANGE,
            SPA_TYPE_RECTANGLE,
            &[&[1920, 1080], &[1, 1], &[8192, 8192]],
        );
        // Minimum 0/1, NOT 1/1. A screencast source advertises a variable
        // rate — it sends on damage, not on a clock — and demanding at least
        // one frame a second leaves no format in common with it: the stream
        // goes straight to Error("no more input formats").
        w.prop_choice(
            FORMAT_VIDEO_FRAMERATE,
            SPA_CHOICE_RANGE,
            SPA_TYPE_FRACTION,
            &[&[60, 1], &[0, 1], &[60, 1]],
        );
    })
}

/// Size and pixel format from a negotiated Format object, if it is raw video.
/// A value may come wrapped in a choice; its first value is the one in force.
fn parse_video_format(pod: &[u8]) -> Option<VideoFormat> {
    let rd = |off: usize| -> Option<u32> {
        Some(u32::from_ne_bytes(pod.get(off..off + 4)?.try_into().ok()?))
    };
    if rd(4)? != SPA_TYPE_OBJECT {
        return None;
    }
    let end = (8 + rd(0)? as usize).min(pod.len());
    let (mut media_type, mut subtype, mut format, mut size) = (None, None, None, None);
    let mut off = 16;
    while off + 16 <= end {
        let key = rd(off)?;
        let value_size = rd(off + 8)? as usize;
        let value_type = rd(off + 12)?;
        let body = off + 16;
        if body + value_size > end {
            break;
        }
        let (ty, at, len) = if value_type == SPA_TYPE_CHOICE && value_size >= 16 {
            (rd(body + 12)?, body + 16, rd(body + 8)? as usize)
        } else {
            (value_type, body, value_size)
        };
        if at + len <= end {
            match (key, ty) {
                (FORMAT_MEDIA_TYPE, SPA_TYPE_ID) if len >= 4 => media_type = rd(at),
                (FORMAT_MEDIA_SUBTYPE, SPA_TYPE_ID) if len >= 4 => subtype = rd(at),
                (FORMAT_VIDEO_FORMAT, SPA_TYPE_ID) if len >= 4 => format = rd(at),
                (FORMAT_VIDEO_SIZE, SPA_TYPE_RECTANGLE) if len >= 8 => {
                    size = rd(at).zip(rd(at + 4));
                }
                _ => {}
            }
        }
        off = body + value_size.next_multiple_of(8);
    }
    let (width, height) = size?;
    (media_type? == MEDIA_TYPE_VIDEO && subtype? == MEDIA_SUBTYPE_RAW).then_some(VideoFormat {
        width,
        height,
        format: format?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(words: &[u64]) -> Vec<u8> {
        words.iter().flat_map(|w| w.to_ne_bytes()).collect()
    }

    #[test]
    fn the_buffers_pod_asks_for_memory_only() {
        let b = bytes(&buffers_pod());
        let word = |i: usize| u32::from_ne_bytes(b[i * 4..i * 4 + 4].try_into().unwrap());
        // Object of 32 body bytes: type, id, then one Int property.
        assert_eq!(b.len(), 40);
        assert_eq!((word(0), word(1)), (32, SPA_TYPE_OBJECT));
        assert_eq!((word(2), word(3)), (SPA_TYPE_OBJECT_PARAM_BUFFERS, SPA_PARAM_BUFFERS));
        assert_eq!((word(4), word(6), word(7)), (PARAM_BUFFERS_DATA_TYPE, 4, SPA_TYPE_INT));
        assert_eq!(word(8), 0b110);
    }

    #[test]
    fn a_negotiated_format_is_read_back() {
        let pod = PodWriter::object(SPA_TYPE_OBJECT_FORMAT, SPA_PARAM_FORMAT, |w| {
            w.prop_id(FORMAT_MEDIA_TYPE, MEDIA_TYPE_VIDEO);
            w.prop_id(FORMAT_MEDIA_SUBTYPE, MEDIA_SUBTYPE_RAW);
            w.prop_id(FORMAT_VIDEO_FORMAT, VIDEO_FORMAT_BGRX);
            // Wrapped in a choice, as a negotiated value may be.
            w.prop_choice(FORMAT_VIDEO_SIZE, 0, SPA_TYPE_RECTANGLE, &[&[2560, 1600]]);
        });
        assert_eq!(
            parse_video_format(&bytes(&pod)),
            Some(VideoFormat {
                width: 2560,
                height: 1600,
                format: VIDEO_FORMAT_BGRX
            })
        );
    }

    #[test]
    fn a_format_that_is_not_raw_video_is_refused() {
        let pod = PodWriter::object(SPA_TYPE_OBJECT_FORMAT, SPA_PARAM_FORMAT, |w| {
            w.prop_id(FORMAT_MEDIA_TYPE, 1);
            w.prop_id(FORMAT_MEDIA_SUBTYPE, MEDIA_SUBTYPE_RAW);
            w.prop_id(FORMAT_VIDEO_FORMAT, VIDEO_FORMAT_BGRX);
            w.prop_choice(FORMAT_VIDEO_SIZE, 0, SPA_TYPE_RECTANGLE, &[&[10, 10]]);
        });
        assert_eq!(parse_video_format(&bytes(&pod)), None);
        assert_eq!(parse_video_format(&[0; 4]), None);
    }

    #[test]
    fn bgrx_rows_become_packed_opaque_rgba() {
        // 2×2 BGRx with a 12-byte stride (4 bytes of row padding).
        let src = [
            1, 2, 3, 0, 4, 5, 6, 0, 9, 9, 9, 9, //
            7, 8, 9, 0, 10, 11, 12, 0, 9, 9, 9, 9,
        ];
        let format = VideoFormat {
            width: 2,
            height: 2,
            format: VIDEO_FORMAT_BGRX,
        };
        let frame = convert(&src, 12, format).unwrap();
        assert_eq!(
            frame.rgba,
            [3, 2, 1, 255, 6, 5, 4, 255, 9, 8, 7, 255, 12, 11, 10, 255]
        );
    }
}
