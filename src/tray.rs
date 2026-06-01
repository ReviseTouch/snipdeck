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

use crossbeam_channel::{Receiver, Sender};

#[derive(Debug, Clone, Copy)]
pub enum TrayCommand {
    /// Restore + focus the main window.
    Show,
    /// Arm a one-shot drag-snip (next left-drag captures, no Win needed).
    NewSnip,
    /// Flip launch-at-login.
    ToggleAutostart,
    /// Quit the whole app.
    Exit,
}

pub struct TrayService {
    pub rx: Receiver<TrayCommand>,
}

#[cfg(target_os = "windows")]
static TRAY_TX: std::sync::OnceLock<Sender<TrayCommand>> = std::sync::OnceLock::new();

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
    #[cfg(not(target_os = "windows"))]
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

#[cfg(target_os = "windows")]
fn send(cmd: TrayCommand) {
    if let Some(tx) = TRAY_TX.get() {
        let _ = tx.send(cmd);
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

    const WM_TRAYICON: u32 = WM_APP + 1;
    const TRAY_UID: u32 = 0x5350;
    const ID_SHOW: u32 = 1;
    const ID_NEWSNIP: u32 = 2;
    const ID_AUTOSTART: u32 = 3;
    const ID_EXIT: u32 = 4;

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
