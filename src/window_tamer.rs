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

/// Toggle whether floating snip windows appear in the taskbar / alt-tab
/// list. False (default) applies WS_EX_TOOLWINDOW, true clears it.
/// Walks all existing SnipdeckFloating windows to reflect the change
/// immediately.
#[cfg(target_os = "windows")]
pub fn set_floating_in_taskbar(in_taskbar: bool) {
    imp::set_floating_in_taskbar(in_taskbar);
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
    use windows::Win32::System::Threading::GetCurrentProcessId;
    use windows::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, EnumWindows, GetClassNameW, GetForegroundWindow, GetWindow,
        GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
        PeekMessageW,
        SetWindowDisplayAffinity, SetWindowLongPtrW, SetWindowPos, TranslateMessage,
        EVENT_OBJECT_CREATE, GWL_EXSTYLE, GWL_HWNDPARENT, GWL_STYLE, GW_OWNER, HWND_TOPMOST, MSG,
        PM_REMOVE, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOSIZE,
        SWP_NOZORDER, WDA_EXCLUDEFROMCAPTURE, WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS,
        WS_CAPTION, WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    };

    const SWEEP_INTERVAL: Duration = Duration::from_millis(16);
    const LOOP_IDLE_SLEEP: Duration = Duration::from_millis(4);
    const FLOATING_REVEAL_DELAY: Duration = Duration::from_millis(120);

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

            // Initial sweep to tame any windows that already existed
            // when the hook was installed (e.g., the main viewport).
            sweep_now();

            // Pump messages for the hook + belt-and-suspenders sweep.
            let mut last_sweep = Instant::now();
            let mut msg = MSG::default();
            loop {
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
                if last_sweep.elapsed() >= SWEEP_INTERVAL {
                    sweep_now();
                    last_sweep = Instant::now();
                }
                reveal_due_windows();
                std::thread::sleep(LOOP_IDLE_SLEEP);
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
            configure_floating_hwnd(hwnd, owner_hwnd, in_taskbar, false);
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
        if desired_ex != current_ex {
            let _ = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, desired_ex);
            changed_shell_visibility = true;
        }

        if changed_shell_visibility {
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
        tame_hwnd(hwnd);
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
        let is_owned_aux_window =
            is_floating_window || title == "SnipdeckContextMenu" || title == "SnipdeckOverlay";
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
                configure_floating_hwnd(hwnd, main_hwnd, in_taskbar, false);
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
