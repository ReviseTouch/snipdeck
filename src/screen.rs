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
pub fn primary_logical_size() -> Option<(u32, u32)> {
    let monitors = xcap::Monitor::all().ok()?;
    let m = monitors
        .iter()
        .find(|m| m.is_primary())
        .or_else(|| monitors.first())?;
    Some((m.width(), m.height()))
}

/// Index of the monitor containing a point given in xcap's LOGICAL space.
///
/// Callers holding a physical screen coordinate — anything that came from a
/// pointer event via the overlay — must divide by the scale factor first.
/// Returns the primary monitor when the point matches none, which is the
/// sensible answer for a cursor that is briefly outside every monitor rect
/// (between two of them, or mid-hotplug).
pub fn monitor_index_at_logical(x: i32, y: i32) -> Option<usize> {
    let monitors = xcap::Monitor::all().ok()?;
    if monitors.is_empty() {
        return None;
    }
    let hit = monitors.iter().position(|m| {
        let (mx, my) = (m.x(), m.y());
        let (mw, mh) = (m.width() as i32, m.height() as i32);
        x >= mx && x < mx + mw && y >= my && y < my + mh
    });
    hit.or_else(|| monitors.iter().position(|m| m.is_primary()))
        .or(Some(0))
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

#[cfg(not(target_os = "windows"))]
pub fn scale_at(_x: i32, _y: i32) -> Option<f32> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

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
