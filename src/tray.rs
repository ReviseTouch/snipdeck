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
    TRAY_ACTIVE.load(std::sync::atomic::Ordering::Relaxed)
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
    use windows::core::{w, PCWSTR};
    use windows::Win32::UI::WindowsAndMessaging::{
        FindWindowW, IsIconic, SetForegroundWindow, ShowWindow, SW_RESTORE,
    };
    unsafe {
        if let Ok(hwnd) = FindWindowW(PCWSTR::null(), w!("Snipdeck")) {
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
    use windows::core::{w, PCWSTR};
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, IsIconic, IsWindowVisible};
    unsafe {
        match FindWindowW(PCWSTR::null(), w!("Snipdeck")) {
            Ok(hwnd) if !hwnd.0.is_null() => {
                IsWindowVisible(hwnd).as_bool() && !IsIconic(hwnd).as_bool()
            }
            _ => false,
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn is_main_window_shown() -> bool {
    true
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

#[cfg(not(target_os = "windows"))]
pub fn trim_working_set() {}

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

        // Clicks land on tray-icon's own global channel. Forward them onto the
        // service channel from inside the GTK loop, then wake the pump — it
        // parks the main thread when idle and would otherwise not drain these
        // until some unrelated event woke it (see pump.rs).
        gtk::glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            while let Ok(event) = MenuEvent::receiver().try_recv() {
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
            }
            gtk::glib::ControlFlow::Continue
        });

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
    use windows::core::{w, HSTRING, PCWSTR};
    use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::Shell::{
        ExtractIconW, Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE,
        NOTIFYICONDATAW,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu,
        DispatchMessageW, FindWindowW, GetCursorPos, GetMessageW, LoadIconW, PostMessageW,
        RegisterClassW, SendMessageW, SetForegroundWindow, TrackPopupMenu, TranslateMessage,
        HICON, ICON_BIG, ICON_SMALL, IDI_APPLICATION, MF_CHECKED, MF_SEPARATOR, MF_STRING,
        MF_UNCHECKED, MSG, TPM_RETURNCMD, TPM_RIGHTBUTTON, WINDOW_EX_STYLE, WINDOW_STYLE, WM_APP,
        WM_LBUTTONDBLCLK, WM_NULL, WM_RBUTTONUP, WM_SETICON, WNDCLASSW,
    };

    /// Set the main window's title-bar / Alt-Tab icon via WM_SETICON. winit
    /// doesn't pick up the exe's embedded icon for the window itself, so we
    /// apply it natively once the window exists. Returns true once the window
    /// has been found (so the caller can stop retrying).
    pub unsafe fn set_window_icon() -> bool {
        let Ok(hwnd) = FindWindowW(PCWSTR::null(), w!("Snipdeck")) else {
            return false;
        };
        if hwnd.0.is_null() {
            return false;
        }
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

        let icon = load_app_icon(HINSTANCE(hinstance.0));
        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: TRAY_UID,
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: WM_TRAYICON,
            hIcon: icon,
            ..Default::default()
        };
        let tip: Vec<u16> = "Snipdeck".encode_utf16().collect();
        nid.szTip[..tip.len()].copy_from_slice(&tip);
        let _ = Shell_NotifyIconW(NIM_ADD, &nid);
        log::info!("tray icon active");

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
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
