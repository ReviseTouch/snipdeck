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
    unix::acquire()
}

/// Like [`acquire`], but for an instance that was just relaunched elevated by a
/// still-running medium instance (the tray "Run as administrator" handoff).
/// That handoff is Windows-only (see `elevate`), so off Windows there is never
/// a predecessor to wait for and this is a plain [`acquire`].
#[cfg(not(target_os = "windows"))]
pub fn acquire_after_handoff() -> bool {
    unix::acquire()
}

/// Unix counterpart of the named mutex: an advisory `flock` on a per-user
/// file. The kernel releases it when the owning process dies — crash
/// included — so, like the Windows mutex and unlike a PID file, a stale
/// lock can never wedge the app.
///
/// Unlike Windows we do NOT raise the running instance's window: doing that
/// portably needs a D-Bus activation service we don't ship. The duplicate
/// just logs and exits, which is the part that actually matters — two
/// copies writing the same SQLite gallery is the bug being prevented.
#[cfg(not(target_os = "windows"))]
mod unix {
    use std::fs::OpenOptions;
    use std::os::unix::io::AsRawFd;
    use std::path::PathBuf;
    use std::sync::OnceLock;

    /// Holds the locked file for the whole process lifetime — closing it
    /// would drop the lock. Never read; kept only for that side effect.
    static LOCK: OnceLock<std::fs::File> = OnceLock::new();

    /// `$XDG_RUNTIME_DIR/snipdeck.lock` when the session provides one: a
    /// per-user tmpfs cleared on logout, the same granularity the Windows
    /// `Local\` namespace gives. Falls back to the data dir otherwise.
    fn lock_path() -> Option<PathBuf> {
        let dir = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .or_else(|| dirs::data_dir().map(|d| d.join("snipdeck")))?;
        std::fs::create_dir_all(&dir).ok()?;
        Some(dir.join("snipdeck.lock"))
    }

    pub fn acquire() -> bool {
        // Every failure below returns `true` (keep running). Refusing to
        // start because we couldn't create a lock file would turn a guard
        // against a rare duplicate launch into a hard startup failure.
        let Some(path) = lock_path() else {
            log::warn!("single-instance: no runtime/data dir — guard skipped");
            return true;
        };
        let file = match OpenOptions::new().create(true).write(true).open(&path) {
            Ok(file) => file,
            Err(err) => {
                log::warn!("single-instance: cannot open {}: {err}", path.display());
                return true;
            }
        };
        // LOCK_NB: answer immediately instead of blocking behind the running
        // instance.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            log::info!(
                "another Snipdeck instance already holds {} — exiting",
                path.display()
            );
            return false;
        }
        let _ = LOCK.set(file);
        true
    }
}

#[cfg(target_os = "windows")]
pub fn acquire() -> bool {
    acquire_inner(false)
}

/// Like [`acquire`], but used by an instance that was just relaunched elevated
/// from a still-running medium instance (the tray "Run as administrator"
/// handoff). The predecessor is exiting concurrently, so instead of mistaking
/// the brief lock overlap for "already running" and quitting — which would
/// leave *no* instance running at all — this waits a short while for the old
/// instance to release the lock, then takes it over.
#[cfg(target_os = "windows")]
pub fn acquire_after_handoff() -> bool {
    acquire_inner(true)
}

#[cfg(target_os = "windows")]
fn acquire_inner(wait_for_handoff: bool) -> bool {
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::{
        CloseHandle, GetLastError, ERROR_ACCESS_DENIED, ERROR_ALREADY_EXISTS, FALSE,
    };
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
        // Elevated-handoff path: the medium instance that spawned us is
        // exiting right now but may still hold the lock for a few ms. Poll
        // until it lets go (then this fresh, high-integrity mutex is ours)
        // rather than seeing the brief overlap as "already running" and
        // quitting — which would leave NO instance running at all. We must
        // close our own reference on every miss, otherwise it keeps the
        // object alive and the predecessor's exit never frees it.
        if wait_for_handoff {
            // ~3s budget at 50ms steps: generous for a process exit, yet
            // bounded so a wedged predecessor can't hang the relaunch forever.
            for _ in 0..60 {
                let probe = CreateMutexW(None, FALSE, name);
                let probe_err = GetLastError();
                let Ok(handle) = probe else {
                    // Can't even open it (unexpected here) — stop waiting and
                    // fall through to the normal one-shot logic below.
                    break;
                };
                if probe_err != ERROR_ALREADY_EXISTS {
                    // Clean creation: the predecessor is gone and this handle
                    // is ours. Leak it (held for the whole process lifetime).
                    return true;
                }
                let _ = CloseHandle(handle);
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            // Handoff didn't complete in time — fall through and take the lock
            // best-effort so the user still gets their elevated instance.
        }

        let created = CreateMutexW(None, FALSE, name);

        // Read the last error immediately: on the success path the
        // windows-rs wrapper does not touch the thread-local last-error,
        // so this still reflects CreateMutexW's own result.
        let last_err = GetLastError();

        // A bare creation failure is treated as a guard malfunction and we
        // fail open (the user is never locked out). The one exception is
        // ERROR_ACCESS_DENIED: that means the named mutex already exists at a
        // HIGHER integrity level than us — e.g. an elevated Snipdeck launched
        // by the "run as administrator at login" task — which a medium
        // process cannot open. That is "already running", not a malfunction,
        // so fall through to the raise-and-bail path instead of spawning a
        // second instance that would fight over the hooks and hotkey.
        if created.is_err() && last_err != ERROR_ACCESS_DENIED {
            return true;
        }

        // The handle is intentionally leaked: `HANDLE` has no `Drop`, so the
        // mutex stays owned for the whole process lifetime and the OS frees
        // it on exit.
        let already_running =
            last_err == ERROR_ALREADY_EXISTS || last_err == ERROR_ACCESS_DENIED;

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
