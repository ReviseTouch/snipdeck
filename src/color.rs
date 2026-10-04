//! Colour: screen sampling for the eyedropper / colour-picker tool, and the
//! HSV colour wheel the annotation editor and the collage share.
//!
//! Reads a small square of desktop pixels around a screen point via GDI
//! `BitBlt` — enough to feed the magnifier loupe and to read the exact pixel
//! under the cursor. GDI sees the composited desktop, which is what an
//! eyedropper wants. (Same caveat as the GDI snip path: a few
//! hardware-protected / HDR surfaces may not read back faithfully.)

/// Capture a `(2*half+1)` square of desktop pixels centred on the screen point
/// `(cx, cy)`. Returns RGBA (opaque). `None` on any GDI failure.
#[cfg(target_os = "windows")]
pub fn capture_region(cx: i32, cy: i32, half: i32) -> Option<image::RgbaImage> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
        GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
        HGDIOBJ, SRCCOPY,
    };
    let side = half * 2 + 1;
    if side <= 0 {
        return None;
    }
    unsafe {
        let screen = GetDC(HWND::default());
        if screen.is_invalid() {
            return None;
        }
        let mem = CreateCompatibleDC(screen);
        let bmp = CreateCompatibleBitmap(screen, side, side);
        // Select our bitmap, blit the screen region into it, then select the
        // ORIGINAL back — GetDIBits requires the bitmap NOT be selected in a DC.
        let old = SelectObject(mem, HGDIOBJ(bmp.0));
        let blit_ok =
            BitBlt(mem, 0, 0, side, side, screen, cx - half, cy - half, SRCCOPY).is_ok();
        SelectObject(mem, old);

        let mut bi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: side,
                biHeight: -side, // negative => top-down rows
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut buf = vec![0u8; (side * side * 4) as usize];
        let lines = if blit_ok {
            GetDIBits(
                mem,
                bmp,
                0,
                side as u32,
                Some(buf.as_mut_ptr() as *mut core::ffi::c_void),
                &mut bi,
                DIB_RGB_COLORS,
            )
        } else {
            0
        };

        // Always free GDI resources — this runs every loupe frame.
        let _ = DeleteObject(HGDIOBJ(bmp.0));
        let _ = DeleteDC(mem);
        ReleaseDC(HWND::default(), screen);

        if lines == 0 {
            return None;
        }
        // GDI gives BGRA with undefined alpha; convert to opaque RGBA.
        for px in buf.chunks_exact_mut(4) {
            px.swap(0, 2);
            px[3] = 255;
        }
        image::RgbaImage::from_raw(side as u32, side as u32, buf)
    }
}

/// Off Windows there is no cheap per-pixel desktop read (GDI's `GetDIBits`
/// has no portable equivalent), so grab the monitor under the point through
/// xcap and crop out the window.
///
/// NOTE this is a FULL-SCREEN readback per call. That is fine for a one-shot
/// `pixel_rgb` pick, but it makes the live loupe substantially heavier than
/// the Windows GDI path, which reads only the few hundred pixels it needs.
#[cfg(not(target_os = "windows"))]
pub fn capture_region(cx: i32, cy: i32, half: i32) -> Option<image::RgbaImage> {
    let half = half.max(0);
    let side = (half * 2 + 1) as u32;

    // Go through `capture::capture_at` rather than picking a monitor here.
    //
    // This used to search for the monitor containing (cx, cy) by comparing
    // against `Monitor::width()/height()`, which xcap reports in LOGICAL pixels
    // — while cx/cy arrive from the pointer in PHYSICAL ones. On a 3200x2000
    // panel at 166% the monitor rect only reaches 1920x1200, so any pick in the
    // right or bottom of the screen matched no monitor at all and the eyedropper
    // returned None: no error, no colour, nothing. `capture_at` already resolves
    // the monitor, falls back to the primary, and hands back a rect that is in
    // the same space as its image.
    let captured = crate::capture::capture_at(cx, cy).ok()?;
    let (ox, oy) = (captured.area.x, captured.area.y);
    let shot = captured.image;

    // Clamp into the captured image so a pick right at a screen edge still
    // returns pixels instead of failing outright.
    let (iw, ih) = (shot.width() as i32, shot.height() as i32);
    if iw <= 0 || ih <= 0 {
        return None;
    }
    let left = (cx - ox - half).clamp(0, iw - 1);
    let top = (cy - oy - half).clamp(0, ih - 1);
    let w = side.min((iw - left) as u32);
    let h = side.min((ih - top) as u32);
    Some(image::imageops::crop_imm(&shot, left as u32, top as u32, w, h).to_image())
}

/// RGB of the single desktop pixel at a screen point.
pub fn pixel_rgb(cx: i32, cy: i32) -> Option<[u8; 3]> {
    let img = capture_region(cx, cy, 0)?;
    let p = img.get_pixel(0, 0);
    Some([p[0], p[1], p[2]])
}

/// "#RRGGBB" (uppercase).
pub fn hex(rgb: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2])
}

/// "rgb(r, g, b)".
pub fn rgb_str(rgb: [u8; 3]) -> String {
    format!("rgb({}, {}, {})", rgb[0], rgb[1], rgb[2])
}

// ---- Colour wheel (annotation editor, collage background) -------------------

/// HSV (h 0..360, s/v 0..1) → RGB.
pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [u8; 3] {
    let h = h.rem_euclid(360.0);
    let c = v * s;
    let x = c * (1.0 - (((h / 60.0) % 2.0) - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h as u32) / 60 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    [
        ((r + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((g + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((b + m) * 255.0).round().clamp(0.0, 255.0) as u8,
    ]
}

/// Generate the HSV colour-wheel texture (hue around, saturation outward, full
/// value). Transparent outside the disc, with a 1.5px anti-aliased edge.
pub fn make_color_wheel(tex: u32) -> slint::Image {
    let mut buf = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(tex, tex);
    let r = tex as f32 / 2.0;
    let data = buf.make_mut_slice();
    for y in 0..tex {
        for x in 0..tex {
            let dx = x as f32 + 0.5 - r;
            let dy = y as f32 + 0.5 - r;
            let dist = (dx * dx + dy * dy).sqrt();
            let idx = (y * tex + x) as usize;
            data[idx] = if dist <= r {
                let hue = dy.atan2(dx).to_degrees().rem_euclid(360.0);
                let sat = (dist / r).min(1.0);
                let [rr, gg, bb] = hsv_to_rgb(hue, sat, 1.0);
                let a = if dist > r - 1.5 {
                    ((r - dist + 0.5).clamp(0.0, 1.0) * 255.0) as u8
                } else {
                    255
                };
                slint::Rgba8Pixel { r: rr, g: gg, b: bb, a }
            } else {
                slint::Rgba8Pixel { r: 0, g: 0, b: 0, a: 0 }
            };
        }
    }
    slint::Image::from_rgba8(buf)
}
