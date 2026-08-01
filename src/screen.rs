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
