//! Optional one-shot "Run as administrator".
//!
//! Snipdeck runs at the user's ordinary (Medium) integrity level by default —
//! the low-friction, AV-friendly design. The only thing that level cannot do
//! is start a capture over a higher-integrity (UAC-elevated) window: Windows
//! skips a medium process's low-level input hooks for input directed at an
//! elevated foreground window (anti-keylogger UIPI protection).
//!
//! For the rare case where the user wants exactly that, this offers a single
//! manual action: relaunch Snipdeck elevated via one UAC prompt. There is NO
//! persistence — no scheduled task, no autostart change. The user clicks it
//! when they need it; next launch is ordinary again. (The preferred path for
//! admin windows is to let the OS snipping tool — Win+Shift+S — capture them
//! and have Snipdeck import the result from the clipboard, which needs no
//! elevation at all.)

/// Is THIS process currently running elevated (high integrity)? Asked only
/// by the Windows tray, the one place that offers elevation.
#[cfg(target_os = "windows")]
pub fn is_elevated() -> bool {
    imp::is_elevated()
}

/// Relaunch Snipdeck elevated via a single UAC prompt. Returns `Ok(true)` if a
/// new elevated instance was launched (the caller should then exit so it takes
/// over the single-instance lock), `Ok(false)` if the user dismissed the UAC
/// prompt (nothing changed).
pub fn relaunch_as_admin() -> anyhow::Result<bool> {
    imp::relaunch_as_admin()
}

#[cfg(target_os = "windows")]
mod imp {
    use anyhow::{anyhow, Result};

    pub fn is_elevated() -> bool {
        use windows::Win32::Foundation::{CloseHandle, HANDLE};
        use windows::Win32::Security::{
            GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
        };
        use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
        unsafe {
            let mut token = HANDLE::default();
            if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
                return false;
            }
            let mut elevation = TOKEN_ELEVATION::default();
            let mut ret_len = 0u32;
            let ok = GetTokenInformation(
                token,
                TokenElevation,
                Some(&mut elevation as *mut _ as *mut std::ffi::c_void),
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut ret_len,
            );
            let _ = CloseHandle(token);
            ok.is_ok() && elevation.TokenIsElevated != 0
        }
    }

    pub fn relaunch_as_admin() -> Result<bool> {
        use windows::core::{HSTRING, PCWSTR};
        use windows::Win32::UI::Shell::{ShellExecuteExW, SHELLEXECUTEINFOW};
        use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        let exe = std::env::current_exe()?;
        let verb = HSTRING::from("runas");
        let file = HSTRING::from(exe.as_os_str());
        // Tell the elevated instance it is taking over from a still-running
        // medium instance, so it waits for our single-instance lock to be
        // released instead of seeing it held and quitting as a duplicate.
        let params = HSTRING::from("--elevated-takeover");

        unsafe {
            let mut info = SHELLEXECUTEINFOW {
                cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
                lpVerb: PCWSTR(verb.as_ptr()),
                lpFile: PCWSTR(file.as_ptr()),
                lpParameters: PCWSTR(params.as_ptr()),
                nShow: SW_SHOWNORMAL.0,
                ..Default::default()
            };
            match ShellExecuteExW(&mut info) {
                Ok(()) => Ok(true),
                Err(e) => {
                    // 0x800704C7 == HRESULT_FROM_WIN32(ERROR_CANCELLED): the
                    // user dismissed the UAC prompt — not an error.
                    if e.code().0 == 0x800704C7u32 as i32 {
                        Ok(false)
                    } else {
                        Err(anyhow!("elevation failed: {e}"))
                    }
                }
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod imp {
    pub fn relaunch_as_admin() -> anyhow::Result<bool> {
        Ok(false)
    }
}
