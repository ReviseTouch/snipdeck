//! Cross-platform snip selector overlay (Slint).
//!
//! This is the cross-platform alternative to `overlay.rs` (which is a
//! Win32 native layered window). On Mac & Linux X11 this is the path
//! that works; on Wayland it works as long as the compositor honours
//! `always-on-top` + transparent borderless windows.
//!
//! Mirror of the Win32 overlay's public surface, but tracks the
//! per-monitor area Rust supplies. App code can switch backends by
//! swapping which type it stores in `AppState`.
//!
//! Limitations vs the Win32 overlay:
//!   - Not click-through. The overlay window receives mouse events;
//!     since the snip drag is driven by the rdev global hook anyway,
//!     this is usually fine — events reach both the overlay (ignored)
//!     and the hook (used).
//!   - No `freeze_screen_during_snip` mode yet (the Win32 path takes a
//!     full snapshot and shows it underneath). For Slint we'd render
//!     the captured RgbaImage as a background `Image` in the overlay;
//!     left as a TODO.

use slint::{Color, ComponentHandle, Image, PhysicalPosition, Rgba8Pixel, SharedPixelBuffer};

use crate::snip::ScreenRect;
use crate::OverlayWindow;

pub struct SlintOverlayManager {
    window: Option<OverlayWindow>,
    /// Monitor rect this overlay was opened for (physical pixels).
    /// Used to translate global screen coords into window-local coords.
    monitor: Option<ScreenRect>,
    scale_factor: f32,
    border_color: (u8, u8, u8),
}

impl SlintOverlayManager {
    pub fn new() -> Self {
        Self {
            window: None,
            monitor: None,
            scale_factor: 1.0,
            border_color: (255, 136, 0),
        }
    }

    pub fn set_border_color(&mut self, r: u8, g: u8, b: u8) {
        self.border_color = (r, g, b);
        if let Some(w) = &self.window {
            w.set_border_color(Color::from_rgb_u8(r, g, b));
        }
    }

    /// Pop the overlay over the given monitor `area`. `scale_factor` is the OS
    /// scale (e.g. 1.5 on a HiDPI display) — Slint widths are in logical pixels,
    /// so we divide. When `frozen` is `Some(img)` (Win+Shift+Space "freeze
    /// first" mode) that snapshot is shown as the backdrop and the selection
    /// happens on the still image; `None` keeps the overlay transparent over the
    /// live desktop (the normal Win+click path).
    pub fn show_for_monitor(
        &mut self,
        area: ScreenRect,
        scale_factor: f32,
        frozen: Option<&image::RgbaImage>,
    ) -> Result<(), slint::PlatformError> {
        self.scale_factor = if scale_factor > 0.001 {
            scale_factor
        } else {
            1.0
        };
        self.monitor = Some(area);

        // Reuse the existing window if we already have one — avoids a
        // visible "close + reopen" flicker between consecutive snips.
        let win = match self.window.take() {
            Some(w) => w,
            None => OverlayWindow::new()?,
        };

        let sf = self.scale_factor;
        win.set_outer_w(area.w as f32 / sf);
        win.set_outer_h(area.h as f32 / sf);
        let (r, g, b) = self.border_color;
        win.set_border_color(Color::from_rgb_u8(r, g, b));
        win.set_selecting(false);
        win.set_sel_w(0.0);
        win.set_sel_h(0.0);
        match frozen {
            Some(img) => {
                let buf = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
                    img.as_raw(),
                    img.width(),
                    img.height(),
                );
                win.set_frozen_bg(Image::from_rgba8(buf));
                win.set_frozen(true);
            }
            None => win.set_frozen(false),
        }
        win.window()
            .set_position(PhysicalPosition::new(area.x, area.y));
        win.show()?;

        self.window = Some(win);
        Ok(())
    }

    /// Update the selection rectangle. Coordinates are in *global*
    /// physical-pixel screen space (i.e. what rdev/winit hand us).
    pub fn update_sel(&self, x1: i32, y1: i32, x2: i32, y2: i32) {
        let (Some(win), Some(monitor)) = (&self.window, &self.monitor) else {
            return;
        };
        let sf = self.scale_factor;
        let sx = (x1.min(x2) - monitor.x) as f32 / sf;
        let sy = (y1.min(y2) - monitor.y) as f32 / sf;
        let sw = ((x1 - x2).abs()) as f32 / sf;
        let sh = ((y1 - y2).abs()) as f32 / sf;
        win.set_sel_x(sx);
        win.set_sel_y(sy);
        win.set_sel_w(sw);
        win.set_sel_h(sh);
        win.set_selecting(true);
    }

    pub fn hide(&mut self) {
        if let Some(w) = self.window.take() {
            let _ = w.hide();
        }
        self.monitor = None;
    }
}
