//! Disables Win11's open/close window animation on every top-level window
//! that belongs to this process.
//!
//! Slint/winit creates its own native windows (gallery + overlay + each
//! floating snip). Without disabling transitions, every fresh viewport
//! fades in over ~200ms — visually this reads as "another window briefly
//! opened and closed" right when the user releases the snip.
//!
//! Strategy: install a per-process SetWinEventHook for OBJECT_CREATE.
//! The hook fires the instant a new HWND appears — well before the
//! fade animation has any visible frames — and we slam
//! `TRANSITIONS_FORCEDISABLED = TRUE` on it. Floating snip viewports
//! also get cloaked briefly; Slint/winit can create and show the HWND
//! before its first frame is fully present, and hiding that first reveal
//! removes the "window flashed in the middle" artifact.
//! Belt-and-suspenders: a 16ms polling loop sweeps for any HWNDs the hook
//! might have missed (e.g., windows created before the hook was installed
//! or titles assigned just after OBJECT_CREATE).

#[cfg(not(target_os = "windows"))]
pub fn start() {}

#[cfg(not(target_os = "windows"))]
pub fn app_is_foreground() -> bool {
    true
}

#[cfg(not(target_os = "windows"))]
pub fn foreground_token() -> isize {
    0
}

#[cfg(not(target_os = "windows"))]
pub fn set_floating_in_taskbar(_in_taskbar: bool) {}

#[cfg(not(target_os = "windows"))]
pub fn configure_floating_window(
    _window: &slint::Window,
    _owner_window: Option<&slint::Window>,
    _in_taskbar: bool,
) {
}

#[cfg(target_os = "windows")]
pub fn start() {
    // Dedicated thread runs both the event-hook message loop *and* the
    // belt-and-suspenders sweep so we only need one OS thread.
    std::thread::spawn(imp::run);
}

/// True when the foreground window belongs to this process — i.e. Snipdeck
/// is the active application. Used to clear the floating-snip selection
/// highlight while the user is working in another app.
#[cfg(target_os = "windows")]
pub fn app_is_foreground() -> bool {
    imp::app_is_foreground()
}

/// The window in the foreground right now, as an opaque token.
///
/// A floating snip refuses activation (WS_EX_NOACTIVATE, so popping one
/// never steals focus), which means clicking it leaves another app in the
/// foreground — and the selection highlight, shown "while Snipdeck is the
/// active app", was never shown at all. The app records this token when a
/// snip is selected and keeps the highlight while it is unchanged: until the
/// user actually moves to another window.
#[cfg(target_os = "windows")]
pub fn foreground_token() -> isize {
    imp::foreground_token()
}

/// Toggle whether floating snip windows appear in the taskbar / alt-tab
/// list. False (default) applies WS_EX_TOOLWINDOW, true clears it.
/// Walks all existing SnipdeckFloating windows to reflect the change
/// immediately.
#[cfg(target_os = "windows")]
pub fn set_floating_in_taskbar(in_taskbar: bool) {
    imp::set_floating_in_taskbar(in_taskbar);
}

/// The top-level window titled exactly `title` that this process owns.
///
/// `FindWindowW(NULL, "Snipdeck")` matched on the title alone, and a title is
/// anyone's to take — an Explorer window open on a folder named Snipdeck, a
/// browser tab, a document. Whichever came first in z-order was raised by the
/// tray's Show, judged for the main window's visibility, given Snipdeck's icon,
/// and made the anchor of the share flyout.
#[cfg(target_os = "windows")]
pub fn find_own_window(title: &str) -> Option<windows::Win32::Foundation::HWND> {
    imp::find_window(title, false)
}

/// The same, owned by any process running this executable: how a second
/// launch finds the first instance's window to raise.
#[cfg(target_os = "windows")]
pub fn find_instance_window(title: &str) -> Option<windows::Win32::Foundation::HWND> {
    imp::find_window(title, true)
}

/// Apply the Alt+Tab/taskbar policy directly to a Slint window.
///
/// This is called from the floating-window creation path, before `show()`
/// when Slint has already created a native HWND, and immediately after
/// `show()` as a fallback for backends that create the HWND lazily.
#[cfg(target_os = "windows")]
pub fn configure_floating_window(
    window: &slint::Window,
    owner_window: Option<&slint::Window>,
    in_taskbar: bool,
) {
    imp::configure_floating_window(window, owner_window, in_taskbar);
}

#[cfg(target_os = "windows")]
mod imp {
    use std::collections::{HashMap, HashSet};
    use std::time::{Duration, Instant};

    use raw_window_handle::{HasWindowHandle as _, RawWindowHandle};
    use windows::Win32::Foundation::*;
    use windows::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_CLOAK, DWMWA_TRANSITIONS_FORCEDISABLED,
    };
    use windows::Win32::System::Threading::{GetCurrentProcessId, GetCurrentThreadId};
    use windows::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, EnumWindows, GetClassNameW, GetForegroundWindow, GetMessageW, GetWindow,
        GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
        IsWindowVisible, KillTimer, PostThreadMessageW, SetTimer,
        SetWindowDisplayAffinity, SetWindowLongPtrW, SetWindowPos, ShowWindow, TranslateMessage,
        EVENT_OBJECT_CREATE, EVENT_SYSTEM_FOREGROUND, GWL_EXSTYLE, GWL_HWNDPARENT, GWL_STYLE, GW_OWNER, HWND_TOPMOST, MSG,
        SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER,
        SWP_NOSIZE, SWP_NOZORDER, SW_HIDE, SW_SHOWNA, WDA_EXCLUDEFROMCAPTURE,
        WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS, WM_APP, WM_TIMER, WS_CAPTION,
        WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    };

    /// Follow-up sweep cadence while there is active window churn.
    const SWEEP_INTERVAL_MS: u32 = 16;
    const FLOATING_REVEAL_DELAY: Duration = Duration::from_millis(120);
    /// How long follow-up sweeps keep running after the last
    /// window-creation event (or taskbar-policy toggle). Covers the
    /// "title assigned just after OBJECT_CREATE" race the sweep exists
    /// for, with a wide margin.
    const ACTIVE_AFTER_EVENT: Duration = Duration::from_secs(2);

    /// Custom thread message: re-apply the taskbar policy now (posted by
    /// `set_floating_in_taskbar` from the main thread).
    const WM_APP_RESWEEP: u32 = WM_APP + 1;

    /// The tamer thread's id, so other threads can post WM_APP_RESWEEP.
    static TAMER_THREAD_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

    thread_local! {
        /// Deadline until which the follow-up sweep timer keeps firing.
        /// Extended by every OBJECT_CREATE / taskbar toggle. Tamer thread only.
        static ACTIVE_UNTIL: std::cell::Cell<Option<Instant>> = const { std::cell::Cell::new(None) };
        /// Id of the armed thread timer (0 = none). Tamer thread only.
        static SWEEP_TIMER: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }

    /// Whether floating snip windows should appear in the taskbar. Set
    /// via `set_floating_in_taskbar` from settings; read by tame_hwnd
    /// each time it sees a SnipdeckFloating window. AtomicBool so the
    /// hook thread can read without locking.
    static FLOATING_IN_TASKBAR: std::sync::atomic::AtomicBool =
        std::sync::atomic::AtomicBool::new(false);

    /// HWND of our main gallery window. Floating and auxiliary Slint
    /// windows are re-owned to it so owned top-level windows stay out
    /// of Alt+Tab and the taskbar (same approach the legacy AHK build
    /// used with `+Owner`). 0 until the main window has been spotted.
    static MAIN_HWND: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

    thread_local! {
        static SEEN: std::cell::RefCell<HashSet<isize>> =
            std::cell::RefCell::new(HashSet::new());
        static DELAYED_REVEAL: std::cell::RefCell<HashSet<isize>> =
            std::cell::RefCell::new(HashSet::new());
        static REVEAL_AT: std::cell::RefCell<HashMap<isize, Instant>> =
            std::cell::RefCell::new(HashMap::new());
    }

    pub fn run() {
        unsafe {
            // EVENT_OBJECT_CREATE fires for every accessibility object —
            // we filter inside the callback to only react to top-level
            // windows of our own process. WINEVENT_SKIPOWNPROCESS would
            // skip our process entirely so we DON'T set it.
            let pid = GetCurrentProcessId();
            let _hook = SetWinEventHook(
                EVENT_OBJECT_CREATE,
                EVENT_OBJECT_CREATE,
                None,
                Some(win_event_cb),
                pid,
                0,
                WINEVENT_OUTOFCONTEXT,
            );

            // Foreground changes anywhere on the desktop wake the pump, which
            // re-judges the floating-snip selection highlight (see
            // `foreground_token`). With the pump asleep, the highlight would
            // otherwise outlast the switch to another app until something
            // else happened to wake it.
            let _foreground_hook = SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_FOREGROUND,
                None,
                Some(foreground_cb),
                0,
                0,
                WINEVENT_OUTOFCONTEXT,
            );

            TAMER_THREAD_ID.store(GetCurrentThreadId(), std::sync::atomic::Ordering::Relaxed);

            // Initial sweep to tame any windows that already existed
            // when the hook was installed (e.g., the main viewport), plus
            // a short window of follow-up sweeps for late startup windows.
            sweep_now();
            extend_active_window();

            // Block in GetMessageW at zero CPU. Crucially, OUTOFCONTEXT
            // WinEvent callbacks are dispatched from INSIDE GetMessageW the
            // instant they arrive — so a new HWND is tamed while the main
            // thread is still setting the window up, BEFORE `show()`. That
            // ordering is what keeps windows from ever being style-fixed
            // while visible (which needs a hide/show bounce → the fullscreen
            // black flicker). A parked MsgWaitForMultipleObjects loop lost
            // that race: it does NOT dispatch WinEvents while waiting, so
            // windows were seen only after they were already on screen.
            //
            // Timed work (the 16 ms belt-and-suspenders sweep for ~2 s after
            // each creation event, and pending delayed reveals) runs off a
            // thread timer (WM_TIMER wakes GetMessageW), armed on demand and
            // killed when idle — so a quiet system stays at zero wake-ups.
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
                match msg.message {
                    // Thread timers (no HWND) are not dispatched to a wndproc;
                    // handle them right here.
                    WM_TIMER => on_sweep_timer(),
                    WM_APP_RESWEEP => {
                        // Taskbar policy toggled: re-apply to existing
                        // floating windows now and keep sweeping briefly.
                        sweep_now();
                        extend_active_window();
                    }
                    _ => {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }
            }
        }
    }

    pub fn configure_floating_window(
        window: &slint::Window,
        owner_window: Option<&slint::Window>,
        in_taskbar: bool,
    ) {
        let Some(hwnd) = hwnd_from_slint(window) else {
            log::debug!("configure_floating_window: floating HWND not available yet");
            return;
        };
        let owner_hwnd = owner_window
            .and_then(hwnd_from_slint)
            .map(|hwnd| hwnd.0 as isize)
            .or_else(|| {
                let cached = MAIN_HWND.load(std::sync::atomic::Ordering::Relaxed);
                (cached != 0).then_some(cached)
            })
            .unwrap_or_default();

        unsafe {
            // exclude_from_capture = false: floating snips must remain
            // capturable so the user can take a new snip *over* an existing
            // floating snip (WDA_EXCLUDEFROMCAPTURE made them invisible to the
            // DXGI capture, so over-snips grabbed the content behind instead).
            // Bounce allowed: this creation-path call happens before/right
            // after show, and floating windows are the ones whose taskbar
            // button must track the setting.
            configure_floating_hwnd(hwnd, owner_hwnd, in_taskbar, false, true);
        }
    }

    fn hwnd_from_slint(window: &slint::Window) -> Option<HWND> {
        let handle = window.window_handle();
        let raw = handle.window_handle().ok()?.as_raw();
        match raw {
            RawWindowHandle::Win32(win32) => Some(HWND(isize::from(win32.hwnd) as *mut _)),
            _ => None,
        }
    }

    unsafe fn configure_floating_hwnd(
        hwnd: HWND,
        _owner_hwnd: isize,
        in_taskbar: bool,
        exclude_from_capture: bool,
        allow_visibility_bounce: bool,
    ) {
        let mut changed_shell_visibility = false;

        // These auxiliary windows (overlay, floating snips, context menu)
        // are intentionally NOT owned by the main gallery window. When an
        // *active* owned window is hidden or destroyed, Windows activates
        // its owner — so the main window flashed to the foreground every
        // time the snip overlay closed. WS_EX_TOOLWINDOW (set below) already
        // keeps them out of the taskbar and Alt+Tab, which is the only thing
        // ownership ever bought us. Clear any owner the window may carry.
        let desired_owner: isize = 0;
        let current_owner = GetWindow(hwnd, GW_OWNER).unwrap_or_default().0 as isize;
        if current_owner != desired_owner {
            let _ = SetWindowLongPtrW(hwnd, GWL_HWNDPARENT, desired_owner);
            changed_shell_visibility = true;
        }

        let current_ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as isize;
        let toolwin = WS_EX_TOOLWINDOW.0 as isize;
        let appwin = WS_EX_APPWINDOW.0 as isize;
        let noactivate = WS_EX_NOACTIVATE.0 as isize;
        // Default (not in taskbar): hide from the shell *and* refuse
        // activation, so popping a snip never steals focus from — nor
        // disturbs the Z-order of — the user's current app. When the user
        // opts a floating snip into the taskbar they clearly want it
        // reachable, so leave it activatable there.
        let desired_ex = if in_taskbar {
            (current_ex & !toolwin & !noactivate) | appwin
        } else {
            (current_ex | toolwin | noactivate) & !appwin
        };
        let ex_changed = desired_ex != current_ex;
        if ex_changed {
            let _ = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, desired_ex);
            changed_shell_visibility = true;
            log::info!(
                "configure_floating: in_taskbar={in_taskbar} visible={} ex {current_ex:#x} -> {desired_ex:#x} (toolwindow now {})",
                IsWindowVisible(hwnd).as_bool(),
                desired_ex & toolwin != 0,
            );
        }

        if changed_shell_visibility {
            // SetWindowLongPtr(EXSTYLE) + SWP_FRAMECHANGED is NOT enough to make
            // the shell ADD or DROP a taskbar button once the window has been
            // shown: Windows only re-reads WS_EX_TOOLWINDOW / WS_EX_APPWINDOW
            // when a window transitions hidden -> shown. So when the exstyle
            // actually changed on an already-visible window, bounce it
            // hidden -> shown (no-activate, so focus is untouched) to force the
            // taskbar to re-evaluate. Without this the toolwindow flag is set
            // but the taskbar button lingers — the regression we were chasing.
            //
            // ONLY for windows whose taskbar button actually matters (floating
            // snips — `allow_visibility_bounce`). Bouncing the fullscreen snip
            // OVERLAY made it flash as a huge black rectangle mid-snip: the
            // event-driven tamer can lose the race with `show()` and see the
            // overlay only after it's visible, and hiding/re-showing a
            // software-rendered fullscreen window paints black until the next
            // present. A transient taskbar button on those windows is far
            // cheaper than the flicker.
            if allow_visibility_bounce && ex_changed && IsWindowVisible(hwnd).as_bool() {
                log::info!(
                    "visibility BOUNCE (hide/show) on hwnd={:?} title={:?}",
                    hwnd.0,
                    window_title(hwnd),
                );
                let _ = ShowWindow(hwnd, SW_HIDE);
                let _ = ShowWindow(hwnd, SW_SHOWNA);
            }
            let _ = SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE
                    | SWP_NOSIZE
                    | SWP_NOZORDER
                    | SWP_NOACTIVATE
                    | SWP_NOOWNERZORDER
                    | SWP_FRAMECHANGED,
            );
        }

        if exclude_from_capture {
            let _ = SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE);
        }
    }

    unsafe extern "system" fn foreground_cb(
        _hook: HWINEVENTHOOK,
        _event: u32,
        _hwnd: HWND,
        _id_object: i32,
        _id_child: i32,
        _id_thread: u32,
        _event_time: u32,
    ) {
        crate::pump::wake();
    }

    pub fn foreground_token() -> isize {
        unsafe { GetForegroundWindow().0 as isize }
    }

    unsafe extern "system" fn win_event_cb(
        _hook: HWINEVENTHOOK,
        event: u32,
        hwnd: HWND,
        id_object: i32,
        _id_child: i32,
        _id_thread: u32,
        _event_time: u32,
    ) {
        // OBJID_WINDOW = 0. Filter out child controls + non-window
        // objects (caret, menu, etc.) — we only care about top-level
        // windows.
        if event != EVENT_OBJECT_CREATE || id_object != 0 {
            return;
        }
        if hwnd.0.is_null() {
            return;
        }
        // A window was just created — schedule follow-up sweeps to catch
        // late title assignment. Runs on the tamer thread (dispatched from
        // its GetMessageW), so the thread-local timer state is ours.
        extend_active_window();
        tame_hwnd(hwnd);
    }

    /// Extend the follow-up sweep deadline and make sure the sweep timer is
    /// armed. Tamer thread only (thread-local state + thread timer).
    fn extend_active_window() {
        ACTIVE_UNTIL.with(|a| a.set(Some(Instant::now() + ACTIVE_AFTER_EVENT)));
        SWEEP_TIMER.with(|t| {
            if t.get() == 0 {
                let id = unsafe { SetTimer(None, 0, SWEEP_INTERVAL_MS, None) };
                t.set(id);
            }
        });
    }

    /// One firing of the follow-up sweep timer: sweep + service delayed
    /// reveals; disarm once the active window expired and nothing is pending.
    fn on_sweep_timer() {
        unsafe {
            sweep_now();
            reveal_due_windows();
        }
        let reveals_pending = REVEAL_AT.with(|r| !r.borrow().is_empty());
        let still_active = ACTIVE_UNTIL
            .with(|a| a.get())
            .is_some_and(|deadline| Instant::now() < deadline);
        if !reveals_pending && !still_active {
            SWEEP_TIMER.with(|t| {
                if t.get() != 0 {
                    let _ = unsafe { KillTimer(None, t.get()) };
                    t.set(0);
                }
            });
        }
    }

    unsafe fn sweep_now() {
        let _ = EnumWindows(Some(sweep_cb), LPARAM(0));
    }

    unsafe extern "system" fn sweep_cb(hwnd: HWND, _lparam: LPARAM) -> BOOL {
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid != GetCurrentProcessId() {
            return TRUE;
        }
        tame_hwnd(hwnd);
        TRUE
    }

    unsafe fn tame_hwnd(hwnd: HWND) {
        let key = hwnd.0 as isize;
        let needs_transition_tame = SEEN.with(|s| {
            let mut s = s.borrow_mut();
            if s.contains(&key) {
                false
            } else {
                s.insert(key);
                true
            }
        });

        if needs_transition_tame {
            let disable: BOOL = TRUE;
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_TRANSITIONS_FORCEDISABLED,
                &disable as *const _ as *const _,
                std::mem::size_of::<BOOL>() as u32,
            );
        }

        // Identify the main gallery window by exact title. Slint
        // `no-frame: true` windows can still report WS_CAPTION, so a
        // style-only heuristic misclassifies floating snips as "main".
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        let has_caption = (style & WS_CAPTION.0) != 0;
        let title = window_title(hwnd);
        let is_main_window = title == "Snipdeck";
        let is_floating_window = title == "SnipdeckFloating";
        let is_owned_aux_window = is_floating_window
            || title == "SnipdeckContextMenu"
            || title == "SnipdeckOverlay"
            || title == "SnipdeckLoupe"
            || title == "SnipdeckColorResult";
        if needs_transition_tame {
            let main_now = MAIN_HWND.load(std::sync::atomic::Ordering::Relaxed);
            log::info!(
                "tame_hwnd first_seen: hwnd={:?} title={:?} caption={} style={:#x} main_hwnd_cached={:x}",
                hwnd.0,
                title,
                has_caption,
                style,
                main_now,
            );
        }
        if is_main_window {
            // Multiple windows could carry a caption in theory; lock in
            // the first one we see and don't churn afterwards.
            if MAIN_HWND.load(std::sync::atomic::Ordering::Relaxed) == 0 {
                MAIN_HWND.store(hwnd.0 as isize, std::sync::atomic::Ordering::Relaxed);
            }
        } else if is_owned_aux_window || !has_caption {
            // Floating snips can report WS_CAPTION even with Slint
            // `no-frame: true`, so title identification is more reliable
            // than style-only classification.
            let main_hwnd = MAIN_HWND.load(std::sync::atomic::Ordering::Relaxed);
            if main_hwnd != 0 {
                let in_taskbar = FLOATING_IN_TASKBAR.load(std::sync::atomic::Ordering::Relaxed);
                let in_taskbar = is_floating_window && in_taskbar;
                // Bounce only floating snips: the sweep can reach a window
                // AFTER it became visible, and bouncing e.g. the fullscreen
                // overlay there flashes a black rectangle mid-snip.
                configure_floating_hwnd(hwnd, main_hwnd, in_taskbar, false, is_floating_window);
            }
        }

        // NOTE: floating snips are deliberately *not* excluded from screen
        // capture (no WDA_EXCLUDEFROMCAPTURE). Excluding them made it
        // impossible to snip over a floating snip — the DXGI capture saw
        // straight through to the desktop behind it.

        delay_reveal_if_needed(hwnd);
    }

    pub fn set_floating_in_taskbar(in_taskbar: bool) {
        FLOATING_IN_TASKBAR.store(in_taskbar, std::sync::atomic::Ordering::Relaxed);
        // The sweep re-applies the policy to existing floating windows; ask
        // the tamer thread (parked in GetMessageW) to do one now.
        let tid = TAMER_THREAD_ID.load(std::sync::atomic::Ordering::Relaxed);
        if tid != 0 {
            unsafe {
                let _ = PostThreadMessageW(tid, WM_APP_RESWEEP, WPARAM(0), LPARAM(0));
            }
        }
    }

    pub fn app_is_foreground() -> bool {
        unsafe {
            let fg = GetForegroundWindow();
            if fg.0.is_null() {
                return false;
            }
            let mut pid: u32 = 0;
            GetWindowThreadProcessId(fg, Some(&mut pid));
            pid == GetCurrentProcessId()
        }
    }

    unsafe fn delay_reveal_if_needed(hwnd: HWND) {
        if !should_delay_reveal(hwnd) {
            return;
        }

        let key = hwnd.0 as isize;
        let should_schedule = DELAYED_REVEAL.with(|s| {
            let mut s = s.borrow_mut();
            s.insert(key)
        });
        if !should_schedule {
            return;
        }

        set_cloak(hwnd, true);
        REVEAL_AT.with(|r| {
            r.borrow_mut()
                .insert(key, Instant::now() + FLOATING_REVEAL_DELAY);
        });
    }

    unsafe fn reveal_due_windows() {
        let now = Instant::now();
        let due = REVEAL_AT.with(|r| {
            let mut r = r.borrow_mut();
            let due: Vec<isize> = r
                .iter()
                .filter_map(|(hwnd, at)| if *at <= now { Some(*hwnd) } else { None })
                .collect();
            for hwnd in &due {
                r.remove(hwnd);
            }
            due
        });

        for key in due {
            let hwnd = HWND(key as *mut _);
            let mut pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if pid == GetCurrentProcessId() {
                set_cloak(hwnd, false);
            }
        }
    }

    unsafe fn should_delay_reveal(hwnd: HWND) -> bool {
        let class_name = window_class(hwnd);
        if class_name == "SnipdeckOverlayClass" {
            return false;
        }

        let title = window_title(hwnd);
        if title == "SnipdeckOverlay" {
            return false;
        }

        if title.starts_with("Snipdeck #") || title == "Snipdeck - Overlay" {
            return true;
        }

        if title.starts_with("Snipdeck") {
            let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
            return (style & WS_CAPTION.0) == 0;
        }

        false
    }

    unsafe fn set_cloak(hwnd: HWND, cloaked: bool) {
        let cloak: BOOL = if cloaked { TRUE } else { FALSE };
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_CLOAK,
            &cloak as *const _ as *const _,
            std::mem::size_of::<BOOL>() as u32,
        );
    }

    struct WindowQuery {
        title: String,
        /// Accept other processes running the same executable, not just this
        /// one.
        any_instance: bool,
        own_pid: u32,
        exe_name: Option<String>,
        found: Option<HWND>,
    }

    pub fn find_window(title: &str, any_instance: bool) -> Option<HWND> {
        let mut query = WindowQuery {
            title: title.to_owned(),
            any_instance,
            own_pid: unsafe { GetCurrentProcessId() },
            exe_name: std::env::current_exe().ok().as_deref().and_then(file_name_lower),
            found: None,
        };
        unsafe {
            // Stopping early makes EnumWindows report an error; the answer is
            // in `found` either way.
            let _ = EnumWindows(
                Some(find_window_cb),
                LPARAM(&mut query as *mut WindowQuery as isize),
            );
        }
        query.found
    }

    unsafe extern "system" fn find_window_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let query = &mut *(lparam.0 as *mut WindowQuery);
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        let ours = pid == query.own_pid
            || (query.any_instance
                && query.exe_name.is_some()
                && process_exe_name(pid) == query.exe_name);
        if ours && window_title(hwnd) == query.title {
            query.found = Some(hwnd);
            return FALSE;
        }
        TRUE
    }

    /// Lower-cased file name of the executable behind a process.
    unsafe fn process_exe_name(pid: u32) -> Option<String> {
        use windows::Win32::System::Threading::{
            OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
            PROCESS_QUERY_LIMITED_INFORMATION,
        };
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let queried = QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            windows::core::PWSTR(buf.as_mut_ptr()),
            &mut len,
        );
        let _ = CloseHandle(process);
        queried.ok()?;
        let path = std::path::PathBuf::from(String::from_utf16_lossy(&buf[..len as usize]));
        file_name_lower(&path)
    }

    fn file_name_lower(path: &std::path::Path) -> Option<String> {
        Some(path.file_name()?.to_string_lossy().to_lowercase())
    }

    unsafe fn window_title(hwnd: HWND) -> String {
        let len = GetWindowTextLengthW(hwnd);
        if len <= 0 {
            return String::new();
        }
        let mut buf = vec![0u16; len as usize + 1];
        let n = GetWindowTextW(hwnd, &mut buf);
        String::from_utf16_lossy(&buf[..n.max(0) as usize])
    }

    unsafe fn window_class(hwnd: HWND) -> String {
        let mut buf = [0u16; 128];
        let n = GetClassNameW(hwnd, &mut buf);
        String::from_utf16_lossy(&buf[..n.max(0) as usize])
    }

    // Silence unused-import warning for WINEVENT_SKIPOWNPROCESS even though
    // it's not currently set — kept here as a hint for future changes.
    #[allow(dead_code)]
    const _: u32 = WINEVENT_SKIPOWNPROCESS;
}
