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
        return capture_monitor(m.clone(), idx);
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
    let monitor = monitors.into_iter().nth(idx).unwrap();
    capture_monitor(monitor, idx)
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

fn capture_monitor(m: xcap::Monitor, idx: usize) -> anyhow::Result<CapturedScreen> {
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
            log::info!("capture path: GDI (xcap)");
            capture_image_robust(&m).context("capturing screen")?
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
    let lx = (region.x - captured.area.x).max(0) as u32;
    let ly = (region.y - captured.area.y).max(0) as u32;
    if lx >= iw || ly >= ih {
        return Err(anyhow!("region starts outside the captured screen"));
    }
    let w = region.w.min(iw - lx);
    let h = region.h.min(ih - ly);
    if w == 0 || h == 0 {
        return Err(anyhow!("region does not overlap the captured screen"));
    }
    if w != region.w || h != region.h {
        log::debug!(
            "region {}x{} at ({lx},{ly}) clamped to {w}x{h} inside {iw}x{ih}",
            region.w,
            region.h
        );
    }

    let view = captured.image.view(lx, ly, w, h);
    Ok(view.to_image())
}
