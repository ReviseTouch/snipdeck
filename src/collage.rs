//! Dynamic collage editor manager.
//!
//! Owns the `CollageWindow` and ALL collage geometry. The Slint side is a dumb
//! renderer: it shows a thumbnail tray (its own image picker, so no gallery
//! multi-selection is needed), reflects the computed cell rectangles, and
//! reports taps. Keeping the layout maths here means the on-screen preview and
//! the saved image come from the SAME code, so they match.
//!
//! Flow (mirrors `annotate.rs`):
//!   1. `open(sources, preselect, scale)` shows the editor over the snips.
//!   2. The user taps tray thumbnails to add/remove images and tweaks the
//!      layout (grid / justified rows / masonry), gap, corner radius, and
//!      background; the preview rebuilds live.
//!   3. On Save, `render_final` composites the selected images at full
//!      resolution (cover-fit + anti-aliased rounded corners) and stashes it.
//!   4. `take_result()` (drained each app tick) hands the new image to the app,
//!      which saves it as a brand-new snip.

use slint::{Brush, Color, ComponentHandle, Image, ModelRc, SharedPixelBuffer, SharedString, VecModel};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use uuid::Uuid;

use crate::{CollageCell, CollageWindow, TrayItem};

/// Working-canvas width (logical units). Layout is computed in [0..WORK_W] ×
/// [0..H]; the same geometry is scaled to the preview and to the final output,
/// so what you see is what you get.
const WORK_W: f32 = 1000.0;
/// Preview fit box (logical px) — the canvas is fit + centred inside this.
const AVAIL_W: f32 = 1040.0;
const AVAIL_H: f32 = 540.0;
/// Final output long edge (px); the short edge follows the collage aspect.
const OUT_LONG: f32 = 2048.0;

const MIN_COLS: u32 = 1;
const MAX_COLS: u32 = 8;
const DEFAULT_COLS: u32 = 3;
const DEFAULT_GAP: f32 = 8.0;
const MAX_GAP: f32 = 40.0;
const MAX_RADIUS: f32 = 80.0;

/// Colour-wheel display + texture size (matches `collage.slint`).
const WHEEL_DISP: f32 = 168.0;
const WHEEL_TEX: u32 = 256;

/// One image the editor can place. The thumbnail drives the tray + preview; the
/// full-resolution source is loaded from `path` only on Save.
pub struct CollageSource {
    pub id: Uuid,
    pub thumb: image::RgbaImage,
    pub path: PathBuf,
}

/// Internal per-image record (thumbnail already converted to a Slint texture).
struct Source {
    id: Uuid,
    thumb: Image,
    aspect: f32, // width / height
    path: PathBuf,
}

struct Shared {
    sources: Vec<Source>,
    /// Indices into `sources`, in collage order (also the selection order).
    selected: Vec<usize>,
    kind: i32, // 0 grid · 1 rows · 2 masonry
    cols: u32,
    gap: f32,    // working units (0..MAX_GAP)
    radius: f32, // working units (0..MAX_RADIUS)
    bg: [u8; 4],
    bg_mode: i32, // 0 transparent · 1 white · 2 black · 3 custom
    hue: f32,
    sat: f32,
    val: f32,
    /// First tap of a swap (a slot in `selected`); the next cell tap swaps.
    pending_swap: Option<usize>,
    scale: f32,
    result: Option<image::RgbaImage>,
}

pub struct Collager {
    window: Option<CollageWindow>,
    shared: Rc<RefCell<Shared>>,
}

impl Collager {
    pub fn new() -> Self {
        Self {
            window: None,
            shared: Rc::new(RefCell::new(Shared {
                sources: Vec::new(),
                selected: Vec::new(),
                kind: 1,
                cols: DEFAULT_COLS,
                gap: DEFAULT_GAP,
                radius: 0.0,
                bg: [255, 255, 255, 255],
                bg_mode: 1,
                hue: 210.0,
                sat: 0.55,
                val: 1.0,
                pending_swap: None,
                scale: 1.0,
                result: None,
            })),
        }
    }

    /// Open the editor with `sources` in the tray. `preselect` lists snip ids to
    /// start in the collage (e.g. the currently-floating ones); if none match,
    /// the first few sources are auto-selected so the user sees a collage at once.
    /// True while the editor window is on screen. Backed by live window
    /// visibility (the `window` field lingers `Some` after close), so the
    /// pump stays awake while the user is building a collage.
    pub fn is_open(&self) -> bool {
        self.window
            .as_ref()
            .map_or(false, |w| w.window().is_visible())
    }

    pub fn open(
        &mut self,
        sources_in: Vec<CollageSource>,
        preselect: Vec<Uuid>,
        scale: f32,
    ) -> Result<(), slint::PlatformError> {
        let mut sources: Vec<Source> = Vec::new();
        for s in sources_in {
            let (w, h) = s.thumb.dimensions();
            if w == 0 || h == 0 {
                continue;
            }
            let aspect = (w as f32 / h as f32).clamp(0.1, 12.0);
            let buf = SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(s.thumb.as_raw(), w, h);
            sources.push(Source {
                id: s.id,
                thumb: Image::from_rgba8(buf),
                aspect,
                path: s.path,
            });
        }
        if sources.is_empty() {
            return Ok(());
        }

        let mut selected: Vec<usize> = Vec::new();
        for id in &preselect {
            if let Some(i) = sources.iter().position(|s| s.id == *id) {
                if !selected.contains(&i) {
                    selected.push(i);
                }
            }
        }
        if selected.is_empty() {
            for i in 0..sources.len().min(4) {
                selected.push(i);
            }
        }

        if let Some(w) = self.window.take() {
            let _ = w.hide();
        }
        {
            let mut st = self.shared.borrow_mut();
            st.sources = sources;
            st.selected = selected;
            st.kind = 1;
            st.cols = DEFAULT_COLS;
            st.gap = DEFAULT_GAP;
            st.radius = 0.0;
            st.bg = [255, 255, 255, 255];
            st.bg_mode = 1;
            st.hue = 210.0;
            st.sat = 0.55;
            st.val = 1.0;
            st.pending_swap = None;
            st.scale = scale;
            st.result = None;
        }

        let w = CollageWindow::new()?;
        w.set_wheel_image(crate::color::make_color_wheel(WHEEL_TEX));
        // One checkerboard for the transparent-background backdrop; generated
        // once and stretched to fit (tile distortion is invisible).
        w.set_checker(make_checker(AVAIL_W as u32, AVAIL_H as u32));
        self.wire(&w);
        refresh(&self.shared, &w.as_weak());

        let win_w = AVAIL_W + 60.0;
        let win_h = 66.0 + AVAIL_H + 110.0 + 66.0 + 16.0;
        w.set_start_w(win_w);
        w.set_start_h(win_h);
        w.window().set_size(slint::LogicalSize::new(win_w, win_h));
        if let Some((px, py)) = crate::screen::centre_on_cursor_monitor(win_w, win_h) {
            w.window().set_position(slint::PhysicalPosition::new(px, py));
        }

        let shared_close = self.shared.clone();
        w.window().on_close_requested(move || {
            shared_close.borrow_mut().result = None;
            slint::CloseRequestResponse::HideWindow
        });

        w.show()?;
        self.window = Some(w);
        Ok(())
    }

    fn wire(&self, w: &CollageWindow) {
        let sh = &self.shared;

        let s = sh.clone();
        let weak = w.as_weak();
        w.on_pick_layout(move |k| {
            s.borrow_mut().kind = k.clamp(0, 2);
            refresh(&s, &weak);
        });

        let s = sh.clone();
        let weak = w.as_weak();
        w.on_cols_inc(move || {
            {
                let mut st = s.borrow_mut();
                st.cols = (st.cols + 1).min(MAX_COLS);
            }
            refresh(&s, &weak);
        });

        let s = sh.clone();
        let weak = w.as_weak();
        w.on_cols_dec(move || {
            {
                let mut st = s.borrow_mut();
                st.cols = st.cols.saturating_sub(1).max(MIN_COLS);
            }
            refresh(&s, &weak);
        });

        let s = sh.clone();
        let weak = w.as_weak();
        w.on_set_gap(move |v| {
            s.borrow_mut().gap = v.clamp(0.0, MAX_GAP);
            refresh(&s, &weak);
        });

        let s = sh.clone();
        let weak = w.as_weak();
        w.on_set_radius(move |v| {
            s.borrow_mut().radius = v.clamp(0.0, MAX_RADIUS);
            refresh(&s, &weak);
        });

        let s = sh.clone();
        let weak = w.as_weak();
        w.on_set_bg(move |mode| {
            {
                let mut st = s.borrow_mut();
                st.bg_mode = mode;
                st.bg = match mode {
                    0 => [0, 0, 0, 0],
                    2 => [0, 0, 0, 255],
                    _ => [255, 255, 255, 255],
                };
            }
            refresh(&s, &weak);
        });

        let s = sh.clone();
        let weak = w.as_weak();
        w.on_pick_wheel(move |x, y, size| {
            {
                let mut st = s.borrow_mut();
                let r = (size / 2.0).max(1.0);
                let (dx, dy) = (x - r, y - r);
                let dist = (dx * dx + dy * dy).sqrt();
                st.sat = (dist / r).min(1.0);
                st.hue = dy.atan2(dx).to_degrees().rem_euclid(360.0);
                let [rr, gg, bb] = crate::color::hsv_to_rgb(st.hue, st.sat, st.val);
                st.bg = [rr, gg, bb, 255];
                st.bg_mode = 3;
            }
            refresh(&s, &weak);
        });

        let s = sh.clone();
        let weak = w.as_weak();
        w.on_set_brightness(move |v| {
            {
                let mut st = s.borrow_mut();
                st.val = (v / 100.0).clamp(0.0, 1.0);
                if st.bg_mode == 3 {
                    let [rr, gg, bb] = crate::color::hsv_to_rgb(st.hue, st.sat, st.val);
                    st.bg = [rr, gg, bb, 255];
                }
            }
            refresh(&s, &weak);
        });

        let s = sh.clone();
        let weak = w.as_weak();
        w.on_tray_tap(move |id| {
            {
                let mut st = s.borrow_mut();
                if let Some(si) = st.sources.iter().position(|x| x.id.to_string() == id.as_str()) {
                    if let Some(pos) = st.selected.iter().position(|&i| i == si) {
                        st.selected.remove(pos);
                        st.pending_swap = None;
                    } else {
                        st.selected.push(si);
                    }
                }
            }
            refresh(&s, &weak);
        });

        let s = sh.clone();
        let weak = w.as_weak();
        w.on_cell_tap(move |index| {
            {
                let mut st = s.borrow_mut();
                let idx = index as usize;
                if idx < st.selected.len() {
                    match st.pending_swap {
                        None => st.pending_swap = Some(idx),
                        Some(a) if a == idx => st.pending_swap = None,
                        Some(a) => {
                            st.selected.swap(a, idx);
                            st.pending_swap = None;
                        }
                    }
                }
            }
            refresh(&s, &weak);
        });

        let s = sh.clone();
        let weak = w.as_weak();
        w.on_cancel(move || {
            {
                let mut st = s.borrow_mut();
                st.result = None;
                st.sources.clear();
            }
            if let Some(c) = weak.upgrade() {
                let _ = c.hide();
            }
        });

        let s = sh.clone();
        let weak = w.as_weak();
        w.on_save(move || {
            let baked = {
                let st = s.borrow();
                if st.selected.is_empty() {
                    None
                } else {
                    Some(render_final(&st))
                }
            };
            if baked.is_none() {
                return; // nothing selected — keep the editor open
            }
            {
                let mut st = s.borrow_mut();
                st.result = baked;
                st.sources.clear();
            }
            if let Some(c) = weak.upgrade() {
                let _ = c.hide();
            }
        });
    }

    /// Drained each app tick: the freshly composited collage, if the user just
    /// saved. Returns once, then clears.
    pub fn take_result(&mut self) -> Option<image::RgbaImage> {
        self.shared.borrow_mut().result.take()
    }
}

// ---- UI refresh ------------------------------------------------------------

fn brush(c: [u8; 4]) -> Brush {
    Brush::SolidColor(Color::from_argb_u8(c[3], c[0], c[1], c[2]))
}

/// Recompute the layout and push the full UI model (cells + tray + chrome).
fn refresh(sh: &Rc<RefCell<Shared>>, weak: &slint::Weak<CollageWindow>) {
    let Some(c) = weak.upgrade() else {
        return;
    };

    // Pull everything we need, then drop the borrow before touching the UI.
    let (cells_model, tray_model, tray_w, cw, ch, count, kind, cols, gap, radius, bg, bg_mode, cur, brightness, sel_x, sel_y) = {
        let st = sh.borrow();
        let aspects: Vec<f32> = st.selected.iter().map(|&i| st.sources[i].aspect).collect();
        let (work_cells, work_h) = layout_cells(&aspects, st.kind, st.cols, st.gap, WORK_W);
        let ph = work_h.max(1.0);
        let pscale = (AVAIL_W / WORK_W).min(AVAIL_H / ph).max(0.01);
        let cw = WORK_W * pscale;
        let ch = work_h * pscale;

        let cells: Vec<CollageCell> = work_cells
            .iter()
            .map(|cr| {
                let src = &st.sources[st.selected[cr.idx]];
                let max_r = (cr.w.min(cr.h) * pscale) / 2.0;
                CollageCell {
                    source: src.thumb.clone(),
                    x: cr.x * pscale,
                    y: cr.y * pscale,
                    w: cr.w * pscale,
                    h: cr.h * pscale,
                    radius: (radius_px(st.radius) * pscale).min(max_r),
                    selected: st.pending_swap == Some(cr.idx),
                    index: cr.idx as i32,
                }
            })
            .collect();

        let tray: Vec<TrayItem> = (0..st.sources.len())
            .map(|si| {
                let order = st
                    .selected
                    .iter()
                    .position(|&i| i == si)
                    .map(|p| p as i32 + 1)
                    .unwrap_or(0);
                TrayItem {
                    thumb: st.sources[si].thumb.clone(),
                    id: SharedString::from(st.sources[si].id.to_string()),
                    order,
                }
            })
            .collect();
        let tray_w = st.sources.len() as f32 * 92.0 + 26.0;

        let [r, g, b] = crate::color::hsv_to_rgb(st.hue, st.sat, st.val);
        let rr = WHEEL_DISP / 2.0;
        let ang = st.hue.to_radians();
        (
            cells,
            tray,
            tray_w,
            cw,
            ch,
            st.selected.len() as i32,
            st.kind,
            st.cols,
            st.gap,
            st.radius,
            st.bg,
            st.bg_mode,
            brush([r, g, b, 255]),
            st.val * 100.0,
            rr + st.sat * rr * ang.cos(),
            rr + st.sat * rr * ang.sin(),
        )
    };

    c.set_canvas_w(cw);
    c.set_canvas_h(ch);
    c.set_cells(ModelRc::new(VecModel::from(cells_model)));
    c.set_tray(ModelRc::new(VecModel::from(tray_model)));
    c.set_tray_width(tray_w);
    c.set_count(count);
    c.set_layout_kind(kind);
    c.set_cols_label(SharedString::from(format!("{cols}")));
    c.set_gap_value(gap);
    c.set_radius_value(radius);
    c.set_bg_color(brush(bg));
    c.set_bg_mode(bg_mode);
    c.set_current_color(cur);
    c.set_brightness(brightness);
    c.set_wheel_sel_x(sel_x);
    c.set_wheel_sel_y(sel_y);
}

/// Map the radius slider (working units) to working px. Identity for now, but
/// kept as a hook so the slider scale can change without touching the layout.
fn radius_px(r: f32) -> f32 {
    r
}

// ---- Layout (one source of truth for preview + final) ----------------------

#[derive(Clone, Copy)]
struct CellRect {
    /// Slot in the selection order (also the cell-tap index).
    idx: usize,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

/// Compute cell rectangles in working units for `aspects` (width/height of each
/// selected image, in selection order). Returns the cells + the canvas height.
fn layout_cells(aspects: &[f32], kind: i32, cols: u32, gap: f32, cw_total: f32) -> (Vec<CellRect>, f32) {
    if aspects.is_empty() {
        return (Vec::new(), cw_total * 0.6);
    }
    match kind {
        0 => grid_cells(aspects, cols, gap, cw_total),
        2 => masonry_cells(aspects, cols, gap, cw_total),
        _ => rows_cells(aspects, cols, gap, cw_total),
    }
}

/// Uniform grid of square cells (cover-fit crops each image). A partial last
/// row is stretched to fill the width so there is no ragged gap.
fn grid_cells(aspects: &[f32], cols: u32, gap: f32, cw_total: f32) -> (Vec<CellRect>, f32) {
    let n = aspects.len();
    let cols = (cols as usize).clamp(1, n);
    let rows = (n + cols - 1) / cols;
    let cell = (cw_total - gap * (cols as f32 + 1.0)) / cols as f32;
    let cell = cell.max(1.0);
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let r = i / cols;
        let col = i % cols;
        let row_start = r * cols;
        let row_count = (n - row_start).min(cols);
        let (w, x) = if row_count < cols {
            let wl = ((cw_total - gap * (row_count as f32 + 1.0)) / row_count as f32).max(1.0);
            (wl, gap + col as f32 * (wl + gap))
        } else {
            (cell, gap + col as f32 * (cell + gap))
        };
        let y = gap + r as f32 * (cell + gap);
        out.push(CellRect { idx: i, x, y, w, h: cell });
    }
    (out, gap + rows as f32 * (cell + gap))
}

/// Justified rows (Telegram-album style): images keep their aspect ratio and
/// each row is scaled to span the full width. `cols` is the approximate number
/// of images per row.
fn rows_cells(aspects: &[f32], cols: u32, gap: f32, cw_total: f32) -> (Vec<CellRect>, f32) {
    let n = aspects.len();
    let target = (cols as f32).max(1.0);
    // Break into rows by accumulated aspect (wide images count for more).
    let mut rows: Vec<Vec<usize>> = Vec::new();
    let mut cur: Vec<usize> = Vec::new();
    let mut cur_sum = 0.0;
    for i in 0..n {
        cur.push(i);
        cur_sum += aspects[i];
        if cur_sum >= target {
            rows.push(std::mem::take(&mut cur));
            cur_sum = 0.0;
        }
    }
    if !cur.is_empty() {
        rows.push(cur);
    }

    let mut out = Vec::with_capacity(n);
    let mut y = gap;
    for row in &rows {
        let k = row.len() as f32;
        let sum_a: f32 = row.iter().map(|&i| aspects[i]).sum::<f32>().max(0.01);
        let row_w = (cw_total - gap * (k + 1.0)).max(1.0);
        let h = (row_w / sum_a).max(1.0);
        let mut x = gap;
        for &i in row {
            let w = (h * aspects[i]).max(1.0);
            out.push(CellRect { idx: i, x, y, w, h });
            x += w + gap;
        }
        y += h + gap;
    }
    (out, y)
}

/// Masonry: `cols` equal-width columns; each image keeps its aspect and is
/// dropped into the currently-shortest column (Pinterest-style packing).
fn masonry_cells(aspects: &[f32], cols: u32, gap: f32, cw_total: f32) -> (Vec<CellRect>, f32) {
    let n = aspects.len();
    let cols = (cols as usize).clamp(1, n);
    let colw = ((cw_total - gap * (cols as f32 + 1.0)) / cols as f32).max(1.0);
    let mut col_h = vec![gap; cols];
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        // Shortest column.
        let mut c = 0;
        for j in 1..cols {
            if col_h[j] < col_h[c] {
                c = j;
            }
        }
        let h = (colw / aspects[i]).max(1.0);
        let x = gap + c as f32 * (colw + gap);
        let y = col_h[c];
        out.push(CellRect { idx: i, x, y, w: colw, h });
        col_h[c] += h + gap;
    }
    let canvas_h = col_h.into_iter().fold(0.0_f32, f32::max);
    (out, canvas_h)
}

// ---- Final composite -------------------------------------------------------

/// Composite the selected full-resolution images into one collage image.
fn render_final(st: &Shared) -> image::RgbaImage {
    let aspects: Vec<f32> = st.selected.iter().map(|&i| st.sources[i].aspect).collect();
    let (cells, work_h) = layout_cells(&aspects, st.kind, st.cols, st.gap, WORK_W);

    let ratio = (work_h / WORK_W).max(0.05);
    let (out_w, out_h) = if ratio <= 1.0 {
        (OUT_LONG, OUT_LONG * ratio)
    } else {
        (OUT_LONG / ratio, OUT_LONG)
    };
    let out_w = (out_w.round() as u32).clamp(16, 4096);
    let out_h = (out_h.round() as u32).clamp(16, 4096);
    let oscale = out_w as f32 / WORK_W;

    let mut canvas = if st.bg[3] > 0 {
        image::RgbaImage::from_pixel(out_w, out_h, image::Rgba(st.bg))
    } else {
        image::RgbaImage::new(out_w, out_h)
    };

    for cr in &cells {
        let src_idx = st.selected[cr.idx];
        let full = match image::open(&st.sources[src_idx].path) {
            Ok(img) => img.to_rgba8(),
            Err(_) => continue,
        };
        if full.width() == 0 || full.height() == 0 {
            continue;
        }
        let cx = cr.x * oscale;
        let cy = cr.y * oscale;
        let cw = cr.w * oscale;
        let ch = cr.h * oscale;
        let rad = (radius_px(st.radius) * oscale).min(cw.min(ch) / 2.0);
        composite_cell(&mut canvas, &full, cx, cy, cw, ch, rad);
    }
    canvas
}

/// Cover-fit `src` into the `cw`×`ch` cell at (`cx`,`cy`) with anti-aliased
/// rounded corners, alpha-blended over `canvas`.
fn composite_cell(
    canvas: &mut image::RgbaImage,
    src: &image::RgbaImage,
    cx: f32,
    cy: f32,
    cw: f32,
    ch: f32,
    radius: f32,
) {
    let cwi = cw.round().max(1.0) as u32;
    let chi = ch.round().max(1.0) as u32;
    let (sw, sh) = src.dimensions();
    if sw == 0 || sh == 0 {
        return;
    }
    // Cover: scale so the source fills the cell, then centre-crop.
    let scale = (cwi as f32 / sw as f32).max(chi as f32 / sh as f32);
    let rw = (sw as f32 * scale).round().max(cwi as f32) as u32;
    let rh = (sh as f32 * scale).round().max(chi as f32) as u32;
    let resized = image::imageops::resize(src, rw, rh, image::imageops::FilterType::Lanczos3);
    let ox = ((rw - cwi) / 2) as i64;
    let oy = ((rh - chi) / 2) as i64;

    let cxr = cx.round() as i64;
    let cyr = cy.round() as i64;
    let (canw, canh) = canvas.dimensions();

    for yy in 0..chi {
        for xx in 0..cwi {
            let cov = rounded_coverage(xx as f32 + 0.5, yy as f32 + 0.5, cwi as f32, chi as f32, radius);
            if cov <= 0.0 {
                continue;
            }
            let sx = (xx as i64 + ox).clamp(0, rw as i64 - 1) as u32;
            let sy = (yy as i64 + oy).clamp(0, rh as i64 - 1) as u32;
            let px = resized.get_pixel(sx, sy).0;
            let dx = cxr + xx as i64;
            let dy = cyr + yy as i64;
            if dx < 0 || dy < 0 || dx >= canw as i64 || dy >= canh as i64 {
                continue;
            }
            blend_over(canvas.get_pixel_mut(dx as u32, dy as u32), px, cov);
        }
    }
}

/// Pixel coverage (0..1) of a rounded rectangle at point (px, py) in a w×h cell,
/// with ~1px anti-aliasing on the corner arcs.
fn rounded_coverage(px: f32, py: f32, w: f32, h: f32, radius: f32) -> f32 {
    let r = radius.min(w / 2.0).min(h / 2.0);
    if r <= 0.5 {
        return 1.0;
    }
    // How far the point is inside a corner zone, per axis.
    let dx = (r - px).max(px - (w - r)).max(0.0);
    let dy = (r - py).max(py - (h - r)).max(0.0);
    if dx <= 0.0 || dy <= 0.0 {
        return 1.0; // straight edge region
    }
    let dist = (dx * dx + dy * dy).sqrt();
    (r + 0.5 - dist).clamp(0.0, 1.0)
}

/// Source-over alpha blend of straight-alpha `s` (with extra `cov` coverage)
/// onto a possibly-transparent `dst`.
fn blend_over(dst: &mut image::Rgba<u8>, s: [u8; 4], cov: f32) {
    let sa = (s[3] as f32 / 255.0) * cov.clamp(0.0, 1.0);
    if sa <= 0.0 {
        return;
    }
    let da = dst.0[3] as f32 / 255.0;
    let oa = sa + da * (1.0 - sa);
    if oa <= 0.0 {
        dst.0 = [0, 0, 0, 0];
        return;
    }
    for c in 0..3 {
        let sc = s[c] as f32;
        let dc = dst.0[c] as f32;
        dst.0[c] = ((sc * sa + dc * da * (1.0 - sa)) / oa).round().clamp(0.0, 255.0) as u8;
    }
    dst.0[3] = (oa * 255.0).round().clamp(0.0, 255.0) as u8;
}

// ---- Colour helpers (shared shape with annotate.rs) ------------------------

/// A subtle dark checkerboard, shown behind the collage when the background is
/// transparent so the user can tell "no background" from a solid colour.
fn make_checker(w: u32, h: u32) -> Image {
    let w = w.max(1);
    let h = h.max(1);
    let tile = 14u32;
    let mut buf = SharedPixelBuffer::<slint::Rgba8Pixel>::new(w, h);
    let data = buf.make_mut_slice();
    for y in 0..h {
        for x in 0..w {
            let on = ((x / tile) + (y / tile)) % 2 == 0;
            let v = if on { 58 } else { 42 };
            data[(y * w + x) as usize] = slint::Rgba8Pixel { r: v, g: v, b: v + 2, a: 255 };
        }
    }
    Image::from_rgba8(buf)
}

