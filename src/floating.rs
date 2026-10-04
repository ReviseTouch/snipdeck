//! Floating snip window management.
//!
//! Each pinned/floating snip gets its own `slint::Window` instance
//! (a `FloatingWindow` component). They live in `FloatingManager`,
//! keyed by snip UUID. Position changes from drag are written back
//! into `Snip::last_position` so they persist across sessions.

use slint::{ComponentHandle, Image, LogicalPosition, PhysicalPosition, SharedPixelBuffer};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use uuid::Uuid;

use crate::settings::Settings;
use crate::snip::Snip;
use crate::FloatingWindow;

/// State held in the closures so callbacks can talk back to the
/// manager without `&mut FloatingManager`.
struct SharedState {
    /// Drag deltas accumulated since last drain.
    pending_pos_writes: HashMap<Uuid, (i32, i32)>,
    /// Right-click events that should pop up a menu (snip id + screen pos).
    pending_menu: Vec<(Uuid, i32, i32)>,
    /// Middle-click closes the snip window.
    pending_close: Vec<Uuid>,
    /// Snips pressed since last drain — the most recent becomes the
    /// selected (highlighted) snip.
    pending_select: Vec<Uuid>,
    /// Resize gestures that finished — app should re-capture the underlying
    /// screen region and update the snip's image + area.
    pending_resize_end: Vec<Uuid>,
    /// Snips whose floating window has been dragged at least once. The
    /// resize handles are only meaningful when the window is still at
    /// its original captured position, so once moved we hide them.
    moved_snips: HashSet<Uuid>,
}

pub struct FloatingHandle {
    pub window: FloatingWindow,
}

impl FloatingHandle {
    /// Current outer rect of the window, in physical screen pixels.
    /// `(x, y, w, h)` where x/y is the top-left corner.
    pub fn physical_rect(&self) -> (i32, i32, i32, i32) {
        let pos = self.window.window().position();
        let sf = self.window.window().scale_factor();
        let w = (self.window.get_outer_w() * sf).round() as i32;
        let h = (self.window.get_outer_h() * sf).round() as i32;
        (pos.x, pos.y, w, h)
    }
}

pub struct FloatingManager {
    pub windows: HashMap<Uuid, FloatingHandle>,
    /// Active when the session cannot host a window per snip and every snip is
    /// drawn inside one surface instead — see `window_env::HostStrategy` and
    /// `snip_surface`. When this is `Some`, `windows` stays empty and every
    /// operation below forwards to it.
    #[cfg(target_os = "linux")]
    surface: Option<crate::snip_surface::SurfaceHost>,
    shared: Rc<RefCell<SharedState>>,
    /// The snip whose window is currently "selected" — its border shows the
    /// active colour while every other floating snip shows the inactive one,
    /// so the user can tell which snip is in focus among several on screen.
    selected: Option<Uuid>,
}

/// Minimum allowed outer size in physical pixels — resize handles clamp
/// at this so the user can't shrink a snip to nothing by accident.
const MIN_OUTER_PHYS: i32 = 32;

impl FloatingManager {
    pub fn new() -> Self {
        // OPT-IN ONLY, via `SNIPDECK_SNIP_HOST=surface`.
        //
        // `host_strategy()` already names SingleSurface as the right answer for
        // this session, and the host is wired end to end — but it is not fit to
        // be the default yet. Two things are unfinished, and one may be
        // unfixable:
        //
        //   - dragging loses snips and misbehaves near the surface edges;
        //   - the co-ordinate spaces still disagree (the surface is sized from
        //     xcap's logical monitor extents while snips are placed from
        //     physical capture coordinates);
        //   - and the host surface is itself an ordinary window, so on GNOME it
        //     cannot stay above other windows. Drawing the snips correctly does
        //     not make them behave like Snipdeck if the whole surface drops
        //     behind the focused app.
        //
        // Defaulting to it would trade a working-but-limited setup for a broken
        // one, so it stays behind the flag until it earns the default.
        #[cfg(target_os = "linux")]
        let surface = {
            if matches!(std::env::var("SNIPDECK_SNIP_HOST").as_deref(), Ok("surface")) {
                log::info!("floating snips hosted in a single surface (opt-in)");
                Some(crate::snip_surface::SurfaceHost::new())
            } else {
                None
            }
        };

        Self {
            windows: HashMap::new(),
            #[cfg(target_os = "linux")]
            surface,
            shared: Rc::new(RefCell::new(SharedState {
                pending_pos_writes: HashMap::new(),
                pending_menu: Vec::new(),
                pending_close: Vec::new(),
                pending_resize_end: Vec::new(),
                pending_select: Vec::new(),
                moved_snips: HashSet::new(),
            })),
            selected: None,
        }
    }

    pub fn open(
        &mut self,
        snip: &Snip,
        settings: &Settings,
        scale_factor: f32,
        owner_window: Option<&slint::Window>,
    ) -> Result<(), slint::PlatformError> {
        #[cfg(target_os = "linux")]
        if let Some(surface) = self.surface.as_mut() {
            if let Err(e) = surface.open(snip, scale_factor) {
                log::warn!("surface host could not show snip: {e:#}");
            }
            return Ok(());
        }

        if self.windows.contains_key(&snip.id) {
            return Ok(());
        }
        let win = FloatingWindow::new()?;

        // Initial position (physical screen coords).
        let pos = snip
            .last_position
            .unwrap_or((snip.area.x - 3, snip.area.y - 3));

        // Sized with the scale of the monitor the snip opens on, where that
        // can be asked (Windows), not the main window's: see
        // `screen::scale_at`.
        let centre = (
            pos.0 + (snip.area.w / 2) as i32,
            pos.1 + (snip.area.h / 2) as i32,
        );
        let scale_factor = crate::screen::scale_at(centre.0, centre.1).unwrap_or(scale_factor);
        let sf = if scale_factor > 0.001 {
            scale_factor
        } else {
            1.0
        };

        // Snip image as Slint texture.
        if let Ok(img) = snip.load_image() {
            let buf = SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
                img.as_raw(),
                img.width(),
                img.height(),
            );
            win.set_snip_image(Image::from_rgba8(buf));
        }

        // Slint's `width`/`height` properties are in logical pixels.
        // Convert the snip's physical pixel dimensions through the
        // OS scale factor so the OS window ends up exactly snip-pixel
        // sized on screen, regardless of HiDPI. The border (3 physical
        // pixels each side) is sent as a logical-pixel length too so
        // the inner image area aligns exactly with the captured pixels.
        let border_phys = 3.0_f32;
        let border_logical = border_phys / sf;
        let outer_w = (snip.area.w as f32 + border_phys * 2.0) / sf;
        let outer_h = (snip.area.h as f32 + border_phys * 2.0) / sf;
        win.set_outer_w(outer_w);
        win.set_outer_h(outer_h);
        win.set_border_len(border_logical);
        win.set_pinned(snip.pinned);

        let active = settings.border_color_active;
        win.set_border_active(slint::Color::from_rgb_u8(active[0], active[1], active[2]));
        // Distinct highlight used only while a snip is selected. Kept separate
        // from the (orange) normal colour so the selected snip stands out.
        win.set_border_selected(slint::Color::from_rgb_u8(0x2a, 0xd4, 0xff));

        // Snips that have been previously dragged shouldn't get the
        // re-capture handles back — those only make sense while the
        // window still sits over its original screen region.
        let already_moved = snip.last_position.is_some();
        win.set_show_handles(!already_moved);
        if already_moved {
            self.shared.borrow_mut().moved_snips.insert(snip.id);
        }

        // Publish the intended position in the title, where the GNOME extension
        // can read it. `set_position` below is silently ignored on Wayland, so
        // without this the snip lands wherever the compositor likes; the
        // extension does the placing and raising on our behalf.
        //
        // Harmless where it is not needed: the title is never shown (these
        // windows are borderless) and Windows only ever matches on the prefix.
        if !crate::window_env::caps().absolute_position {
            // `listed` carries the taskbar setting across to the shell
            // extension, which is the only party on this platform that can act
            // on it. On Windows the same setting becomes WS_EX_TOOLWINDOW; on
            // GNOME it decides whether a snip appears in the Activities
            // overview and the window switcher. Same user intent, two very
            // different levers, one setting.
            //
            // It sits BEFORE the coordinates on purpose: the extension parses
            // those with a pattern anchored to the end of the title, so
            // anything appended after them would break it.
            let listed = if settings.floating_in_taskbar { " listed" } else { "" };
            let title = format!("SnipdeckFloating{listed} {},{}", pos.0, pos.1);
            // The one place the app and the GNOME extension have to agree. When
            // a snip opens somewhere unexpected, this line says whether the app
            // asked for the right place or the compositor ignored it.
            log::info!(
                "publishing placement in title: {title:?} (area={},{} last_position={:?} sf={})",
                snip.area.x,
                snip.area.y,
                snip.last_position,
                sf
            );
            win.set_win_title(slint::SharedString::from(title));
        } else {
            log::info!("absolute_position=true; title stays the bare marker");
        }
        win.window()
            .set_position(PhysicalPosition::new(pos.0, pos.1));
        crate::window_tamer::configure_floating_window(
            win.window(),
            owner_window,
            settings.floating_in_taskbar,
        );

        // Wire callbacks.
        let snip_id = snip.id;
        // Compositor-driven move. Wayland forbids a client from placing its own
        // window, so `on_drag_delta` below is dead there — but `xdg_toplevel.move`
        // exists precisely for this: ask the compositor to take over the drag and
        // it follows the pointer for us. winit exposes it as `drag_window()`, and
        // it must be requested from the press, before any motion.
        //
        // X11 gets the same treatment (winit maps it to _NET_WM_MOVERESIZE),
        // which also hands off to the window manager rather than fighting it.
        // Windows keeps the delta path: it works, and it is what the rest of the
        // Win32 floating-window behaviour is built around.
        #[cfg(not(target_os = "windows"))]
        {
            let shared_drag = self.shared.clone();
            let weak_drag = win.as_weak();
            win.on_drag_start(move || {
                if !crate::window_env::caps().interactive_move {
                    return;
                }
                let Some(c) = weak_drag.upgrade() else { return };
                use i_slint_backend_winit::WinitWindowAccessor;
                let started = c
                    .window()
                    .with_winit_window(|w| w.drag_window().is_ok())
                    .unwrap_or(false);
                if !started {
                    log::debug!("compositor refused the interactive move");
                    return;
                }
                // The compositor owns the drag from here; no motion events reach
                // us, so mark the snip moved now rather than on the first delta.
                // Not in crop mode: there the handles are the crop, and hiding
                // them left the snip stuck cropping with no way to finish.
                if shared_drag.borrow_mut().moved_snips.insert(snip_id) && !c.get_cropping() {
                    c.set_show_handles(false);
                }
            });
        }

        let shared = self.shared.clone();
        let weak = win.as_weak();
        win.on_drag_delta(move |dx, dy| {
            crate::pump::wake();
            if let Some(c) = weak.upgrade() {
                // The delta is in logical pixels, the window position in
                // physical ones. Added as they came, the snip covered 1/scale
                // of the pointer's travel each step and trailed behind it on
                // any display not at 100%.
                let sf = c.window().scale_factor();
                let pos = c.window().position();
                let nx = pos.x + (dx * sf).round() as i32;
                let ny = pos.y + (dy * sf).round() as i32;
                c.window().set_position(PhysicalPosition::new(nx, ny));
                let mut sh = shared.borrow_mut();
                sh.pending_pos_writes.insert(snip_id, (nx, ny));
                // First drag → the snip has been moved, hide the re-capture
                // handles — except in crop mode, where they are the crop.
                if sh.moved_snips.insert(snip_id) && !c.get_cropping() {
                    c.set_show_handles(false);
                }
            }
        });

        let shared_menu = self.shared.clone();
        let weak_menu = win.as_weak();
        win.on_right_pressed(move |lx, ly| {
            crate::pump::wake();
            if let Some(c) = weak_menu.upgrade() {
                // Logical pointer offset → physical, like the position it is
                // added to; unscaled, the menu opened up and to the left of
                // the click on a scaled display.
                let sf = c.window().scale_factor();
                let pos = c.window().position();
                let sx = pos.x + (lx * sf).round() as i32;
                let sy = pos.y + (ly * sf).round() as i32;
                shared_menu
                    .borrow_mut()
                    .pending_menu
                    .push((snip_id, sx, sy));
            }
        });

        let shared_close = self.shared.clone();
        win.on_middle_pressed(move || {
            crate::pump::wake();
            shared_close.borrow_mut().pending_close.push(snip_id);
        });

        let shared_select = self.shared.clone();
        win.on_pressed(move || {
            crate::pump::wake();
            shared_select.borrow_mut().pending_select.push(snip_id);
        });

        // Resize and crop: the side index drives which edge(s) move. We apply
        // the geometry to the OS window directly so the user sees live
        // feedback; app.rs persists the resize, or commits the crop, when
        // resize-end fires. Every move is computed from the snapshot taken at
        // the press — see `ResizeStart` for why adding moves up ran away.
        let gesture: Rc<Cell<Option<ResizeStart>>> = Rc::new(Cell::new(None));

        let (gesture_start, weak_start) = (gesture.clone(), win.as_weak());
        win.on_resize_start(move |side| {
            let Some(c) = weak_start.upgrade() else {
                return;
            };
            let pos = c.window().position();
            let (l, t) = (c.get_clip_x(), c.get_clip_y());
            // The crop is drawn relative to where the full image would sit.
            // Re-derived from where the window is now, so a snip moved since
            // crop mode began crops in place instead of jumping back.
            if c.get_cropping() {
                c.set_crop_anchor_x(pos.x - l);
                c.set_crop_anchor_y(pos.y - t);
            }
            gesture_start.set(Some(ResizeStart {
                side,
                pos: (pos.x, pos.y),
                size: (c.get_outer_w(), c.get_outer_h()),
                clip: (l, t, l + c.get_clip_w(), t + c.get_clip_h()),
            }));
        });

        let (gesture_delta, weak_res) = (gesture.clone(), win.as_weak());
        win.on_resize_delta(move |_side, dx, dy| {
            crate::pump::wake();
            let Some(c) = weak_res.upgrade() else {
                return;
            };
            // A move with no press behind it has nothing to be measured from.
            let Some(start) = gesture_delta.get() else {
                return;
            };
            let sf = c.window().scale_factor();
            let sf = if sf > 0.001 { sf } else { 1.0 };
            let pos = c.window().position();
            let offset = pointer_offset(
                &start,
                (dx, dy),
                (pos.x, pos.y),
                (c.get_outer_w(), c.get_outer_h()),
                sf,
            );
            // Crop mode: the same handles drag inward only and update the
            // source-clip preview instead of stretching the snip.
            if c.get_cropping() {
                let offset_px = ((offset.0 * sf).round() as i32, (offset.1 * sf).round() as i32);
                let clip = cropped(&start, offset_px, (c.get_img_w(), c.get_img_h()));
                show_clip(&c, clip, sf);
                return;
            }
            let ((x, y), (w, h)) = resized(&start, offset, sf, MIN_OUTER_PHYS as f32 / sf);
            c.set_outer_w(w);
            c.set_outer_h(h);
            c.window().set_position(PhysicalPosition::new(x, y));
        });

        let shared_resize_end = self.shared.clone();
        win.on_resize_end(move || {
            crate::pump::wake();
            gesture.set(None);
            shared_resize_end
                .borrow_mut()
                .pending_resize_end
                .push(snip_id);
        });

        // Fix the surface size BEFORE mapping. Setting `outer-w`/`outer-h`
        // alone only takes effect when Slint first lays the window out, which
        // happens after the compositor has already mapped and placed it — the
        // window is centred at its default size and then grows down and right,
        // leaving it visibly off-centre. Sizing the window up front means the
        // placement GNOME picks is the placement for the size we actually want.
        win.window()
            .set_size(slint::LogicalSize::new(outer_w, outer_h));
        win.show()?;
        // NOTE: this never resolves an HWND on the winit backend (the native
        // window handle isn't exposed through raw-window-handle here — logs
        // show "not available yet" even after show). The style work actually
        // happens in window_tamer's OBJECT_CREATE hook, which fires while the
        // window is still hidden. Kept as a harmless fallback for backends
        // where the handle does resolve.
        crate::window_tamer::configure_floating_window(
            win.window(),
            owner_window,
            settings.floating_in_taskbar,
        );

        self.windows.insert(
            snip.id,
            FloatingHandle {
                window: win,
            },
        );
        // Note: a freshly opened snip is intentionally NOT auto-selected —
        // selection is purely user-driven (a press), so nothing shows the
        // highlight colour until the user actually picks a snip.
        Ok(())
    }

    pub fn close(&mut self, id: Uuid) {
        #[cfg(target_os = "linux")]
        if let Some(surface) = self.surface.as_mut() {
            surface.close(id);
            self.shared.borrow_mut().moved_snips.remove(&id);
            if self.selected == Some(id) {
                self.selected = None;
            }
            return;
        }
        if let Some(h) = self.windows.remove(&id) {
            let _ = h.window.hide();
        }
        self.shared.borrow_mut().moved_snips.remove(&id);
        if self.selected == Some(id) {
            self.selected = None;
        }
    }

    pub fn close_all(&mut self) {
        #[cfg(target_os = "linux")]
        if let Some(surface) = self.surface.as_mut() {
            surface.close_all();
            self.shared.borrow_mut().moved_snips.clear();
            self.selected = None;
            return;
        }
        for (_, h) in self.windows.drain() {
            let _ = h.window.hide();
        }
        self.shared.borrow_mut().moved_snips.clear();
        self.selected = None;
    }

    /// Record the user-selected snip. The highlight is only *shown* while
    /// Snipdeck is the foreground app (see `refresh_selection_display`), so
    /// this just stores the intent.
    pub fn select(&mut self, id: Uuid) {
        #[cfg(target_os = "linux")]
        if let Some(surface) = self.surface.as_mut() {
            surface.select(id);
            self.selected = Some(id);
            return;
        }
        self.selected = Some(id);
    }

    /// Apply the selection highlight to the live windows. While Snipdeck is
    /// in the background (`app_active == false`) the selection is reset and
    /// every snip falls back to its default border — so a highlight never
    /// lingers on the desktop while the user is working in another app.
    pub fn refresh_selection_display(&mut self, app_active: bool) {
        if !app_active {
            self.selected = None;
        }
        for (wid, h) in self.windows.iter() {
            h.window.set_selected(self.selected == Some(*wid));
        }
    }

    pub fn toggle(
        &mut self,
        snip: &Snip,
        settings: &Settings,
        scale_factor: f32,
        owner_window: Option<&slint::Window>,
    ) {
        if self.windows.contains_key(&snip.id) {
            self.close(snip.id);
        } else {
            let _ = self.open(snip, settings, scale_factor, owner_window);
        }
    }

    pub fn refresh_pin(&mut self, id: Uuid, pinned: bool) {
        if let Some(h) = self.windows.get(&id) {
            h.window.set_pinned(pinned);
        }
    }

    /// Is this snip currently shown on screen?
    ///
    /// Callers used to reach into `windows` directly, which pinned the whole
    /// app to "a floating snip is an OS window". It is not, on every platform:
    /// where the compositor refuses to place or stack windows, snips are drawn
    /// inside a single host surface instead and there is no per-snip window to
    /// look up. Asking the question instead of inspecting the storage lets that
    /// backend swap out underneath.
    pub fn is_shown(&self, id: Uuid) -> bool {
        #[cfg(target_os = "linux")]
        if let Some(surface) = &self.surface {
            return surface.is_shown(id);
        }
        self.windows.contains_key(&id)
    }

    /// Is anything shown on screen right now?
    pub fn any_shown(&self) -> bool {
        #[cfg(target_os = "linux")]
        if let Some(surface) = &self.surface {
            return surface.any_shown();
        }
        !self.windows.is_empty()
    }

    /// How many snips are on screen.
    pub fn shown_count(&self) -> usize {
        #[cfg(target_os = "linux")]
        if let Some(surface) = &self.surface {
            return surface.shown_count();
        }
        self.windows.len()
    }

    pub fn handle(&self, id: Uuid) -> Option<&FloatingHandle> {
        self.windows.get(&id)
    }

    /// Re-enable the resize handles on an already-open floating window.
    /// Used by the "Resize" context menu action so the user
    /// can adjust a snip's region after the window has been dragged.
    pub fn enable_resize_handles(&mut self, id: Uuid) {
        if let Some(h) = self.windows.get(&id) {
            h.window.set_show_handles(true);
        }
        self.shared.borrow_mut().moved_snips.remove(&id);
    }

    /// Enter crop mode on an open floating window. Resets the window to a
    /// native 1:1 view of the image (so the crop maths is pixel-exact even if
    /// the snip had been stretched via the resize handles), seeds the clip
    /// rect to the full image, and records the on-screen anchor. The resize
    /// handles then drag inward only; `app.rs` commits the real pixel crop
    /// when the gesture ends.
    pub fn enable_crop(&mut self, id: Uuid, img_w: u32, img_h: u32) {
        if let Some(h) = self.windows.get(&id) {
            let c = &h.window;
            let sf = c.window().scale_factor();
            let sf = if sf > 0.001 { sf } else { 1.0 };
            let border = c.get_border_len();
            c.set_outer_w(img_w as f32 / sf + border * 2.0);
            c.set_outer_h(img_h as f32 / sf + border * 2.0);
            let pos = c.window().position();
            c.set_img_w(img_w as i32);
            c.set_img_h(img_h as i32);
            c.set_clip_x(0);
            c.set_clip_y(0);
            c.set_clip_w(img_w as i32);
            c.set_clip_h(img_h as i32);
            c.set_crop_anchor_x(pos.x);
            c.set_crop_anchor_y(pos.y);
            c.set_show_handles(true);
            c.set_cropping(true);
        }
        self.shared.borrow_mut().moved_snips.remove(&id);
    }

    /// Drain any queued events (position writes from drag, menu requests,
    /// close requests, resize-end signals).
    pub fn drain(&mut self) -> DrainResult {
        // The surface host reports the same events through its own channel;
        // merge them so the app loop never learns which host is running.
        #[cfg(target_os = "linux")]
        if let Some(surface) = self.surface.as_mut() {
            let ev = surface.drain();
            let mut position_updates = HashMap::new();
            for (id, x, y) in ev.moved {
                position_updates.insert(id, (x, y));
            }
            return DrainResult {
                position_updates,
                menu_requests: ev.menu_requests,
                close_requests: ev.close_requests,
                // Resize handles are not implemented on the surface host yet.
                resize_ends: Vec::new(),
                select_requests: ev.select_requests,
            };
        }

        let mut sh = self.shared.borrow_mut();
        DrainResult {
            position_updates: std::mem::take(&mut sh.pending_pos_writes),
            menu_requests: std::mem::take(&mut sh.pending_menu),
            close_requests: std::mem::take(&mut sh.pending_close),
            resize_ends: std::mem::take(&mut sh.pending_resize_end),
            select_requests: std::mem::take(&mut sh.pending_select),
        }
    }

    /// Let the active host reconcile with the compositor. No-op unless the
    /// single-surface host is running, where the surface has to adopt its real
    /// size and refresh which rectangles accept input.
    pub fn sync(&mut self) {
        #[cfg(target_os = "linux")]
        if let Some(surface) = self.surface.as_mut() {
            surface.sync();
        }
    }

    /// Does the host need [`FloatingManager::sync`] every tick? Only the
    /// single-surface host does; real windows wake the pump themselves.
    pub fn needs_ticks(&self) -> bool {
        #[cfg(target_os = "linux")]
        {
            self.surface.is_some()
        }
        #[cfg(not(target_os = "linux"))]
        {
            false
        }
    }
}

pub struct DrainResult {
    pub position_updates: HashMap<Uuid, (i32, i32)>,
    pub menu_requests: Vec<(Uuid, i32, i32)>,
    pub close_requests: Vec<Uuid>,
    pub resize_ends: Vec<Uuid>,
    pub select_requests: Vec<Uuid>,
}

// Silence unused warning for symmetry with eframe build.
#[allow(dead_code)]
fn _logical_position_marker(_p: LogicalPosition) {}

/// Smallest crop, in image pixels, on either axis.
const CROP_MIN: i32 = 16;

/// The geometry a resize or crop gesture started from.
///
/// Every move recomputes from this, never from the previous move. A handle
/// reports the pointer's offset since the press measured from where the handle
/// is *now*, and the handle rides along with the edge it sits on. On Windows and
/// X11 the window follows the drag, so each report is roughly the latest motion
/// alone; on Wayland the left and top edges cannot move at all — a client may
/// not place its own window — so each report carries the whole gesture. Adding
/// reports up was right for the first and ran away on the second: a left-edge
/// crop clipped 1+2+3+… pixels per event and committed a 16-px sliver over the
/// snip's only image. Recovering the pointer's real offset (`pointer_offset`)
/// and applying it to this snapshot is right on both.
#[derive(Clone, Copy, Debug, PartialEq)]
struct ResizeStart {
    /// 0=NW 1=N 2=NE 3=E 4=SE 5=S 6=SW 7=W.
    side: i32,
    /// Window position, physical pixels. Always 0,0 on Wayland, which is
    /// harmless: it is only ever compared with another reading of itself.
    pos: (i32, i32),
    /// `outer-w` × `outer-h`, logical pixels.
    size: (f32, f32),
    /// The crop clip as `(left, top, right, bottom)`, image pixels.
    clip: (i32, i32, i32, i32),
}

fn moves_left(side: i32) -> bool {
    matches!(side, 0 | 6 | 7)
}

fn moves_right(side: i32) -> bool {
    matches!(side, 2..=4)
}

fn moves_top(side: i32) -> bool {
    matches!(side, 0..=2)
}

fn moves_bottom(side: i32) -> bool {
    matches!(side, 4..=6)
}

/// The pointer's offset on screen since the press, logical pixels.
///
/// `reported` is the handle's own measurement, taken from wherever the handle
/// is now; `pos` and `size` are the window's current geometry. A left or top
/// handle has travelled with the window's origin, a right or bottom handle
/// with the origin plus the size — add that back and what is left is the
/// pointer.
fn pointer_offset(
    start: &ResizeStart,
    reported: (f32, f32),
    pos: (i32, i32),
    size: (f32, f32),
    sf: f32,
) -> (f32, f32) {
    let moved_x = (pos.0 - start.pos.0) as f32 / sf;
    let moved_y = (pos.1 - start.pos.1) as f32 / sf;
    let handle_x = if moves_right(start.side) {
        moved_x + (size.0 - start.size.0)
    } else if moves_left(start.side) {
        moved_x
    } else {
        0.0
    };
    let handle_y = if moves_bottom(start.side) {
        moved_y + (size.1 - start.size.1)
    } else if moves_top(start.side) {
        moved_y
    } else {
        0.0
    };
    (reported.0 + handle_x, reported.1 + handle_y)
}

/// Window geometry for a resize: position in physical pixels, size in logical.
///
/// The edge opposite each dragged one stays where it was — including when the
/// minimum size stops the drag, because the origin is derived from the
/// clamped size rather than from the pointer.
fn resized(
    start: &ResizeStart,
    offset: (f32, f32),
    sf: f32,
    min_log: f32,
) -> ((i32, i32), (f32, f32)) {
    let (mut w, mut h) = start.size;
    if moves_left(start.side) {
        w -= offset.0;
    } else if moves_right(start.side) {
        w += offset.0;
    }
    if moves_top(start.side) {
        h -= offset.1;
    } else if moves_bottom(start.side) {
        h += offset.1;
    }
    let w = w.max(min_log);
    let h = h.max(min_log);
    let x = if moves_left(start.side) {
        start.pos.0 + ((start.size.0 - w) * sf).round() as i32
    } else {
        start.pos.0
    };
    let y = if moves_top(start.side) {
        start.pos.1 + ((start.size.1 - h) * sf).round() as i32
    } else {
        start.pos.1
    };
    ((x, y), (w, h))
}

/// Clip rect for a crop, as `(left, top, right, bottom)` in image pixels.
///
/// Each dragged edge moves by the pointer's offset, inward only, and the rect
/// never shrinks below `CROP_MIN` — or below the image itself, for a snip
/// smaller than that.
fn cropped(start: &ResizeStart, offset_px: (i32, i32), img: (i32, i32)) -> (i32, i32, i32, i32) {
    let (l0, t0, r0, b0) = start.clip;
    let min_w = CROP_MIN.min(img.0);
    let min_h = CROP_MIN.min(img.1);
    let clamp = |v: i32, lo: i32, hi: i32| v.min(hi).max(lo);
    let (mut l, mut t, mut r, mut b) = (l0, t0, r0, b0);
    if moves_left(start.side) {
        l = clamp(l0 + offset_px.0, 0, r0 - min_w);
    } else if moves_right(start.side) {
        r = clamp(r0 + offset_px.0, l0 + min_w, img.0);
    }
    if moves_top(start.side) {
        t = clamp(t0 + offset_px.1, 0, b0 - min_h);
    } else if moves_bottom(start.side) {
        b = clamp(b0 + offset_px.1, t0 + min_h, img.1);
    }
    (l, t, r, b)
}

/// Show a crop clip: the kept region at 1:1, the window shrunk around it and
/// anchored where crop mode began. The `source-clip` binding does the preview.
fn show_clip(c: &FloatingWindow, (l, t, r, b): (i32, i32, i32, i32), sf: f32) {
    c.set_clip_x(l);
    c.set_clip_y(t);
    c.set_clip_w(r - l);
    c.set_clip_h(b - t);
    let border = c.get_border_len();
    c.set_outer_w((r - l) as f32 / sf + border * 2.0);
    c.set_outer_h((b - t) as f32 / sf + border * 2.0);
    c.window().set_position(PhysicalPosition::new(
        c.get_crop_anchor_x() + l,
        c.get_crop_anchor_y() + t,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn start(side: i32, pos: (i32, i32), size: (f32, f32), clip: (i32, i32, i32, i32)) -> ResizeStart {
        ResizeStart {
            side,
            pos,
            size,
            clip,
        }
    }

    /// The reported bug, replayed: a left-edge crop on Wayland, where the window
    /// cannot move, so the handle stays put and every report carries the whole
    /// gesture. Adding reports up clipped 5+10+15+… pixels; the clip has to
    /// follow the pointer and nothing else.
    #[test]
    fn wayland_left_crop_follows_the_pointer_instead_of_accumulating() {
        let s = start(7, (0, 0), (1006.0, 606.0), (0, 0, 1000, 600));
        for step in 1..=10 {
            let pointer = 5.0 * step as f32;
            // Window never moves on Wayland; the left handle never moves with it.
            let offset = pointer_offset(&s, (pointer, 0.0), (0, 0), (1006.0 - pointer, 606.0), 1.0);
            let (l, t, r, b) = cropped(&s, (offset.0 as i32, 0), (1000, 600));
            assert_eq!((l, t, r, b), (pointer as i32, 0, 1000, 600), "step {step}");
        }
    }

    /// Windows and X11: the window follows a left-edge drag, so the handle
    /// moves under the pointer and reports only what is left over. The result
    /// must be the same pointer-driven size as on Wayland.
    #[test]
    fn following_window_left_resize_tracks_the_pointer() {
        let sf = 2.0;
        let s = start(7, (100, 100), (200.0, 150.0), (0, 0, 0, 0));
        let (mut pos, mut size) = (s.pos, s.size);
        for step in 1..=8 {
            let pointer = -4.0 * step as f32;
            let moved = (pos.0 - s.pos.0) as f32 / sf;
            let offset = pointer_offset(&s, (pointer - moved, 0.0), pos, size, sf);
            let (p, z) = resized(&s, offset, sf, 16.0);
            assert!((z.0 - (200.0 - pointer)).abs() < 1e-3, "step {step}: width {}", z.0);
            (pos, size) = (p, z);
        }
    }

    /// A right-edge handle rides along with the width on every platform.
    #[test]
    fn right_resize_subtracts_the_handles_own_travel() {
        let s = start(3, (0, 0), (300.0, 200.0), (0, 0, 0, 0));
        let mut size = s.size;
        for step in 1..=6 {
            let pointer = 7.0 * step as f32;
            let travelled = size.0 - s.size.0;
            let offset = pointer_offset(&s, (pointer - travelled, 0.0), (0, 0), size, 1.0);
            let (_, z) = resized(&s, offset, 1.0, 16.0);
            assert!((z.0 - (300.0 + pointer)).abs() < 1e-3, "step {step}");
            size = z;
        }
    }

    /// Dragging the left edge past the minimum must not slide the window: the
    /// right edge stays where it was.
    #[test]
    fn minimum_size_keeps_the_opposite_edge_in_place() {
        let sf = 1.5;
        let s = start(7, (300, 40), (100.0, 100.0), (0, 0, 0, 0));
        let min_log = MIN_OUTER_PHYS as f32 / sf;
        let ((x, y), (w, _)) = resized(&s, (500.0, 0.0), sf, min_log);
        assert_eq!(w, min_log);
        assert_eq!(y, 40);
        let right = x as f32 + w * sf;
        assert!((right - (300.0 + 100.0 * sf)).abs() <= 1.0, "right edge moved to {right}");
    }

    /// A snip smaller than `CROP_MIN` cannot be cropped past itself.
    #[test]
    fn tiny_image_crop_stays_inside_the_image() {
        for side in 0..8 {
            let s = start(side, (0, 0), (16.0, 16.0), (0, 0, 10, 8));
            for offset in [(-100, -100), (100, 100)] {
                let (l, t, r, b) = cropped(&s, offset, (10, 8));
                assert!(0 <= l && l < r && r <= 10, "side {side}: x {l}..{r}");
                assert!(0 <= t && t < b && b <= 8, "side {side}: y {t}..{b}");
            }
        }
    }
}
