//! Monitor geometry, with the units written down.
//!
//! This module exists because one unstated assumption caused three separate
//! bugs. `xcap` reports monitor extents in LOGICAL pixels on a scaled Wayland
//! session — 1920x1200 on a 3200x2000 panel at 166% — while `Monitor::capture_image`
//! hands back a PHYSICAL-pixel buffer. Nothing in its API says which is which,
//! so every call site guessed, and they did not all guess the same way:
//!
//!   - the snip overlay was sized from the logical rect and filled with the
//!     physical image, so it covered a third of the screen;
//!   - the context-menu surface was sized as if the logical extent were
//!     physical, and covered 60% of it;
//!   - the snip-host surface repeated the same mistake a third time.
//!
//! So: `xcap`'s monitor rects are LOGICAL, always, and every helper here says
//! so in its name. Anything that needs physical pixels derives them from the
//! captured image (see `capture::normalise_to_image`) rather than guessing a
//! scale factor.

/// Logical size of the monitor the app should cover, in xcap's coordinate
/// space.
///
/// Prefers the primary monitor, falls back to the first one, and answers `None`
/// only when there are no monitors at all — in which case callers should leave
/// a window at its natural size rather than invent numbers.
///
/// Enumerating monitors does not capture anything, so this is cheap enough to
/// call whenever a surface is opened.
#[cfg_attr(target_os = "windows", allow(dead_code))] // the menu covers the screen only off Windows
pub fn primary_logical_size() -> Option<(u32, u32)> {
    let monitors = xcap::Monitor::all().ok()?;
    let m = monitors
        .iter()
        .find(|m| m.is_primary())
        .or_else(|| monitors.first())?;
    Some((m.width(), m.height()))
}

/// Scale factor of the monitor under a physical screen point, as Windows
/// reports it to a per-monitor-aware process. `None` off Windows, and when
/// the lookup fails.
///
/// The app's own scale factor is the main window's — that of the monitor the
/// main window is on. A window opened on a monitor of another scale and sized
/// with that number came out the ratio of the two too big or too small, once
/// Windows moved it over and rescaled it: the snip overlay covering part of
/// the screen or spilling off it, a floating snip no longer pixel for pixel.
/// Elsewhere there is nothing to ask: the overlay is fullscreen and adopts
/// whatever scale the compositor gives it (`SlintOverlayManager::sync_to_window`).
#[cfg(target_os = "windows")]
pub fn scale_at(x: i32, y: i32) -> Option<f32> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::{MonitorFromPoint, MONITOR_DEFAULTTONEAREST};
    use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
    let (mut dpi_x, mut dpi_y) = (0u32, 0u32);
    unsafe {
        let monitor = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
        GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y).ok()?;
    }
    (dpi_x > 0).then(|| dpi_x as f32 / 96.0)
}

/// Work area (the monitor minus the taskbar) of the monitor nearest a
/// physical screen point, as `(left, top, right, bottom)` in physical pixels.
/// Windows only — the one platform where the app places its own windows.
#[cfg(target_os = "windows")]
pub fn work_area_at(x: i32, y: i32) -> Option<(i32, i32, i32, i32)> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    unsafe {
        let monitor = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
        if !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return None;
        }
    }
    let r = info.rcWork;
    Some((r.left, r.top, r.right, r.bottom))
}

/// Top-left for a `w`×`h` popup opened at a click, kept inside `work`
/// (`left, top, right, bottom`), all physical pixels. It opens toward the room
/// there is, as a native menu does: up from a click near the bottom, to the
/// left from one near the right-hand edge; and where it fits neither way it is
/// pushed inside, the work area's top-left edge winning.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub fn fit_popup(click: (i32, i32), size: (i32, i32), work: (i32, i32, i32, i32)) -> (i32, i32) {
    let (left, top, right, bottom) = work;
    let axis = |at: i32, len: i32, lo: i32, hi: i32| {
        let start = if at + len > hi { at - len } else { at };
        start.min(hi - len).max(lo)
    };
    (
        axis(click.0, size.0, left, right),
        axis(click.1, size.1, top, bottom),
    )
}

#[cfg(not(target_os = "windows"))]
pub fn scale_at(_x: i32, _y: i32) -> Option<f32> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORK: (i32, i32, i32, i32) = (0, 0, 1920, 1040);

    #[test]
    fn popup_opens_at_the_click_when_it_fits() {
        assert_eq!(fit_popup((100, 200), (300, 400), WORK), (100, 200));
    }

    #[test]
    fn popup_flips_away_from_the_right_and_bottom_edges() {
        assert_eq!(fit_popup((1800, 900), (300, 400), WORK), (1500, 500));
    }

    #[test]
    fn popup_taller_than_the_room_either_way_is_pushed_inside() {
        // 690 px tall from a click at 600 on a 1040 px work area: neither
        // below (to 1290) nor above (from -90) fits.
        assert_eq!(fit_popup((10, 600), (300, 690), WORK), (10, 0));
        // A second monitor to the left, at negative coordinates.
        assert_eq!(
            fit_popup((-50, 10), (300, 400), (-1920, 0, 0, 1080)),
            (-350, 10)
        );
    }

    /// Not an assertion about this machine — just that the helper answers
    /// something coherent wherever it runs, including headless CI with no
    /// monitors at all.
    #[test]
    fn primary_size_is_either_absent_or_non_degenerate() {
        if let Some((w, h)) = primary_logical_size() {
            assert!(w > 0 && h > 0, "a monitor with no extent makes no sense");
        }
    }
}
