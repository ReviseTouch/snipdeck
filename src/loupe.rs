//! Colour-picker (eyedropper) controller.
//!
//! A magnifier loupe window follows the cursor sampling desktop pixels, and a
//! result popup shows the picked colour (HEX + RGB + swatch). The actual click
//! that captures, and Esc to cancel, are handled by the mouse/keyboard hook
//! (see `mouse_trigger`), which sends us `ColorPick` / `Cancel` events.

use slint::ComponentHandle;

use crate::{ColorResultWindow, LoupeWindow};

/// Half-size of the sampled square: a (2*HALF+1) px region around the cursor.
const HALF: i32 = 7;
/// Upscaled loupe image edge, in pixels (nearest-neighbour, so pixels stay crisp).
const ZOOM: u32 = 150;

pub struct ColorPicker {
    loupe: Option<LoupeWindow>,
    result: Option<ColorResultWindow>,
    active: bool,
    /// Last sampled centre colour, used as a fallback if the final read fails.
    last: [u8; 3],
    /// Linux: the desktop portal's answer, once it has one. `Ok(None)` is the
    /// user dismissing the portal's picker.
    #[cfg(target_os = "linux")]
    portal: Option<crossbeam_channel::Receiver<Result<Option<[u8; 3]>, String>>>,
}

impl ColorPicker {
    pub fn new() -> Self {
        Self {
            loupe: None,
            result: None,
            active: false,
            last: [0, 0, 0],
            #[cfg(target_os = "linux")]
            portal: None,
        }
    }

    /// Linux: ask the desktop portal for a colour.
    ///
    /// The loupe below follows the cursor and waits for a click from the global
    /// pointer hook — none of which exists on Linux: Wayland tells no client
    /// where the pointer is or lets it read the screen, and the hook needed
    /// /dev/input. So this picker never worked there, and from the snip menu it
    /// left an empty loupe on screen with nothing able to close it. The portal
    /// shows the desktop's own picker (GNOME's is its screenshot UI) and hands
    /// back the colour; [`ColorPicker::poll`] shows it when it arrives.
    #[cfg(target_os = "linux")]
    pub fn pick_via_portal(&mut self) {
        if self.portal.is_some() {
            return; // already asking
        }
        log::info!("color picker: asking the desktop portal");
        let (tx, rx) = crossbeam_channel::bounded(1);
        let spawned = std::thread::Builder::new()
            .name("portal-color".into())
            .spawn(move || {
                let picked = pollster::block_on(async {
                    ashpd::desktop::Color::pick().send().await?.response()
                });
                let to_u8 = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
                let result = match picked {
                    Ok(c) => Ok(Some([to_u8(c.red()), to_u8(c.green()), to_u8(c.blue())])),
                    Err(ashpd::Error::Response(ashpd::desktop::ResponseError::Cancelled)) => {
                        Ok(None)
                    }
                    Err(e) => Err(e.to_string()),
                };
                log::info!("color picker: portal answered {result:?}");
                let _ = tx.send(result);
                crate::pump::wake();
            });
        if spawned.is_ok() {
            self.portal = Some(rx);
        }
    }

    /// Linux: show the portal's colour if it has answered. `Some(Err)` is a
    /// failure worth telling the user about; a dismissed picker is not.
    #[cfg(target_os = "linux")]
    pub fn poll(&mut self) -> Option<Result<(), String>> {
        let answer = self.portal.as_ref()?.try_recv().ok()?;
        self.portal = None;
        match answer {
            Ok(Some(rgb)) => {
                let (hex, rgbs) = (crate::color::hex(rgb), crate::color::rgb_str(rgb));
                if let Err(e) = crate::clipboard::copy_text(&hex) {
                    log::warn!("copy picked hex failed: {e}");
                }
                log::info!("color picked: {hex} {rgbs}");
                // Wayland ignores the position; the compositor places it.
                self.show_result(rgb, &hex, &rgbs, 0, 0);
                Some(Ok(()))
            }
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Enter pick mode: show the loupe and arm the hook so the next click grabs.
    #[cfg_attr(target_os = "linux", allow(dead_code))] // hook-driven; Linux picks through the portal
    pub fn begin(&mut self) {
        if self.loupe.is_none() {
            match LoupeWindow::new() {
                Ok(w) => self.loupe = Some(w),
                Err(e) => {
                    log::warn!("loupe window init failed: {e}");
                    return;
                }
            }
        }
        self.active = true;
        self.update(); // paint a first frame before the window appears
        if let Some(w) = &self.loupe {
            let _ = w.show();
            crate::window_env::keep_above(w);
        }
        crate::mouse_trigger::arm_color_pick();
        log::info!("color picker: armed");
    }

    /// Sample around the cursor and refresh the loupe. Call every tick while active.
    pub fn update(&mut self) {
        if !self.active {
            return;
        }
        let Some((cx, cy)) = cursor_pos() else {
            return;
        };
        let Some(region) = crate::color::capture_region(cx, cy, HALF) else {
            return;
        };
        let c = region.get_pixel(HALF as u32, HALF as u32);
        self.last = [c[0], c[1], c[2]];
        let up =
            image::imageops::resize(&region, ZOOM, ZOOM, image::imageops::FilterType::Nearest);
        if let Some(w) = &self.loupe {
            let buf = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
                up.as_raw(),
                ZOOM,
                ZOOM,
            );
            w.set_zoom(slint::Image::from_rgba8(buf));
            w.set_hex(crate::color::hex(self.last).into());
            w.set_swatch(slint::Color::from_rgb_u8(self.last[0], self.last[1], self.last[2]));
            // Offset below-right of the cursor so the loupe never covers the
            // pixel being sampled.
            w.window()
                .set_position(slint::PhysicalPosition::new(cx + 18, cy + 18));
        }
    }

    /// Leave pick mode without grabbing (Esc / external cancel).
    pub fn cancel(&mut self) {
        if !self.active {
            return;
        }
        self.active = false;
        crate::mouse_trigger::disarm_color_pick();
        if let Some(w) = &self.loupe {
            let _ = w.hide();
        }
        log::info!("color picker: cancelled");
    }

    /// Finalise a pick at the click point: copy the HEX, hide the loupe, and show
    /// the result popup.
    pub fn finish(&mut self, cx: i32, cy: i32) {
        self.active = false;
        crate::mouse_trigger::disarm_color_pick();
        if let Some(w) = &self.loupe {
            let _ = w.hide();
        }
        let rgb = crate::color::pixel_rgb(cx, cy).unwrap_or(self.last);
        let hex = crate::color::hex(rgb);
        let rgbs = crate::color::rgb_str(rgb);
        if let Err(e) = crate::clipboard::copy_text(&hex) {
            log::warn!("copy picked hex failed: {e}");
        }
        log::info!("color picked: {hex} {rgbs}");
        self.show_result(rgb, &hex, &rgbs, cx, cy);
    }

    fn show_result(&mut self, rgb: [u8; 3], hex: &str, rgbs: &str, cx: i32, cy: i32) {
        if self.result.is_none() {
            match ColorResultWindow::new() {
                Ok(w) => {
                    let weak = w.as_weak();
                    w.on_dismissed(move || {
                        if let Some(c) = weak.upgrade() {
                            let _ = c.hide();
                        }
                    });
                    let weak_h = w.as_weak();
                    w.on_copy_hex(move || {
                        if let Some(c) = weak_h.upgrade() {
                            let _ = crate::clipboard::copy_text(&c.get_hex().to_string());
                        }
                    });
                    let weak_r = w.as_weak();
                    w.on_copy_rgb(move || {
                        if let Some(c) = weak_r.upgrade() {
                            let _ = crate::clipboard::copy_text(&c.get_rgb().to_string());
                        }
                    });
                    self.result = Some(w);
                }
                Err(e) => {
                    log::warn!("color result window init failed: {e}");
                    return;
                }
            }
        }
        if let Some(w) = &self.result {
            w.set_hex(hex.into());
            w.set_rgb(rgbs.into());
            w.set_swatch(slint::Color::from_rgb_u8(rgb[0], rgb[1], rgb[2]));
            w.window()
                .set_position(slint::PhysicalPosition::new(cx + 18, cy + 18));
            let _ = w.show();
            crate::window_env::keep_above(w);
        }
    }
}

#[cfg(target_os = "windows")]
fn cursor_pos() -> Option<(i32, i32)> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
    let mut p = POINT::default();
    unsafe { GetCursorPos(&mut p).ok()? };
    Some((p.x, p.y))
}

#[cfg(not(target_os = "windows"))]
fn cursor_pos() -> Option<(i32, i32)> {
    None
}
