//! Runtime Windows-version gate (Windows 10 vs 11).
//!
//! Currently unused: the Win-key Start-menu disguise that needed this gate
//! was unified into a single release-time suppress-and-replace that works on
//! both Windows 10 and 11 (see `mouse_trigger::DISGUISE_WINUP_ARMED`). Kept
//! (allow(dead_code)) because OS-gated workarounds keep coming up — e.g.
//! Win10-specific DXGI capture quirks — and the detection method below is
//! non-obvious, hard-won knowledge.
//!
//! Detection uses `RtlGetVersion` (ntdll) rather than `GetVersionEx`: the
//! latter lies without an app manifest (it caps at the manifested version or
//! Windows 8), while `RtlGetVersion` reports the true build regardless of
//! manifest or compatibility shims. Build 22000+ is Windows 11.

/// True on Windows 11 (build >= 22000), false on Windows 10 / earlier and on
/// non-Windows. Cached after the first call.
#[cfg(target_os = "windows")]
#[allow(dead_code)]
pub fn is_windows_11() -> bool {
    use std::sync::OnceLock;
    static CACHE: OnceLock<bool> = OnceLock::new();
    *CACHE.get_or_init(|| {
        let win11 = build_number().is_some_and(|b| b >= 22000);
        log::info!(
            "OS gate: build={:?} -> {}",
            build_number(),
            if win11 { "Windows 11" } else { "Windows 10 / earlier" }
        );
        win11
    })
}

#[cfg(target_os = "windows")]
#[allow(dead_code)]
fn build_number() -> Option<u32> {
    use windows::Wdk::System::SystemServices::RtlGetVersion;
    use windows::Win32::System::SystemInformation::OSVERSIONINFOW;

    let mut info = OSVERSIONINFOW {
        dwOSVersionInfoSize: std::mem::size_of::<OSVERSIONINFOW>() as u32,
        ..Default::default()
    };
    // RtlGetVersion always succeeds (STATUS_SUCCESS); guard anyway.
    let status = unsafe { RtlGetVersion(&mut info) };
    status.is_ok().then_some(info.dwBuildNumber)
}

#[cfg(not(target_os = "windows"))]
#[allow(dead_code)]
pub fn is_windows_11() -> bool {
    false
}
