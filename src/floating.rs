//! Floating snip window management.
//!
//! Each pinned/floating snip gets its own `slint::Window` instance
//! (a `FloatingWindow` component). They live in `FloatingManager`,
//! keyed by snip UUID. Position changes from drag are written back
//! into `Snip::last_position` so they persist across sessions.

use slint::{ComponentHandle, Image, LogicalPosition, PhysicalPosition, SharedPixelBuffer};
use std::cell::RefCell;
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

        // Initial position (physical screen coords).
        let pos = snip
            .last_position
            .unwrap_or((snip.area.x - 3, snip.area.y - 3));

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
                if shared_drag.borrow_mut().moved_snips.insert(snip_id) {
                    c.set_show_handles(false);
                }
            });
        }

        let shared = self.shared.clone();
        let weak = win.as_weak();
        win.on_drag_delta(move |dx, dy| {
            crate::pump::wake();
            if let Some(c) = weak.upgrade() {
                let pos = c.window().position();
                let nx = pos.x + dx as i32;
                let ny = pos.y + dy as i32;
                c.window().set_position(PhysicalPosition::new(nx, ny));
                let mut sh = shared.borrow_mut();
                sh.pending_pos_writes.insert(snip_id, (nx, ny));
                // First drag → the snip has been moved, hide handles.
                if sh.moved_snips.insert(snip_id) {
                    c.set_show_handles(false);
                }
            }
        });

        let shared_menu = self.shared.clone();
        let weak_menu = win.as_weak();
        win.on_right_pressed(move |lx, ly| {
            crate::pump::wake();
            if let Some(c) = weak_menu.upgrade() {
                let pos = c.window().position();
                let sx = pos.x + lx as i32;
                let sy = pos.y + ly as i32;
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

        // Resize: side index drives which edge(s) move. We apply the
        // geometry change to the OS window directly (same idea as the
        // drag callback) so the user sees live feedback; the actual
        // re-capture from the screen happens in app.rs when resize-end
        // fires.
        let weak_res = win.as_weak();
        win.on_resize_delta(move |side, dx, dy| {
            crate::pump::wake();
            let Some(c) = weak_res.upgrade() else {
                return;
            };
            let sf = c.window().scale_factor();
            // Crop mode: the same handles drag inward only and update the
            // source-clip preview instead of stretching the snip.
            if c.get_cropping() {
                crop_delta(&c, side, dx, dy, sf);
                return;
            }
            let pos = c.window().position();
            // Work in logical pixels for outer_w/h (the property's unit)
            // but in physical pixels for position (what set_position wants).
            let mut new_x = pos.x;
            let mut new_y = pos.y;
            let mut new_w_log = c.get_outer_w();
            let mut new_h_log = c.get_outer_h();
            let dxp = (dx * sf).round() as i32;
            let dyp = (dy * sf).round() as i32;
            let dxl = dx;
            let dyl = dy;
            match side {
                0 => {
                    new_x += dxp;
                    new_y += dyp;
                    new_w_log -= dxl;
                    new_h_log -= dyl;
                }
                1 => {
                    new_y += dyp;
                    new_h_log -= dyl;
                }
                2 => {
                    new_y += dyp;
                    new_w_log += dxl;
                    new_h_log -= dyl;
                }
                3 => {
                    new_w_log += dxl;
                }
                4 => {
                    new_w_log += dxl;
                    new_h_log += dyl;
                }
                5 => {
                    new_h_log += dyl;
                }
                6 => {
                    new_x += dxp;
                    new_w_log -= dxl;
                    new_h_log += dyl;
                }
                7 => {
                    new_x += dxp;
                    new_w_log -= dxl;
                }
                _ => {}
            }
            // Clamp minimum size (in physical px → logical for the prop).
            let min_log = MIN_OUTER_PHYS as f32 / sf;
            if new_w_log < min_log {
                new_w_log = min_log;
            }
            if new_h_log < min_log {
                new_h_log = min_log;
            }
            c.set_outer_w(new_w_log);
            c.set_outer_h(new_h_log);
            c.window().set_position(PhysicalPosition::new(new_x, new_y));
        });

        let shared_resize_end = self.shared.clone();
        win.on_resize_end(move || {
            crate::pump::wake();
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

/// Live crop-handle drag. The clip rect (in image pixels) is the single
/// source of truth: each handle nudges its edge(s), clamped to stay within
/// `[0, img]` (so handles only ever move *inward*), and the window geometry
/// is derived from the clip rect anchored at the position captured when crop
/// mode began. The `source-clip` binding then previews exactly the kept
/// region at 1:1.
fn crop_delta(c: &FloatingWindow, side: i32, dx: f32, dy: f32, sf: f32) {
    let dxp = (dx * sf).round() as i32;
    let dyp = (dy * sf).round() as i32;
    let img_w = c.get_img_w();
    let img_h = c.get_img_h();
    let min = 16;
    let cl = |v: i32, lo: i32, hi: i32| v.min(hi).max(lo);

    let mut left = c.get_clip_x();
    let mut top = c.get_clip_y();
    let mut right = left + c.get_clip_w();
    let mut bottom = top + c.get_clip_h();

    // side index: 0=NW 1=N 2=NE 3=E 4=SE 5=S 6=SW 7=W
    match side {
        0 => {
            left = cl(left + dxp, 0, right - min);
            top = cl(top + dyp, 0, bottom - min);
        }
        1 => top = cl(top + dyp, 0, bottom - min),
        2 => {
            right = cl(right + dxp, left + min, img_w);
            top = cl(top + dyp, 0, bottom - min);
        }
        3 => right = cl(right + dxp, left + min, img_w),
        4 => {
            right = cl(right + dxp, left + min, img_w);
            bottom = cl(bottom + dyp, top + min, img_h);
        }
        5 => bottom = cl(bottom + dyp, top + min, img_h),
        6 => {
            left = cl(left + dxp, 0, right - min);
            bottom = cl(bottom + dyp, top + min, img_h);
        }
        7 => left = cl(left + dxp, 0, right - min),
        _ => {}
    }

    let clip_w = right - left;
    let clip_h = bottom - top;
    c.set_clip_x(left);
    c.set_clip_y(top);
    c.set_clip_w(clip_w);
    c.set_clip_h(clip_h);

    let border = c.get_border_len();
    c.set_outer_w(clip_w as f32 / sf + border * 2.0);
    c.set_outer_h(clip_h as f32 / sf + border * 2.0);
    let ax = c.get_crop_anchor_x();
    let ay = c.get_crop_anchor_y();
    c.window()
        .set_position(PhysicalPosition::new(ax + left, ay + top));
}
