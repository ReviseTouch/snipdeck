//! `SingleSurface` snip host: every floating snip drawn inside one surface.
//!
//! See `window_env::HostStrategy` for why this exists. The short version: on
//! GNOME/Mutter a client may not place, stack or locate its own windows, and no
//! layer-shell is offered to work around it. So the snips stop being windows.
//! One screen-sized surface holds all of them, and position and z-order become
//! layout — something we are entirely allowed to decide.
//!
//! The cost is that a screen-sized surface would eat every click meant for the
//! desktop. `wayland_input_region` confines pointer input to the snip
//! rectangles, so the gaps between them behave as if the surface were not
//! there. That call is the linchpin: without it this design is unusable, which
//! is why `Caps::input_region` gates the strategy.
//!
//! What this cannot recover, and no amount of drawing will: the host surface is
//! still one ordinary window, so it cannot sit above a fullscreen application
//! and it keeps a single taskbar entry. Those need `LayerShell`.

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use slint::{ComponentHandle, Image, Model, SharedPixelBuffer, SharedString, VecModel};
use uuid::Uuid;

use crate::snip::Snip;
use crate::wayland_input_region::InputRect;
use crate::{HostedSnip, SnipSurfaceWindow};

/// Events the surface collected since the last drain, mirroring the shape
/// `FloatingManager` already hands the app loop so the caller does not care
/// which host produced them.
#[derive(Default)]
pub struct SurfaceEvents {
    pub select_requests: Vec<Uuid>,
    /// `(id, x, y)` with x/y in PHYSICAL screen pixels, matching what the
    /// window-per-snip path reports.
    pub menu_requests: Vec<(Uuid, i32, i32)>,
    pub close_requests: Vec<Uuid>,
    /// Snips whose position changed, so the gallery can persist it.
    pub moved: Vec<(Uuid, i32, i32)>,
}

#[derive(Default)]
struct Shared {
    events: SurfaceEvents,
}

pub struct SurfaceHost {
    window: Option<SnipSurfaceWindow>,
    /// Draw order IS z-order: later entries paint over earlier ones, so raising
    /// a snip is a move to the end of this model rather than a request to the
    /// compositor.
    model: Rc<VecModel<HostedSnip>>,
    /// Parallel to `model`, because `HostedSnip.id` is a string for Slint's
    /// sake and parsing a UUID on every hit test would be wasteful.
    ids: Vec<Uuid>,
    shared: Rc<RefCell<Shared>>,
    selected: Option<Uuid>,
    moved_snips: HashSet<Uuid>,
    /// Logical size of the host surface, corrected from the compositor once it
    /// answers — see `sync`.
    surface_w: f32,
    surface_h: f32,
    scale: f32,
}

impl SurfaceHost {
    pub fn new() -> Self {
        Self {
            window: None,
            model: Rc::new(VecModel::default()),
            ids: Vec::new(),
            shared: Rc::new(RefCell::new(Shared::default())),
            selected: None,
            moved_snips: HashSet::new(),
            surface_w: 0.0,
            surface_h: 0.0,
            scale: 1.0,
        }
    }

    pub fn is_shown(&self, id: Uuid) -> bool {
        self.ids.contains(&id)
    }

    pub fn any_shown(&self) -> bool {
        !self.ids.is_empty()
    }

    pub fn shown_count(&self) -> usize {
        self.ids.len()
    }

    /// Put a snip on screen, or move an existing one back to its captured spot.
    pub fn open(&mut self, snip: &Snip, scale_factor: f32) -> anyhow::Result<()> {
        self.scale = if scale_factor > 0.001 { scale_factor } else { 1.0 };
        self.ensure_window()?;

        let image = match snip.load_image() {
            Ok(img) => {
                let buf = SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
                    img.as_raw(),
                    img.width(),
                    img.height(),
                );
                Image::from_rgba8(buf)
            }
            Err(e) => {
                return Err(anyhow::anyhow!("loading snip image: {e}"));
            }
        };

        // Captured coordinates are physical; the surface lays out in logical.
        let (px, py) = snip.last_position.unwrap_or((snip.area.x, snip.area.y));
        let hosted = HostedSnip {
            id: SharedString::from(snip.id.to_string()),
            x: px as f32 / self.scale,
            y: py as f32 / self.scale,
            w: snip.area.w as f32 / self.scale,
            h: snip.area.h as f32 / self.scale,
            image,
            selected: self.selected == Some(snip.id),
            pinned: snip.pinned,
        };

        if let Some(idx) = self.index_of(snip.id) {
            self.model.set_row_data(idx, hosted);
        } else {
            self.model.push(hosted);
            self.ids.push(snip.id);
        }
        if snip.last_position.is_some() {
            self.moved_snips.insert(snip.id);
        }

        self.refresh();
        Ok(())
    }

    pub fn close(&mut self, id: Uuid) {
        if let Some(idx) = self.index_of(id) {
            self.model.remove(idx);
            self.ids.remove(idx);
        }
        if self.selected == Some(id) {
            self.selected = None;
        }
        self.refresh();
    }

    pub fn close_all(&mut self) {
        while self.model.row_count() > 0 {
            self.model.remove(0);
        }
        self.ids.clear();
        self.selected = None;
        self.refresh();
    }

    /// Select a snip and raise it: in a single surface, "on top" is just the
    /// last row of the model.
    pub fn select(&mut self, id: Uuid) {
        let Some(idx) = self.index_of(id) else { return };
        if idx + 1 != self.ids.len() {
            if let Some(row) = self.model.row_data(idx) {
                self.model.remove(idx);
                self.ids.remove(idx);
                self.model.push(row);
                self.ids.push(id);
            }
        }
        self.selected = Some(id);
        self.apply_selection();
    }

    pub fn drain(&mut self) -> SurfaceEvents {
        std::mem::take(&mut self.shared.borrow_mut().events)
    }

    /// Adopt the compositor's idea of the surface size and refresh the input
    /// region. Cheap; call every tick while snips are shown.
    ///
    /// A surface does not know its real size until it has been configured, so
    /// the size set at creation is a guess that has to be corrected — the same
    /// pattern the snip overlay and the context menu need.
    pub fn sync(&mut self) {
        let Some(win) = &self.window else { return };
        let size = win.window().size();
        if size.width == 0 || size.height == 0 {
            return;
        }
        let sf = win.window().scale_factor().max(0.001);
        let (w, h) = (size.width as f32 / sf, size.height as f32 / sf);
        if (self.surface_w - w).abs() > 0.5 || (self.surface_h - h).abs() > 0.5 {
            self.surface_w = w;
            self.surface_h = h;
            win.set_outer_w(w);
            win.set_outer_h(h);
            log::debug!("snip surface is {w}x{h} logical (sf={sf})");
        }
        self.scale = sf;
        self.apply_input_region();
    }

    fn index_of(&self, id: Uuid) -> Option<usize> {
        self.ids.iter().position(|i| *i == id)
    }

    fn ensure_window(&mut self) -> anyhow::Result<()> {
        if self.window.is_some() {
            return Ok(());
        }
        let win = SnipSurfaceWindow::new()?;
        win.set_snips(self.model.clone().into());

        // Cover the monitor. xcap reports its extents in logical pixels on a
        // scaled session, so they go out as logical and Slint applies the scale
        // itself — passing them as physical is what left an earlier surface
        // covering 60% of the screen.
        if let Some((lw, lh)) = crate::screen::primary_logical_size() {
            win.window()
                .set_size(slint::LogicalSize::new(lw as f32, lh as f32));
            self.surface_w = lw as f32;
            self.surface_h = lh as f32;
            win.set_outer_w(lw as f32);
            win.set_outer_h(lh as f32);
        }

        self.wire(&win);
        win.show()?;
        self.window = Some(win);
        Ok(())
    }

    fn wire(&self, win: &SnipSurfaceWindow) {
        let shared = self.shared.clone();
        win.on_pressed(move |id| {
            crate::pump::wake();
            if let Ok(uuid) = Uuid::parse_str(&id) {
                shared.borrow_mut().events.select_requests.push(uuid);
            }
        });

        let shared = self.shared.clone();
        let model = self.model.clone();
        win.on_dragged(move |id, dx, dy| {
            crate::pump::wake();
            let Ok(uuid) = Uuid::parse_str(&id) else { return };
            // Moving a snip is editing our own layout — no compositor involved,
            // which is exactly why this works where the window-per-snip path
            // cannot move anything at all.
            for i in 0..model.row_count() {
                let Some(mut row) = model.row_data(i) else { continue };
                if row.id != id {
                    continue;
                }
                row.x += dx;
                row.y += dy;
                let (x, y) = (row.x, row.y);
                model.set_row_data(i, row);
                shared
                    .borrow_mut()
                    .events
                    .moved
                    .push((uuid, x as i32, y as i32));
                break;
            }
        });

        let shared = self.shared.clone();
        let model = self.model.clone();
        win.on_right_pressed(move |id, lx, ly| {
            crate::pump::wake();
            let Ok(uuid) = Uuid::parse_str(&id) else { return };
            // Unlike the window-per-snip path, the click's position on screen is
            // knowable here: the surface covers the screen and starts at its
            // origin, so surface-local coordinates ARE screen coordinates.
            let mut sx = lx;
            let mut sy = ly;
            for i in 0..model.row_count() {
                let Some(row) = model.row_data(i) else { continue };
                if row.id == id {
                    sx += row.x;
                    sy += row.y;
                    break;
                }
            }
            shared
                .borrow_mut()
                .events
                .menu_requests
                .push((uuid, sx as i32, sy as i32));
        });

        let shared = self.shared.clone();
        win.on_middle_pressed(move |id| {
            crate::pump::wake();
            if let Ok(uuid) = Uuid::parse_str(&id) {
                shared.borrow_mut().events.close_requests.push(uuid);
            }
        });
    }

    fn refresh(&mut self) {
        // With nothing to show the surface is pure obstruction: hide it so it
        // cannot take focus or sit in the window list for no reason.
        if self.ids.is_empty() {
            if let Some(win) = &self.window {
                let _ = win.hide();
            }
            return;
        }
        if let Some(win) = &self.window {
            let _ = win.show();
        }
        self.apply_selection();
        self.apply_input_region();
    }

    fn apply_selection(&self) {
        for i in 0..self.model.row_count() {
            let Some(mut row) = self.model.row_data(i) else { continue };
            let is_sel = self
                .ids
                .get(i)
                .is_some_and(|id| self.selected == Some(*id));
            if row.selected != is_sel {
                row.selected = is_sel;
                self.model.set_row_data(i, row);
            }
        }
    }

    /// Confine pointer input to the snips so the desktop behind stays usable.
    fn apply_input_region(&self) {
        let Some(win) = &self.window else { return };
        let rects: Vec<InputRect> = (0..self.model.row_count())
            .filter_map(|i| self.model.row_data(i))
            .map(|s| InputRect {
                x: s.x as i32,
                y: s.y as i32,
                w: s.w as i32,
                h: s.h as i32,
            })
            .collect();
        if let Err(e) = crate::wayland_input_region::set_input_rects(win.window(), &rects) {
            // Not fatal, but it means the surface is swallowing the desktop's
            // clicks — worth saying out loud rather than leaving the user to
            // discover it.
            log::warn!("snip surface input region failed ({e:#}); the surface will block clicks");
        }
    }
}

