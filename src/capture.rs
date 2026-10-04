use anyhow::{anyhow, Context};
use image::{GenericImageView, RgbaImage};

use crate::snip::ScreenRect;

#[derive(Debug, Clone)]
pub struct CapturedScreen {
    /// Screen rect this image covers (in virtual-screen coordinates).
    pub area: ScreenRect,
    pub image: RgbaImage,
    /// Human-readable monitor identifier (e.g. "Monitor 1").
    pub monitor_label: String,
}

/// Capture the monitor that contains the given virtual-screen point.
/// Falls back to the primary monitor if no monitor matches (shouldn't
/// happen on a well-behaved system, but cheap to handle).
pub fn capture_at(x: i32, y: i32) -> anyhow::Result<CapturedScreen> {
    let monitors = xcap::Monitor::all().context("enumerating monitors")?;
    if let Some((idx, m)) = monitors.iter().enumerate().find(|(_, m)| {
        let mx = m.x();
        let my = m.y();
        let mw = m.width() as i32;
        let mh = m.height() as i32;
        x >= mx && x < mx + mw && y >= my && y < my + mh
    }) {
        return capture_monitor(m.clone(), idx, monitors.len());
    }
    // Fall back to the first monitor, not just the primary one. Nothing
    // guarantees any monitor claims to be primary — some compositors report
    // none, and a capture that fails outright there would take the whole
    // feature down over a flag that is only ever a hint.
    let idx = monitors
        .iter()
        .position(|m| m.is_primary())
        .or(if monitors.is_empty() { None } else { Some(0) })
        .ok_or_else(|| anyhow!("no monitors available"))?;
    let count = monitors.len();
    let monitor = monitors.into_iter().nth(idx).unwrap();
    capture_monitor(monitor, idx, count)
}

/// Rescale a monitor rect reported in logical pixels onto the pixel grid of
/// the image that was actually captured.
///
/// The origin is scaled by the same ratio as the size: on a multi-monitor
/// virtual desktop a monitor's logical origin is offset by the logical widths
/// of the monitors left of it, so it needs the same treatment as the extent.
/// Returns the input untouched when the image already matches (ratio 1) or
/// when either dimension is zero, so a degenerate monitor can never produce a
/// division by zero or a NaN rect.
fn normalise_to_image(
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    image: &RgbaImage,
) -> (i32, i32, u32, u32) {
    let (iw, ih) = (image.width(), image.height());
    if w == 0 || h == 0 || (iw == w && ih == h) {
        return (x, y, w, h);
    }
    let sx = iw as f64 / w as f64;
    let sy = ih as f64 / h as f64;
    log::debug!("monitor rect {w}x{h} rescaled to image {iw}x{ih} (x{sx:.3}, x{sy:.3})");
    (
        (x as f64 * sx).round() as i32,
        (y as f64 * sy).round() as i32,
        iw,
        ih,
    )
}

/// The compositor's own buffers, where that is offered.
///
/// `None` means "not this platform, the session could not be had, or this is
/// not a setup we can be sure about" — never a hard failure, because the xcap
/// path behind it still works everywhere.
///
/// Single-monitor only, and that restriction is the point. The portal grants a
/// cast of ONE monitor — whichever the user picked, or whichever the restore
/// token re-grants without asking — and the stream has no idea which monitor
/// the caller wanted. This used to hand back the granted monitor's pixels for
/// a capture of any monitor: `capture_monitor` would then fit the *requested*
/// monitor's rect onto the *granted* monitor's image, and the snip came out
/// showing the wrong screen at coordinates belonging to the right one.
///
/// The doc here already claimed a capture of a different monitor "falls
/// through to xcap". It did not; nothing compared them, and the position the
/// stream reports — which is the only thing that could identify the monitor —
/// was discarded at the match. Comparing it is not straightforward either:
/// the stream reports in its own pixels while xcap reports the monitor rect in
/// logical ones, so on a scaled session the two do not agree on any number.
/// One monitor is the case where the question cannot arise, and it is also the
/// case the fast path was justified by — "nearly everyone".
#[cfg(not(target_os = "windows"))]
fn screencast_image(monitor_count: usize) -> Option<image::RgbaImage> {
    if !crate::wayland_screencast::usable() {
        return None;
    }
    if monitor_count != 1 {
        log::debug!("screencast skipped: {monitor_count} monitors, cast covers only one");
        return None;
    }
    match crate::wayland_screencast::capture() {
        Ok((_, _, img)) => {
            log::info!("capture path: ScreenCast (PipeWire)");
            Some(img)
        }
        Err(e) => {
            log::debug!("screencast capture unavailable ({e:#}); using xcap");
            None
        }
    }
}

#[cfg(target_os = "windows")]
fn screencast_image(_monitor_count: usize) -> Option<image::RgbaImage> {
    None
}

fn capture_monitor(
    m: xcap::Monitor,
    idx: usize,
    monitor_count: usize,
) -> anyhow::Result<CapturedScreen> {
    let x = m.x();
    let y = m.y();
    let w = m.width();
    let h = m.height();
    // xcap exposes a `name()` (e.g. "\\.\DISPLAY1") which is ugly for
    // a UI; prefer a friendly "Monitor N" label and append the OS name
    // only if it's something descriptive (e.g. "DELL U2422H").
    let raw = m.name();
    let friendly = if raw.starts_with("\\\\") || raw.is_empty() {
        format!("{} {}", crate::i18n::t("Monitor"), idx + 1)
    } else {
        format!("{} {} · {}", crate::i18n::t("Monitor"), idx + 1, raw)
    };
    // Prefer DXGI Desktop Duplication: GDI BitBlt (xcap's backend) returns
    // black for GPU-composited windows (Warp, hardware-accelerated browsers,
    // games). DXGI grabs the real composited frame. Fall back to GDI only if
    // duplication is unavailable. The duplication image is physical-pixel; fit
    // it to the monitor's reported (w, h) so the existing crop/overlay math —
    // which the GDI path established at exactly those dimensions — is unchanged.
    let cx = x + (w / 2) as i32;
    let cy = y + (h / 2) as i32;
    let image = match crate::dxgi_capture::capture_monitor_at(cx, cy) {
        // A real desktop is never fully black (the taskbar/chrome alone has
        // non-black pixels), so an all-black DXGI frame means duplication handed
        // us an empty / metadata-only frame; don't ship it, fall through to GDI
        // (which has its own black-retry, `capture_image_robust`). Applies on
        // both Windows 10 and 11 — the empty-first-frame race happens on both.
        Some(img) if !is_blank_black(&img) => {
            log::info!("capture path: DXGI Desktop Duplication");
            fit_to(img, w, h)
        }
        other => {
            if other.is_some() {
                log::warn!("DXGI frame came back fully black; falling back to GDI");
            }
            // Wayland first, where xcap's path is not merely slower but
            // unusable: it asks GNOME to encode the whole screen as a PNG into
            // /tmp and reads it back, which measured five to nine seconds and
            // rang the shutter every time. See `wayland_screencast`.
            match screencast_image(monitor_count) {
                Some(img) => img,
                None => {
                    log::info!("capture path: GDI (xcap)");
                    capture_image_robust(&m).context("capturing screen")?
                }
            }
        }
    };
    // xcap is inconsistent about units on a scaled Wayland session: the
    // monitor rect comes back in LOGICAL pixels while `capture_image` hands
    // back a PHYSICAL-pixel buffer. On this 3200x2000 panel at GNOME's 166%
    // that is 1920x1200 vs 3200x2000 — every consumer that crops the image
    // with these coordinates, or sizes the overlay from them, ends up off by
    // the scale factor (the overlay rendered at roughly a third of the screen).
    //
    // Normalise the rect onto the image's own pixel grid so `area` and `image`
    // always describe the same space. Where xcap already agrees — Windows, and
    // any unscaled session — the ratio is 1 and this is a no-op.
    let (x, y, w, h) = normalise_to_image(x, y, w, h, &image);

    Ok(CapturedScreen {
        area: ScreenRect::new(x, y, w, h),
        image,
        monitor_label: friendly,
    })
}

/// Resize a capture to the monitor's expected (w, h) when it differs (e.g.
/// DXGI returns physical pixels while the rest of the app works in the
/// monitor's reported coordinate space). A no-op when they already match,
/// which is the common case on a DPI-aware process.
fn fit_to(img: RgbaImage, w: u32, h: u32) -> RgbaImage {
    if img.width() == w && img.height() == h {
        img
    } else {
        log::info!(
            "capture {}x{} fitted to monitor {}x{}",
            img.width(),
            img.height(),
            w,
            h
        );
        image::imageops::resize(&img, w, h, image::imageops::FilterType::Triangle)
    }
}

/// Capture a monitor, retrying if the frame comes back fully black.
///
/// xcap's Windows backend uses the DXGI Desktop Duplication API, whose
/// freshly-acquired frame is occasionally an all-black/empty buffer — the
/// duplication hasn't produced a real frame yet (most common right after
/// the app starts, after an idle period, or when nothing on screen changed
/// recently). The release build runs the capture sooner after the click,
/// which makes the race surface more often. A genuinely all-black monitor
/// is effectively impossible in normal use (the taskbar/chrome alone has
/// non-black pixels), so retrying when the whole frame is black can't
/// misfire on real content. The sleeps only run on the (rare) black path,
/// so a normal snip pays no extra latency.
fn capture_image_robust(m: &xcap::Monitor) -> anyhow::Result<RgbaImage> {
    let mut image = m.capture_image().context("capturing screen")?;
    for attempt in 1..=3u32 {
        if !is_blank_black(&image) {
            return Ok(image);
        }
        log::warn!("capture came back fully black (attempt {attempt}); retrying");
        std::thread::sleep(std::time::Duration::from_millis(40));
        match m.capture_image() {
            Ok(next) => image = next,
            Err(e) => {
                log::warn!("black-capture retry failed: {e}");
                break;
            }
        }
    }
    Ok(image)
}

/// True if every sampled pixel is pure black (R=G=B=0) — i.e. the capture
/// looks like an empty/blank DXGI frame rather than real screen content.
/// Samples a coarse grid instead of every pixel: scanning a full 4K frame
/// on each snip would add real latency, while a grid reliably catches an
/// all-black frame. A single non-black sample short-circuits to `false`.
fn is_blank_black(img: &RgbaImage) -> bool {
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return true;
    }
    let step_x = (w / 64).max(1);
    let step_y = (h / 64).max(1);
    let mut y = 0;
    while y < h {
        let mut x = 0;
        while x < w {
            let p = img.get_pixel(x, y);
            if p[0] != 0 || p[1] != 0 || p[2] != 0 {
                return false;
            }
            x += step_x;
        }
        y += step_y;
    }
    true
}

/// Lightweight lookup: which monitor's friendly label contains the given
/// virtual-screen point? Doesn't capture pixels — used to refresh the
/// `monitor_label` on a floating snip while the user drags it across
/// monitors. Returns None if no monitor matches (very rare).
pub fn monitor_label_at(x: i32, y: i32) -> Option<String> {
    let monitors = xcap::Monitor::all().ok()?;
    monitors.iter().enumerate().find_map(|(idx, m)| {
        let mx = m.x();
        let my = m.y();
        let mw = m.width() as i32;
        let mh = m.height() as i32;
        if x >= mx && x < mx + mw && y >= my && y < my + mh {
            let raw = m.name();
            let label = if raw.starts_with("\\\\") || raw.is_empty() {
                format!("{} {}", crate::i18n::t("Monitor"), idx + 1)
            } else {
                format!("{} {} · {}", crate::i18n::t("Monitor"), idx + 1, raw)
            };
            Some(label)
        } else {
            None
        }
    })
}

/// [`monitor_label_at`], falling back to the primary monitor's label (or the
/// first's) when the point lands on none — the same fallback [`capture_at`]
/// makes.
///
/// It lands on none routinely on a scaled Wayland session: the point is in
/// physical pixels while xcap reports monitor rects in logical ones, so the
/// right and bottom of the screen lie outside every rect.
#[cfg(target_os = "linux")]
pub fn monitor_label_or_primary(x: i32, y: i32) -> Option<String> {
    monitor_label_at(x, y).or_else(|| {
        let monitors = xcap::Monitor::all().ok()?;
        let m = monitors
            .iter()
            .find(|m| m.is_primary())
            .or(monitors.first())?;
        monitor_label_at(m.x(), m.y())
    })
}

/// Crop a sub-rect (in virtual-screen coordinates) out of a captured screen.
pub fn crop(captured: &CapturedScreen, region: ScreenRect) -> anyhow::Result<RgbaImage> {
    let (iw, ih) = (captured.image.width(), captured.image.height());

    // Translate from screen coords to image-local coords, clamped to what was
    // actually captured.
    //
    // Dragging into the edge of the screen routinely lands a pixel or two
    // outside it: the pointer is reported in the overlay's logical space and
    // scaled up to physical, so rounding alone can push the far corner past the
    // last row. Rejecting the whole region for that lost the user's snip with
    // no visible reason — the drag simply produced nothing. Clamping costs a
    // sliver nobody selected on purpose.
    //
    // The clamp is an intersection: a region that starts left of or above the
    // capture loses that overhang from its size too. Clamping only the start
    // kept the full width and height from there, which added a strip of pixels
    // past the far edge that the selection never covered.
    let area = captured.area;
    let left = i64::from(region.x).max(i64::from(area.x));
    let top = i64::from(region.y).max(i64::from(area.y));
    let right = (i64::from(region.x) + i64::from(region.w)).min(i64::from(area.x) + i64::from(iw));
    let bottom =
        (i64::from(region.y) + i64::from(region.h)).min(i64::from(area.y) + i64::from(ih));
    if right <= left || bottom <= top {
        return Err(anyhow!("region does not overlap the captured screen"));
    }
    let (lx, ly) = ((left - i64::from(area.x)) as u32, (top - i64::from(area.y)) as u32);
    let (w, h) = ((right - left) as u32, (bottom - top) as u32);
    if w != region.w || h != region.h {
        log::debug!(
            "region {}x{} at ({},{}) clamped to {w}x{h} at ({lx},{ly}) inside {iw}x{ih}",
            region.w,
            region.h,
            region.x,
            region.y
        );
    }

    let view = captured.image.view(lx, ly, w, h);
    Ok(view.to_image())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 100×80 capture of the monitor at (1000, 0), each pixel's red and
    /// green holding its own image coordinates.
    fn screen() -> CapturedScreen {
        CapturedScreen {
            area: ScreenRect::new(1000, 0, 100, 80),
            image: RgbaImage::from_fn(100, 80, |x, y| image::Rgba([x as u8, y as u8, 0, 255])),
            monitor_label: String::new(),
        }
    }

    #[test]
    fn region_inside_is_cut_exactly() {
        let img = crop(&screen(), ScreenRect::new(1010, 20, 30, 10)).unwrap();
        assert_eq!(img.dimensions(), (30, 10));
        assert_eq!(img.get_pixel(0, 0).0[..2], [10, 20]);
    }

    #[test]
    fn overhang_before_the_capture_shrinks_the_region() {
        // Starts 5 px left of and 3 px above the monitor: only the 25×7 that
        // overlaps it was selected.
        let img = crop(&screen(), ScreenRect::new(995, -3, 30, 10)).unwrap();
        assert_eq!(img.dimensions(), (25, 7));
        assert_eq!(img.get_pixel(0, 0).0[..2], [0, 0]);
    }

    #[test]
    fn overhang_past_the_capture_is_cut_off() {
        let img = crop(&screen(), ScreenRect::new(1090, 75, 30, 10)).unwrap();
        assert_eq!(img.dimensions(), (10, 5));
        assert_eq!(img.get_pixel(0, 0).0[..2], [90, 75]);
    }

    #[test]
    fn region_off_the_capture_is_an_error() {
        assert!(crop(&screen(), ScreenRect::new(900, 0, 50, 50)).is_err());
        assert!(crop(&screen(), ScreenRect::new(1100, 0, 50, 50)).is_err());
        assert!(crop(&screen(), ScreenRect::new(1000, 80, 50, 50)).is_err());
    }
}
