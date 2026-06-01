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
    use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
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

        // Acquire the current frame. The first call after duplication usually
        // returns the desktop immediately, but can report WAIT_TIMEOUT if
        // nothing changed yet — retry briefly (the user just moved/clicked,
        // so a frame is almost always available at once). Bounded so a
        // pathologically static screen falls back to GDI instead of hanging.
        let mut frame_info = DXGI_OUTDUPL_FRAME_INFO::default();
        let mut resource: Option<IDXGIResource> = None;
        let mut acquired = false;
        for _ in 0..4 {
            match dup.AcquireNextFrame(150, &mut frame_info, &mut resource) {
                Ok(()) => {
                    acquired = true;
                    break;
                }
                Err(e) if e.code() == DXGI_ERROR_WAIT_TIMEOUT => continue,
                Err(e) => return Err(e).context("AcquireNextFrame"),
            }
        }
        if !acquired {
            return Err(anyhow!("AcquireNextFrame kept timing out"));
        }
        let resource = resource.ok_or_else(|| anyhow!("null desktop resource"))?;
        let frame_tex: ID3D11Texture2D = resource.cast().context("frame as texture")?;

        let mut tex_desc = D3D11_TEXTURE2D_DESC::default();
        frame_tex.GetDesc(&mut tex_desc);

        if tex_desc.Format != DXGI_FORMAT_B8G8R8A8_UNORM {
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
            let row = std::slice::from_raw_parts(src.add(y * row_pitch), width * 4);
            let dst = &mut buf[y * width * 4..(y + 1) * width * 4];
            // BGRA -> RGBA; force opaque alpha (duplication alpha is undefined).
            for x in 0..width {
                let s = &row[x * 4..x * 4 + 4];
                let d = &mut dst[x * 4..x * 4 + 4];
                d[0] = s[2];
                d[1] = s[1];
                d[2] = s[0];
                d[3] = 255;
            }
        }

        context.Unmap(&staging, 0);
        let _ = dup.ReleaseFrame();

        RgbaImage::from_raw(width as u32, height as u32, buf)
            .ok_or_else(|| anyhow!("RgbaImage::from_raw failed"))
    }
}
