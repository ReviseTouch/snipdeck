//! Screen colour sampling for the eyedropper / colour-picker tool.
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

#[cfg(not(target_os = "windows"))]
pub fn capture_region(_cx: i32, _cy: i32, _half: i32) -> Option<image::RgbaImage> {
    None
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
