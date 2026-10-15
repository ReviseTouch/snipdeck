//! System-tray icon + context menu.
//!
//! Closing the main window with its X hides it to the tray rather than
//! quitting (see `main.rs`). The tray icon's menu lets the user bring the
//! window back, start a fresh snip, toggle launch-at-login, or actually
//! quit. Commands are delivered to the main (Slint) thread over a channel
//! that the timer drains each tick — the same pattern the mouse-trigger
//! service uses — so all UI work still happens on the main thread.
//!
//! Implemented natively (no extra crate) via `Shell_NotifyIconW` + a hidden
//! message window on its own thread, mirroring `native_mouse_trigger`.

use crossbeam_channel::Receiver;
// Only the Windows path keeps a Sender at module scope (in `TRAY_TX`); the
// Linux tray owns its own inside `linux`.
#[cfg(target_os = "windows")]
use crossbeam_channel::Sender;

#[derive(Debug, Clone, Copy)]
pub enum TrayCommand {
    /// Restore + focus the main window.
    Show,
    /// Arm a one-shot drag-snip (next left-drag captures, no Win needed).
    NewSnip,
    /// Flip launch-at-login.
    ToggleAutostart,
    /// Relaunch Snipdeck elevated via one UAC prompt (one-shot, no persistence).
    /// Windows only: no other tray offers it.
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    RunAsAdmin,
    /// Quit the whole app.
    Exit,
}

pub struct TrayService {
    pub rx: Receiver<TrayCommand>,
}

/// Set once the Linux tray icon is actually live on the panel — see
/// [`can_hide_to_tray`].
#[cfg(target_os = "linux")]
static TRAY_ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Whether closing the main window may hide it instead of quitting — i.e.
/// whether anything exists that can bring it back.
///
/// On Windows the `Shell_NotifyIconW` icon is created unconditionally, so this
/// is always true. On Linux the tray can genuinely fail to appear (a session
/// with no StatusNotifierHost, e.g. stock GNOME without the AppIndicator
/// extension), which would otherwise strand a running process with no window
/// and no way back — so it is a runtime question, not a compile-time one.
#[cfg(target_os = "windows")]
pub fn can_hide_to_tray() -> bool {
    true
}

#[cfg(target_os = "linux")]
pub fn can_hide_to_tray() -> bool {
    TRAY_ACTIVE.load(std::sync::atomic::Ordering::Relaxed) && status_notifier_host_present()
}

/// Is anything on the panel able to show the icon right now?
///
/// Building the icon proves nothing: libayatana-appindicator registers it
/// whether or not a StatusNotifierHost exists to draw it, so on stock GNOME
/// without the AppIndicator extension the tray was "live" and invisible, and
/// the window's X hid the app with no way back. The watcher is asked each time
/// the window closes, not once: the extension that provides it can load after
/// Snipdeck has started.
#[cfg(target_os = "linux")]
fn status_notifier_host_present() -> bool {
    let answer = pollster::block_on(async {
        let conn = zbus::Connection::session().await.ok()?;
        let watcher = zbus::Proxy::new(
            &conn,
            "org.kde.StatusNotifierWatcher",
            "/StatusNotifierWatcher",
            "org.kde.StatusNotifierWatcher",
        )
        .await
        .ok()?;
        watcher.get_property::<bool>("IsStatusNotifierHostRegistered").await.ok()
    });
    let present = answer.unwrap_or(false);
    if !present {
        log::info!("no StatusNotifierHost on the panel; the window's X quits instead of hiding");
    }
    present
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub fn can_hide_to_tray() -> bool {
    false
}

#[cfg(target_os = "windows")]
static TRAY_TX: std::sync::OnceLock<Sender<TrayCommand>> = std::sync::OnceLock::new();

/// Shared tray-icon UID (used both when adding the icon and when removing it
/// from another thread in [`shutdown`]).
#[cfg(target_os = "windows")]
const TRAY_UID: u32 = 0x5350;

/// Create the tray icon (on its own thread) and return a handle whose `rx`
/// the caller polls for menu commands.
pub fn start() -> TrayService {
    let (tx, rx) = crossbeam_channel::unbounded();
    #[cfg(target_os = "windows")]
    {
        if TRAY_TX.set(tx).is_err() {
            log::warn!("tray already started");
        } else {
            std::thread::spawn(|| unsafe { win::run() });
        }
    }
    #[cfg(target_os = "linux")]
    {
        std::thread::spawn(move || linux::run(tx));
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        let _ = tx;
    }
    TrayService { rx }
}

/// Restore and foreground the main Snipdeck window (called from the Show
/// command handler). Matches the raise logic in `single_instance`.
#[cfg(target_os = "windows")]
pub fn focus_main_window() {
    use windows::Win32::UI::WindowsAndMessaging::{
        IsIconic, SetForegroundWindow, ShowWindow, SW_RESTORE,
    };
    unsafe {
        if let Some(hwnd) = crate::window_tamer::find_own_window("Snipdeck") {
            if IsIconic(hwnd).as_bool() {
                let _ = ShowWindow(hwnd, SW_RESTORE);
            }
            let _ = SetForegroundWindow(hwnd);
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn focus_main_window() {}

/// Whether the main window currently exists, is visible, and is NOT minimized
/// — i.e. actually on screen. The timer compares this across ticks to detect
/// the window reappearing (restored from the taskbar / tray / Start-menu
/// minimize) so it can kick a repaint.
#[cfg(target_os = "windows")]
pub fn is_main_window_shown() -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{IsIconic, IsWindowVisible};
    unsafe {
        match crate::window_tamer::find_own_window("Snipdeck") {
            Some(hwnd) => IsWindowVisible(hwnd).as_bool() && !IsIconic(hwnd).as_bool(),
            None => false,
        }
    }
}


/// Trim the process working set when the window is hidden/minimized so RAM
/// drops while Snipdeck sits in the tray. The pages move to the standby list
/// rather than being discarded, so bringing the window back is fast (served
/// from standby, not re-read from disk). The 8ms timer keeps running for snip
/// capture, so the set only grows back as far as each tick actually touches —
/// the gallery thumbnails and render buffers stay parked while hidden.
#[cfg(target_os = "windows")]
pub fn trim_working_set() {
    use windows::Win32::System::Threading::{GetCurrentProcess, SetProcessWorkingSetSize};
    // Both sizes (SIZE_T)-1 tells Windows to trim the working set to the minimum.
    unsafe {
        let _ = SetProcessWorkingSetSize(GetCurrentProcess(), usize::MAX, usize::MAX);
    }
}


/// Remove the tray icon from outside the tray thread. Used on the
/// "restart elevated" path, where the process exits via `quit_event_loop`
/// (which never reaches the tray thread's own cleanup) so the icon would
/// otherwise linger as a ghost until the next hover.
#[cfg(target_os = "windows")]
pub fn shutdown() {
    use windows::core::{w, PCWSTR};
    use windows::Win32::UI::Shell::{Shell_NotifyIconW, NIM_DELETE, NOTIFYICONDATAW};
    use windows::Win32::UI::WindowsAndMessaging::FindWindowW;
    unsafe {
        if let Ok(hwnd) = FindWindowW(w!("SnipdeckTrayWindow"), PCWSTR::null()) {
            if !hwnd.0.is_null() {
                let nid = NOTIFYICONDATAW {
                    cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                    hWnd: hwnd,
                    uID: TRAY_UID,
                    ..Default::default()
                };
                let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn shutdown() {}

/// Apply the embedded app icon to the main window's title bar / Alt-Tab.
/// Returns true once the window exists and the icon has been applied, so the
/// caller can stop retrying on subsequent ticks.
#[cfg(target_os = "windows")]
pub fn set_main_window_icon() -> bool {
    unsafe { win::set_window_icon() }
}

#[cfg(not(target_os = "windows"))]
pub fn set_main_window_icon() -> bool {
    true
}

/// Linux tray, via `tray-icon` (StatusNotifierItem / AppIndicator).
///
/// Unlike the Windows side — a hand-rolled `Shell_NotifyIconW` message window
/// — this needs GTK: the icon must be created on a thread where `gtk::init`
/// ran, and a GTK main loop has to service it. Both live on this thread, so
/// the app's Slint/winit loop is untouched.
///
/// Every failure path here degrades to "no tray" rather than killing the
/// thread's caller: on a desktop with no StatusNotifierHost (a stock GNOME
/// without the AppIndicator extension) the rest of the app still works.
#[cfg(target_os = "linux")]
mod linux {
    use super::TrayCommand;
    use crossbeam_channel::Sender;
    use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
    use tray_icon::{Icon, TrayIconBuilder};

    /// Pre-decoded 32x32 RGBA — the same buffer the Windows tray icon uses,
    /// so both platforms show the identical glyph with no image decoding.
    const ICON_RGBA: &[u8] = include_bytes!("../assets/tray_32.rgba");
    const ICON_SIDE: u32 = 32;

    pub fn run(tx: Sender<TrayCommand>) {
        // `libappindicator-sys` PANICS rather than returning an Err when it
        // can dlopen neither libayatana-appindicator3 nor libappindicator3 —
        // i.e. exactly the slim desktop / container / WSLg case this is meant
        // to survive. Found by actually running under WSLg: without this the
        // thread unwinds and the crash-log hook records a backtrace, which
        // reads to the user as "Snipdeck crashed" instead of "no tray here".
        // `TRAY_ACTIVE` stays false either way, so the window's X keeps
        // quitting rather than hiding into a tray that does not exist.
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run_inner(tx)));
        if caught.is_err() {
            log::warn!(
                "tray unavailable: the appindicator library could not be loaded \
                 — continuing without a tray"
            );
        }
    }

    fn run_inner(tx: Sender<TrayCommand>) {
        if let Err(err) = gtk::init() {
            log::warn!("tray disabled: GTK init failed ({err})");
            return;
        }

        let menu = Menu::new();
        let show = MenuItem::new(crate::i18n::t("Show Snipdeck"), true, None);
        let new_snip = MenuItem::new(crate::i18n::t("New Snip"), true, None);
        // Reflects the current state at build time; the menu is rebuilt only
        // on restart, same as the Windows menu's check mark.
        let autostart = CheckMenuItem::new(
            crate::i18n::t("Launch at Login"),
            true,
            crate::autostart::is_enabled(),
            None,
        );
        let exit = MenuItem::new(crate::i18n::t("Exit"), true, None);
        // "Run as administrator" is deliberately absent: the elevated relaunch
        // is a Windows/UAC concept (see elevate.rs), not a Linux one.
        if let Err(err) = menu.append_items(&[
            &show,
            &new_snip,
            &PredefinedMenuItem::separator(),
            &autostart,
            &PredefinedMenuItem::separator(),
            &exit,
        ]) {
            log::warn!("tray disabled: menu build failed ({err})");
            return;
        }

        let icon = match Icon::from_rgba(ICON_RGBA.to_vec(), ICON_SIDE, ICON_SIDE) {
            Ok(icon) => icon,
            Err(err) => {
                log::warn!("tray disabled: icon rejected ({err})");
                return;
            }
        };

        // Held for the lifetime of the GTK loop below — dropping it removes
        // the icon from the panel.
        let _tray = match TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Snipdeck")
            .with_icon(icon)
            .build()
        {
            Ok(tray) => tray,
            Err(err) => {
                log::warn!("tray unavailable ({err}) — continuing without one");
                return;
            }
        };
        // Only now may the main window's X hide instead of quitting: there is
        // finally something that can bring it back.
        super::TRAY_ACTIVE.store(true, std::sync::atomic::Ordering::Relaxed);

        let (show_id, new_id, autostart_id, exit_id) = (
            show.id().clone(),
            new_snip.id().clone(),
            autostart.id().clone(),
            exit.id().clone(),
        );

        // Clicks arrive through tray-icon's event handler, called on the GTK
        // thread the moment a menu item is chosen: forward each onto the
        // service channel and wake the pump, which parks the main thread
        // when idle (see pump.rs). This replaced a 100 ms GTK timeout that
        // polled the channel, which woke the process ten times a second
        // while it sat in the tray with nothing to do.
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            let cmd = if event.id == show_id {
                Some(TrayCommand::Show)
            } else if event.id == new_id {
                Some(TrayCommand::NewSnip)
            } else if event.id == autostart_id {
                Some(TrayCommand::ToggleAutostart)
            } else if event.id == exit_id {
                Some(TrayCommand::Exit)
            } else {
                None
            };
            if let Some(cmd) = cmd {
                let _ = tx.send(cmd);
                crate::pump::wake();
            }
        }));

        gtk::main();
    }
}

#[cfg(target_os = "windows")]
fn send(cmd: TrayCommand) {
    if let Some(tx) = TRAY_TX.get() {
        let _ = tx.send(cmd);
        // Tray clicks arrive while we're idle in the tray — wake the pump so
        // it drains the command instead of waiting for the next event.
        crate::pump::wake();
    }
}

#[cfg(target_os = "windows")]
mod win {
    use super::{send, TrayCommand};
    use std::sync::atomic::{AtomicU32, Ordering};
    use windows::core::{w, HSTRING, PCWSTR};
    use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::Shell::{
        ExtractIconW, Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE,
        NOTIFYICONDATAW,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        AppendMenuW, ChangeWindowMessageFilterEx, CreatePopupMenu, CreateWindowExW,
        DefWindowProcW, DestroyMenu, DispatchMessageW, GetCursorPos, GetMessageW,
        LoadIconW, PostMessageW, RegisterClassW, RegisterWindowMessageW, SendMessageW,
        SetForegroundWindow, TrackPopupMenu, TranslateMessage, MSGFLT_ALLOW,
        HICON, ICON_BIG, ICON_SMALL, IDI_APPLICATION, MF_CHECKED, MF_SEPARATOR, MF_STRING,
        MF_UNCHECKED, MSG, TPM_RETURNCMD, TPM_RIGHTBUTTON, WINDOW_EX_STYLE, WINDOW_STYLE, WM_APP,
        WM_LBUTTONDBLCLK, WM_NULL, WM_RBUTTONUP, WM_SETICON, WNDCLASSW,
    };

    /// Set the main window's title-bar / Alt-Tab icon via WM_SETICON. winit
    /// doesn't pick up the exe's embedded icon for the window itself, so we
    /// apply it natively once the window exists. Returns true once the window
    /// has been found (so the caller can stop retrying).
    pub unsafe fn set_window_icon() -> bool {
        let Some(hwnd) = crate::window_tamer::find_own_window("Snipdeck") else {
            return false;
        };
        let hinst = GetModuleHandleW(None).unwrap_or_default();
        let icon = load_app_icon(HINSTANCE(hinst.0));
        if !icon.0.is_null() {
            let lp = windows::Win32::Foundation::LPARAM(icon.0 as isize);
            let _ = SendMessageW(hwnd, WM_SETICON, WPARAM(ICON_SMALL as usize), lp);
            let _ = SendMessageW(hwnd, WM_SETICON, WPARAM(ICON_BIG as usize), lp);
        }
        true
    }

    use super::TRAY_UID;

    const WM_TRAYICON: u32 = WM_APP + 1;
    const ID_SHOW: u32 = 1;
    const ID_NEWSNIP: u32 = 2;
    const ID_AUTOSTART: u32 = 3;
    const ID_EXIT: u32 = 4;
    const ID_RUNADMIN: u32 = 5;

    pub unsafe fn run() {
        let hinstance = GetModuleHandleW(None).unwrap_or_default();
        let class_name = w!("SnipdeckTrayWindow");

        let wc = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: HINSTANCE(hinstance.0),
            lpszClassName: class_name,
            ..Default::default()
        };
        RegisterClassW(&wc);

        let hwnd = match CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class_name,
            w!("Snipdeck Tray"),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            None,
            None,
            HINSTANCE(hinstance.0),
            None,
        ) {
            Ok(h) => h,
            Err(e) => {
                log::error!("tray window failed: {e}");
                return;
            }
        };

        // Explorer broadcasts "TaskbarCreated" when the taskbar comes back
        // after a crash or restart, and every tray icon has to be added
        // again: the new taskbar knows nothing of the old one's. Unheard, the
        // icon was simply gone, and with the main window hidden to the tray
        // so was the way back into the app. An elevated Snipdeck ("Run as
        // administrator") would not hear it either without letting the
        // message through UIPI: Explorer runs at the lower integrity level.
        let taskbar_created = RegisterWindowMessageW(w!("TaskbarCreated"));
        if taskbar_created != 0 {
            TASKBAR_CREATED.store(taskbar_created, Ordering::Relaxed);
            let _ = ChangeWindowMessageFilterEx(hwnd, taskbar_created, MSGFLT_ALLOW, None);
        }

        add_icon(hwnd);
        log::info!("tray icon active");

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    /// The registered "TaskbarCreated" message, 0 until registered.
    static TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);

    unsafe fn add_icon(hwnd: HWND) {
        let hinstance = GetModuleHandleW(None).unwrap_or_default();
        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: TRAY_UID,
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: WM_TRAYICON,
            hIcon: load_app_icon(HINSTANCE(hinstance.0)),
            ..Default::default()
        };
        let tip: Vec<u16> = "Snipdeck".encode_utf16().collect();
        nid.szTip[..tip.len()].copy_from_slice(&tip);
        if !Shell_NotifyIconW(NIM_ADD, &nid).as_bool() {
            log::warn!("tray icon could not be added");
        }
    }

    /// Prefer the embedded app icon (extracted from our own exe, so it works
    /// regardless of the resource id winresource picked); fall back to the
    /// generic application icon if embedding was skipped.
    unsafe fn load_app_icon(hinstance: HINSTANCE) -> HICON {
        use std::os::windows::ffi::OsStrExt;
        if let Ok(exe) = std::env::current_exe() {
            let wide: Vec<u16> = exe
                .as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            let h = ExtractIconW(hinstance, PCWSTR(wide.as_ptr()), 0);
            // ExtractIconW returns NULL (no icons) or HICON(1) (not an
            // icon-bearing file); treat both as "no icon".
            if !h.0.is_null() && h.0 as usize != 1 {
                return h;
            }
        }
        LoadIconW(None, IDI_APPLICATION).unwrap_or_default()
    }

    unsafe extern "system" fn wndproc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        let taskbar_created = TASKBAR_CREATED.load(Ordering::Relaxed);
        if taskbar_created != 0 && msg == taskbar_created {
            log::info!("taskbar recreated; adding the tray icon again");
            add_icon(hwnd);
            return LRESULT(0);
        }
        if msg == WM_TRAYICON {
            let mouse_msg = (lparam.0 as u32) & 0xFFFF;
            match mouse_msg {
                WM_RBUTTONUP => show_menu(hwnd),
                WM_LBUTTONDBLCLK => send(TrayCommand::Show),
                _ => {}
            }
            return LRESULT(0);
        }
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }

    unsafe fn show_menu(hwnd: HWND) {
        let menu = match CreatePopupMenu() {
            Ok(m) => m,
            Err(_) => return,
        };
        let show = HSTRING::from(crate::i18n::t("Show Snipdeck").as_str());
        let _ = AppendMenuW(menu, MF_STRING, ID_SHOW as usize, PCWSTR(show.as_ptr()));
        let new_snip = HSTRING::from(crate::i18n::t("New Snip").as_str());
        let _ = AppendMenuW(menu, MF_STRING, ID_NEWSNIP as usize, PCWSTR(new_snip.as_ptr()));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let check = if crate::autostart::is_enabled() {
            MF_CHECKED
        } else {
            MF_UNCHECKED
        };
        let autostart_label = HSTRING::from(crate::i18n::t("Launch at Login").as_str());
        let _ = AppendMenuW(
            menu,
            MF_STRING | check,
            ID_AUTOSTART as usize,
            PCWSTR(autostart_label.as_ptr()),
        );
        // One-shot "Run as administrator": relaunch elevated via UAC so a
        // capture can reach over administrator (UAC-elevated) windows. Pointless
        // once we are already elevated, so hide it then.
        if !crate::elevate::is_elevated() {
            let admin_label = HSTRING::from(crate::i18n::t("Run as administrator").as_str());
            let _ = AppendMenuW(
                menu,
                MF_STRING,
                ID_RUNADMIN as usize,
                PCWSTR(admin_label.as_ptr()),
            );
        }
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let exit = HSTRING::from(crate::i18n::t("Exit").as_str());
        let _ = AppendMenuW(menu, MF_STRING, ID_EXIT as usize, PCWSTR(exit.as_ptr()));

        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        // Required so the menu dismisses when the user clicks elsewhere.
        let _ = SetForegroundWindow(hwnd);
        let cmd = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON,
            pt.x,
            pt.y,
            0,
            hwnd,
            None,
        );
        let _ = DestroyMenu(menu);
        let _ = PostMessageW(hwnd, WM_NULL, WPARAM(0), LPARAM(0));

        match cmd.0 as u32 {
            ID_SHOW => send(TrayCommand::Show),
            ID_NEWSNIP => send(TrayCommand::NewSnip),
            ID_AUTOSTART => send(TrayCommand::ToggleAutostart),
            ID_RUNADMIN => send(TrayCommand::RunAsAdmin),
            ID_EXIT => {
                remove_icon(hwnd);
                send(TrayCommand::Exit);
            }
            _ => {}
        }
    }

    unsafe fn remove_icon(hwnd: HWND) {
        let nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: TRAY_UID,
            ..Default::default()
        };
        let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
    }
}
