//! Single-instance guard — one Snipdeck per user session.
//!
//! Snipdeck installs process-wide low-level mouse/keyboard hooks, a global
//! hotkey, and the window-tamer event hook. Launching a second copy makes
//! the two fight over those shared resources: the duplicate fails to
//! register its hotkey (`register hotkey CtrlShift+C`) and the tamer hooks
//! double up. That is exactly the "the app won't start" symptom of starting
//! it while another instance is already open.
//!
//! Mechanism: a named mutex. The first instance creates it; any later
//! instance sees `ERROR_ALREADY_EXISTS`, raises the existing window to the
//! foreground, and bails out. The OS releases the mutex when the owning
//! process dies, so a crash never leaves a stale lock behind (unlike a PID
//! file).

/// Returns `true` if this process may continue (it is the sole instance),
/// `false` if another instance already owns the lock — in which case the
/// caller should exit immediately.
#[cfg(not(target_os = "windows"))]
pub fn acquire() -> bool {
    true
}

#[cfg(target_os = "windows")]
pub fn acquire() -> bool {
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS, FALSE};
    use windows::Win32::System::Threading::CreateMutexW;
    use windows::Win32::UI::WindowsAndMessaging::{
        FindWindowW, IsIconic, SetForegroundWindow, ShowWindow, SW_RESTORE,
    };

    // Session-local namespace ("Local\\") → one instance per logon session,
    // which is the right granularity for a per-user tray/gallery app. The
    // suffix is an arbitrary fixed token to avoid colliding with any other
    // app's mutex names.
    let name = w!("Local\\Snipdeck-SingleInstance-7f1c9e2a");

    unsafe {
        let created = CreateMutexW(None, FALSE, name);

        // Read the last error immediately: on the success path the
        // windows-rs wrapper does not touch the thread-local last-error,
        // so this still reflects CreateMutexW's own result.
        let already_running = GetLastError() == ERROR_ALREADY_EXISTS;

        // The handle is intentionally leaked: `HANDLE` has no `Drop`, so the
        // mutex stays owned for the whole process lifetime and the OS frees
        // it on exit.
        if created.is_err() {
            // Could not create the lock at all — fail open so the user is
            // never locked out by a guard malfunction.
            return true;
        }

        if already_running {
            // Another instance holds the lock. Surface its window so the
            // user sees the app they just tried to launch, then quit.
            if let Ok(hwnd) = FindWindowW(PCWSTR::null(), w!("Snipdeck")) {
                if IsIconic(hwnd).as_bool() {
                    let _ = ShowWindow(hwnd, SW_RESTORE);
                }
                let _ = SetForegroundWindow(hwnd);
            }
            return false;
        }

        true
    }
}
