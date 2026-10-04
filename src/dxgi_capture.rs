//! DXGI Desktop Duplication screen capture.
//!
//! Our screen-capture crate (`xcap`) uses GDI `BitBlt` on Windows, which
//! cannot read GPU-composited / DirectComposition surfaces — the Warp
//! terminal, hardware-accelerated browsers, video players and games all
//! come out fully black. The Desktop Duplication API instead grabs the
//! actual frame DWM composited on the GPU, so those windows capture
//! correctly. We fall back to the GDI path only when duplication is
//! unavailable (the secure/UAC desktop, an HDR / non-BGRA output, or no
//! output matching the point). Genuinely DRM-protected content (e.g.
//! Netflix) is black under duplication too — that's enforced in hardware
//! and unavoidable.

/// Capture the full monitor whose desktop rect contains the given
/// virtual-screen point. Returns `None` (so the caller can fall back to the
/// GDI path) on any failure. The image is the monitor at its physical
/// resolution; the caller fits it to the expected logical size.
#[cfg(not(target_os = "windows"))]
pub fn capture_monitor_at(_cx: i32, _cy: i32) -> Option<image::RgbaImage> {
    None
}

#[cfg(target_os = "windows")]
pub fn capture_monitor_at(cx: i32, cy: i32) -> Option<image::RgbaImage> {
    match imp::capture(cx, cy) {
        Ok(img) => Some(img),
        Err(e) => {
            log::warn!("DXGI capture failed ({e}); falling back to GDI");
            None
        }
    }
}

#[cfg(target_os = "windows")]
mod imp {
    use anyhow::{anyhow, Context, Result};
    use image::RgbaImage;
    use windows::core::Interface;
    use windows::Win32::Foundation::HMODULE;
    use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_UNKNOWN, D3D_FEATURE_LEVEL};
    use windows::Win32::Graphics::Direct3D11::{
        D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D,
        D3D11_CPU_ACCESS_READ, D3D11_CREATE_DEVICE_FLAG, D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ,
        D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
    };
    use windows::Win32::Graphics::Dxgi::Common::{
        DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_R16G16B16A16_FLOAT, DXGI_MODE_ROTATION_ROTATE180,
        DXGI_MODE_ROTATION_ROTATE270, DXGI_MODE_ROTATION_ROTATE90,
    };
    use windows::Win32::Graphics::Dxgi::{
        CreateDXGIFactory1, IDXGIAdapter1, IDXGIFactory1, IDXGIOutput1, IDXGIOutputDuplication,
        IDXGIResource, DXGI_ERROR_WAIT_TIMEOUT, DXGI_OUTDUPL_FRAME_INFO,
    };

    pub fn capture(cx: i32, cy: i32) -> Result<RgbaImage> {
        unsafe {
            let factory: IDXGIFactory1 =
                CreateDXGIFactory1().context("CreateDXGIFactory1")?;

            let mut adapter_idx = 0u32;
            while let Ok(adapter) = factory.EnumAdapters1(adapter_idx) {
                adapter_idx += 1;
                let mut output_idx = 0u32;
                while let Ok(output) = adapter.EnumOutputs(output_idx) {
                    output_idx += 1;
                    let desc = output.GetDesc().context("IDXGIOutput::GetDesc")?;
                    let r = desc.DesktopCoordinates;
                    if cx >= r.left && cx < r.right && cy >= r.top && cy < r.bottom {
                        let output1: IDXGIOutput1 =
                            output.cast().context("cast IDXGIOutput1")?;
                        return capture_output(&adapter, &output1);
                    }
                }
            }
            Err(anyhow!("no DXGI output contains point ({cx},{cy})"))
        }
    }

    unsafe fn capture_output(
        adapter: &IDXGIAdapter1,
        output1: &IDXGIOutput1,
    ) -> Result<RgbaImage> {
        // A D3D11 device on this adapter. D3D_DRIVER_TYPE_UNKNOWN is
        // required whenever an explicit adapter is supplied.
        let mut device: Option<ID3D11Device> = None;
        let mut context: Option<ID3D11DeviceContext> = None;
        D3D11CreateDevice(
            adapter,
            D3D_DRIVER_TYPE_UNKNOWN,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_FLAG(0),
            None,
            D3D11_SDK_VERSION,
            Some(&mut device),
            Some(std::ptr::null_mut::<D3D_FEATURE_LEVEL>()),
            Some(&mut context),
        )
        .context("D3D11CreateDevice")?;
        let device = device.ok_or_else(|| anyhow!("null D3D11 device"))?;
        let context = context.ok_or_else(|| anyhow!("null D3D11 context"))?;

        let dup: IDXGIOutputDuplication =
            output1.DuplicateOutput(&device).context("DuplicateOutput")?;

        // Acquire a frame that actually carries a DESKTOP present.
        //
        // The first AcquireNextFrame after a fresh DuplicateOutput frequently
        // hands back an EMPTY surface (all zeros, LastPresentTime == 0) — the
        // duplication hasn't been populated by a real present yet. Copying that
        // gives a fully black snip (confirmed: centre pixel read back 00 00…).
        // Pointer/metadata-only updates (cursor moved, nothing else) are empty
        // the same way. So discard any frame with LastPresentTime == 0 and keep
        // acquiring until a real present lands or we hit the deadline. Bounded
        // by wall-clock (not attempt count) because metadata frames return
        // instantly and would otherwise burn the retries in microseconds. If no
        // present arrives (a truly static desktop), bail to the GDI fallback.
        let mut frame_info = DXGI_OUTDUPL_FRAME_INFO::default();
        let mut resource: Option<IDXGIResource> = None;
        let mut acquired = false;
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(300);
        while std::time::Instant::now() < deadline {
            // Release a metadata-only frame from the previous iteration before
            // re-acquiring, or AcquireNextFrame returns DXGI_ERROR_INVALID_CALL.
            if resource.take().is_some() {
                let _ = dup.ReleaseFrame();
            }
            match dup.AcquireNextFrame(50, &mut frame_info, &mut resource) {
                Ok(()) => {
                    if frame_info.LastPresentTime != 0 {
                        acquired = true;
                        break;
                    }
                }
                Err(e) if e.code() == DXGI_ERROR_WAIT_TIMEOUT => continue,
                Err(e) => return Err(e).context("AcquireNextFrame"),
            }
        }
        if !acquired {
            return Err(anyhow!("no desktop present within deadline (only empty/metadata frames)"));
        }
        let resource = resource.ok_or_else(|| anyhow!("null desktop resource"))?;
        let frame_tex: ID3D11Texture2D = resource.cast().context("frame as texture")?;

        let mut tex_desc = D3D11_TEXTURE2D_DESC::default();
        frame_tex.GetDesc(&mut tex_desc);

        // Accept the two formats the desktop duplication actually hands us:
        //   - B8G8R8A8_UNORM: the standard SDR desktop.
        //   - R16G16B16A16_FLOAT: an HDR desktop (scRGB, linear FP16). When HDR
        //     is ON, EVERY frame is this format — rejecting it sent us to the GDI
        //     fallback, which captures GPU/HDR windows as BLACK. We tone-map it
        //     to SDR 8-bit below instead.
        let is_hdr = tex_desc.Format == DXGI_FORMAT_R16G16B16A16_FLOAT;
        if tex_desc.Format != DXGI_FORMAT_B8G8R8A8_UNORM && !is_hdr {
            let _ = dup.ReleaseFrame();
            return Err(anyhow!("unsupported DXGI format {:?}", tex_desc.Format));
        }

        // CPU-readable staging copy of the GPU frame.
        let mut staging_desc = tex_desc;
        staging_desc.Usage = D3D11_USAGE_STAGING;
        staging_desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
        staging_desc.BindFlags = 0;
        staging_desc.MiscFlags = 0;

        let mut staging: Option<ID3D11Texture2D> = None;
        let staging_result =
            device.CreateTexture2D(&staging_desc, None, Some(&mut staging));
        if let Err(e) = staging_result {
            let _ = dup.ReleaseFrame();
            return Err(e).context("CreateTexture2D(staging)");
        }
        let staging = staging.ok_or_else(|| anyhow!("null staging texture"))?;

        context.CopyResource(&staging, &frame_tex);

        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        let map_result = context.Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped));
        if let Err(e) = map_result {
            let _ = dup.ReleaseFrame();
            return Err(e).context("Map(staging)");
        }

        let width = tex_desc.Width as usize;
        let height = tex_desc.Height as usize;
        let row_pitch = mapped.RowPitch as usize;
        let src = mapped.pData as *const u8;

        let mut buf = vec![0u8; width * height * 4];
        for y in 0..height {
            let dst = &mut buf[y * width * 4..(y + 1) * width * 4];
            let row_ptr = src.add(y * row_pitch);
            if is_hdr {
                // scRGB FP16: 8 bytes/pixel (4 halfs), linear, 1.0 = SDR white.
                // Tone-map to sRGB 8-bit (clip HDR highlights / wide gamut).
                let row = std::slice::from_raw_parts(row_ptr, width * 8);
                for x in 0..width {
                    let p = &row[x * 8..x * 8 + 8];
                    let r = half_to_f32(u16::from_le_bytes([p[0], p[1]]));
                    let g = half_to_f32(u16::from_le_bytes([p[2], p[3]]));
                    let b = half_to_f32(u16::from_le_bytes([p[4], p[5]]));
                    let d = &mut dst[x * 4..x * 4 + 4];
                    d[0] = linear_to_srgb8(r);
                    d[1] = linear_to_srgb8(g);
                    d[2] = linear_to_srgb8(b);
                    d[3] = 255;
                }
            } else {
                // B8G8R8A8_UNORM -> RGBA; opaque alpha (duplication alpha is undefined).
                let row = std::slice::from_raw_parts(row_ptr, width * 4);
                for x in 0..width {
                    let s = &row[x * 4..x * 4 + 4];
                    let d = &mut dst[x * 4..x * 4 + 4];
                    d[0] = s[2];
                    d[1] = s[1];
                    d[2] = s[0];
                    d[3] = 255;
                }
            }
        }

        context.Unmap(&staging, 0);
        let _ = dup.ReleaseFrame();

        let frame = RgbaImage::from_raw(width as u32, height as u32, buf)
            .ok_or_else(|| anyhow!("RgbaImage::from_raw failed"))?;
        // The duplicated frame comes in the panel's native orientation, not
        // the desktop's: on a monitor turned to portrait it is still landscape,
        // and fitting it to the portrait monitor squashed it sideways. Turn it
        // the way the desktop is turned — for ROTATE90 the frame's top row is
        // the desktop's right-hand column, a quarter turn clockwise (the
        // Desktop Duplication sample maps it the same way).
        Ok(match dup.GetDesc().Rotation {
            DXGI_MODE_ROTATION_ROTATE90 => image::imageops::rotate90(&frame),
            DXGI_MODE_ROTATION_ROTATE180 => image::imageops::rotate180(&frame),
            DXGI_MODE_ROTATION_ROTATE270 => image::imageops::rotate270(&frame),
            _ => frame,
        })
    }

    /// IEEE-754 binary16 (half) -> f32. Used to read scRGB FP16 HDR frames.
    fn half_to_f32(h: u16) -> f32 {
        let sign = (h >> 15) & 1;
        let exp = (h >> 10) & 0x1f;
        let mant = h & 0x3ff;
        let v = if exp == 0 {
            // Subnormal (or zero).
            (mant as f32) * 2f32.powi(-24)
        } else if exp == 0x1f {
            // Inf / NaN — treat as a large finite value; it'll clamp to white.
            if mant == 0 {
                65504.0
            } else {
                0.0
            }
        } else {
            (1.0 + (mant as f32) / 1024.0) * 2f32.powi(exp as i32 - 15)
        };
        if sign == 1 {
            -v
        } else {
            v
        }
    }

    /// Linear scRGB component (1.0 = SDR white) -> 8-bit sRGB. Clips HDR
    /// highlights (>1) to white and wide-gamut negatives to black, then applies
    /// the sRGB transfer curve — a plain SDR tone-map of the HDR desktop.
    fn linear_to_srgb8(v: f32) -> u8 {
        let v = v.clamp(0.0, 1.0);
        let s = if v <= 0.0031308 {
            v * 12.92
        } else {
            1.055 * v.powf(1.0 / 2.4) - 0.055
        };
        (s * 255.0 + 0.5).clamp(0.0, 255.0) as u8
    }
}
