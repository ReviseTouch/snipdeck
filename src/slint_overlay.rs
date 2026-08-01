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

use std::cell::Cell;
use std::rc::Rc;

use slint::{Color, ComponentHandle, Image, PhysicalPosition, Rgba8Pixel, SharedPixelBuffer};

use crate::mouse_trigger::{ModifierSet, MouseTriggerEvent};
use crate::snip::ScreenRect;
use crate::OverlayWindow;

pub struct SlintOverlayManager {
    window: Option<OverlayWindow>,
    /// Monitor rect this overlay was opened for (physical pixels).
    /// Used to translate global screen coords into window-local coords.
    monitor: Option<ScreenRect>,
    scale_factor: f32,
    border_color: (u8, u8, u8),
    /// Where overlay-driven selection events go: the SAME channel the global
    /// mouse hook feeds, so the entire selection/crop/floating pipeline works
    /// unchanged whether the drag came from the hook (Windows) or from the
    /// overlay's own TouchArea (Linux/Wayland, where global hooks don't
    /// exist). Shared with the callback closures.
    event_tx: Option<crossbeam_channel::Sender<MouseTriggerEvent>>,
    /// (monitor origin, scale factor) snapshot the input callbacks read to
    /// map window-local logical coords → global physical. Updated on every
    /// `show_for_monitor`.
    input_space: Rc<Cell<(i32, i32, f32)>>,
}

impl SlintOverlayManager {
    pub fn new() -> Self {
        Self {
            window: None,
            monitor: None,
            scale_factor: 1.0,
            border_color: (255, 136, 0),
            event_tx: None,
            input_space: Rc::new(Cell::new((0, 0, 1.0))),
        }
    }

    /// Provide the channel overlay-driven selection events are sent into
    /// (the mouse-trigger channel). Must be set before the first
    /// `show_for_monitor` for the input-capture path to do anything.
    pub fn set_event_sender(&mut self, tx: crossbeam_channel::Sender<MouseTriggerEvent>) {
        self.event_tx = Some(tx);
    }

    /// Wire the overlay-driven selection callbacks on a freshly created
    /// window. The overlay is recreated per snip (hide() drops it), so this
    /// runs once per window.
    fn wire_input(&self, win: &OverlayWindow) {
        let Some(tx) = self.event_tx.clone() else {
            return;
        };
        let space = self.input_space.clone();
        let to_global = move |lx: f32, ly: f32| {
            let (mx, my, sf) = space.get();
            (mx + (lx * sf) as i32, my + (ly * sf) as i32)
        };
        let send = move |ev: MouseTriggerEvent| {
            let _ = tx.send(ev);
            crate::pump::wake();
        };

        let (tg, snd) = (to_global.clone(), send.clone());
        win.on_sel_pressed(move |lx, ly| {
            let (x, y) = tg(lx, ly);
            snd(MouseTriggerEvent::StartSnip {
                x,
                y,
                mods: ModifierSet {
                    ctrl: false,
                    alt: false,
                    shift: false,
                },
            });
        });
        let (tg, snd) = (to_global.clone(), send.clone());
        win.on_sel_moved(move |lx, ly| {
            let (x, y) = tg(lx, ly);
            snd(MouseTriggerEvent::Move { x, y });
        });
        let (tg, snd) = (to_global.clone(), send.clone());
        win.on_sel_released(move |lx, ly| {
            let (x, y) = tg(lx, ly);
            snd(MouseTriggerEvent::Release { x, y });
        });
        let snd = send.clone();
        win.on_sel_cancelled(move || {
            snd(MouseTriggerEvent::Cancel);
        });
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
            None => {
                let w = OverlayWindow::new()?;
                self.wire_input(&w);
                w
            }
        };

        // Publish the coordinate space for the input callbacks and, on
        // platforms without a global mouse hook, let the overlay itself
        // capture the drag.
        self.input_space
            .set((area.x, area.y, self.scale_factor));
        win.set_input_capture(cfg!(not(target_os = "windows")));

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
        // Placement, then fullscreen — both, everywhere but Windows.
        //
        // The position picks the monitor: X11 (including XWayland) honours it
        // exactly, and Wayland silently ignores it, which costs nothing. The
        // fullscreen request is what actually sizes the surface, and it is the
        // only placement Wayland honours at all. Doing both means one code path
        // lands the overlay correctly on a Wayland session, on an X11 session,
        // and on the XWayland hybrid — none of which agree on what a window may
        // ask for.
        //
        // Sizing it by hand instead (`area.w / scale_factor`) is what produced
        // the third-of-the-screen overlay: the app's scale factor, the overlay
        // window's own scale factor and xcap's units were all different numbers
        // on a fractionally-scaled display. Fullscreen removes the arithmetic
        // rather than trying to get it right.
        //
        // Windows keeps the explicit rect: it is exact across a multi-monitor
        // virtual desktop, where fullscreen only ever covers one monitor.
        win.window()
            .set_position(PhysicalPosition::new(area.x, area.y));
        if !cfg!(target_os = "windows") {
            win.window().set_fullscreen(true);
        }
        win.show()?;
        // Route Escape to the overlay's FocusScope so the overlay-driven
        // path can cancel without the global keyboard hook.
        if cfg!(not(target_os = "windows")) {
            win.invoke_grab_keyboard();
        }

        self.window = Some(win);
        Ok(())
    }

    /// Re-read the window's real geometry into the layout properties.
    ///
    /// Fullscreen is granted asynchronously: the size reported straight after
    /// `show()` is still the pre-fullscreen one (often 0x0). Meanwhile the
    /// root element's width/height are bound to `outer-w`/`outer-h`, so the
    /// frozen backdrop and the dim panels keep whatever size we guessed and
    /// cover only part of the surface — the overlay looked cropped along the
    /// right and bottom edges.
    ///
    /// Rather than predict the size, adopt it: the compositor is the authority
    /// on how big a fullscreen surface is, and on what scale factor it carries
    /// (which need not match the main window's on a mixed X11/Wayland setup).
    /// Cheap enough to call every tick while the overlay is up.
    pub fn sync_to_window(&self) {
        let Some(win) = &self.window else { return };
        let size = win.window().size();
        if size.width == 0 || size.height == 0 {
            return;
        }
        let sf = win.window().scale_factor().max(0.001);
        let (w, h) = (size.width as f32 / sf, size.height as f32 / sf);
        if (win.get_outer_w() - w).abs() > 0.5 || (win.get_outer_h() - h).abs() > 0.5 {
            log::debug!("overlay resized to {}x{} logical (sf={sf})", w, h);
            win.set_outer_w(w);
            win.set_outer_h(h);
        }
        // Keep the input mapping on the same scale as the surface, or the drag
        // lands somewhere other than where the pointer is.
        if let Some(monitor) = self.monitor {
            self.input_space.set((monitor.x, monitor.y, sf));
        }
    }

    /// Update the selection rectangle. Coordinates are in *global*
    /// physical-pixel screen space (i.e. what rdev/winit hand us).
    pub fn update_sel(&self, x1: i32, y1: i32, x2: i32, y2: i32) {
        let (Some(win), Some(monitor)) = (&self.window, &self.monitor) else {
            return;
        };
        // The overlay's own scale factor, not the main window's: under XWayland
        // beside a fractionally-scaled Wayland session the two disagree, and
        // using the wrong one drew the selection rectangle offset from the
        // pointer.
        let sf = win.window().scale_factor().max(0.001);
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
