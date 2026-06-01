//! Annotation editor manager.
//!
//! Owns the `AnnotateWindow` and ALL annotation geometry. The Slint side is a
//! dumb renderer: it reports pointer gestures and reflects the shape list +
//! a live "draft" shape that Rust maintains. Keeping geometry here means the
//! on-screen preview and the saved result come from the SAME code, so they
//! match exactly.
//!
//! Flow (mirrors `translate_popup.rs`):
//!   1. `open(id, image, scale)` shows the editor for a snip.
//!   2. Toolbar callbacks set tool/colour/width; canvas callbacks build shapes.
//!   3. On Save, `flatten` bakes the shapes onto the image at full resolution
//!      (tiny-skia for vectors, ab_glyph for text) and stashes the result.
//!   4. `take_result()` (drained each app tick) hands the new image to the app,
//!      which rewrites the snip exactly like a crop.

use slint::{Brush, Color, ComponentHandle, Image, ModelRc, SharedPixelBuffer, SharedString, VecModel};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::OnceLock;
use uuid::Uuid;

use crate::{AnnoShape, AnnotateWindow};

/// Colour-wheel display size in logical px — MUST match `annotate.slint`'s
/// `wheel` Rectangle so the selector dot lands exactly under the pointer.
const WHEEL_DISP: f32 = 168.0;
/// Resolution the wheel texture is generated at (downscaled to WHEEL_DISP).
const WHEEL_TEX: u32 = 256;

/// Per-tool default stroke width in DISPLAY px (index = tool code). Kept on
/// screen regardless of zoom; stored as image px (÷ display scale) per shape.
const DEFAULT_WIDTHS: [f32; 7] = [4.0, 22.0, 4.0, 4.0, 4.0, 4.0, 4.0];
/// Per-tool default opacity, 0..1 (index = tool code). Highlighter starts low.
const DEFAULT_OPACITIES: [f32; 7] = [1.0, 0.4, 1.0, 1.0, 1.0, 1.0, 1.0];
/// Stroke-width slider bounds (DISPLAY px).
const MIN_WIDTH: f32 = 1.0;
const MAX_WIDTH: f32 = 64.0;
/// Text height in DISPLAY px (image px = this ÷ scale).
const TEXT_DISPLAY_SIZE: f32 = 26.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Pen,
    Highlighter,
    Rect,
    Ellipse,
    Line,
    Arrow,
    Text,
}

impl Kind {
    fn from_tool(t: i32) -> Kind {
        match t {
            1 => Kind::Highlighter,
            2 => Kind::Rect,
            3 => Kind::Ellipse,
            4 => Kind::Line,
            5 => Kind::Arrow,
            6 => Kind::Text,
            _ => Kind::Pen,
        }
    }
    fn freehand(self) -> bool {
        matches!(self, Kind::Pen | Kind::Highlighter)
    }
}

/// One annotation primitive in IMAGE-pixel coordinates.
#[derive(Clone)]
struct Shape {
    kind: Kind,
    pts: Vec<(f32, f32)>,
    color: [u8; 4],
    stroke: f32, // image px
    text: String,
    text_size: f32, // image px
}

struct Shared {
    id: Option<Uuid>,
    base: Option<image::RgbaImage>,
    scale: f32, // EFFECTIVE display px per image px (= base_disp × zoom)
    base_disp: f32, // fit scale chosen at open (zoom 1.0)
    zoom: f32,
    tool: i32,
    /// Active colour as HSV: hue 0..360, saturation 0..1, value 0..1.
    hue: f32,
    sat: f32,
    val: f32,
    /// Per-tool stroke width (DISPLAY px) and opacity (0..1), so each tool keeps
    /// its own feel as the user switches between them.
    widths: [f32; 7],
    opacities: [f32; 7],
    shapes: Vec<Shape>,
    draft: Option<Shape>,
    undo: Vec<Vec<Shape>>,
    redo: Vec<Vec<Shape>>,
    text_anchor: (f32, f32),
    /// Frame counter for the blinking size-preview pulse (advances each tick).
    pulse_frame: u32,
    result: Option<(Uuid, image::RgbaImage)>,
}

pub struct Annotator {
    window: Option<AnnotateWindow>,
    shared: Rc<RefCell<Shared>>,
}

impl Annotator {
    pub fn new() -> Self {
        Self {
            window: None,
            shared: Rc::new(RefCell::new(Shared {
                id: None,
                base: None,
                scale: 1.0,
                base_disp: 1.0,
                zoom: 1.0,
                tool: 0,
                hue: 0.0,
                sat: 1.0,
                val: 1.0,
                widths: DEFAULT_WIDTHS,
                opacities: DEFAULT_OPACITIES,
                shapes: Vec::new(),
                draft: None,
                undo: Vec::new(),
                redo: Vec::new(),
                text_anchor: (0.0, 0.0),
                pulse_frame: 0,
                result: None,
            })),
        }
    }

    /// Open the editor for snip `id`. `scale` is the UI scale factor (used only
    /// to place the window in physical pixels).
    pub fn open(
        &mut self,
        id: Uuid,
        image: image::RgbaImage,
        scale: f32,
    ) -> Result<(), slint::PlatformError> {
        let (iw, ih) = (image.width(), image.height());
        if iw == 0 || ih == 0 {
            return Ok(());
        }
        let (iwf, ihf) = (iw as f32, ih as f32);
        // Fit the snip into the working canvas WITHOUT upscaling: large snips
        // shrink to fit, normal/small ones open at 1:1 (natural size). The user
        // zooms in with the +/Fit control instead of it opening magnified.
        let disp = (1200.0 / iwf).min(580.0 / ihf).min(1.0).max(0.02);
        let cw = iwf * disp;
        let ch = ihf * disp;

        // Drop any previous editor so we never strand a window.
        if let Some(w) = self.window.take() {
            let _ = w.hide();
        }
        {
            let mut s = self.shared.borrow_mut();
            s.id = Some(id);
            s.scale = disp;
            s.base_disp = disp;
            s.zoom = 1.0;
            s.tool = 0;
            s.hue = 0.0;
            s.sat = 1.0;
            s.val = 1.0;
            s.widths = DEFAULT_WIDTHS;
            s.opacities = DEFAULT_OPACITIES;
            s.pulse_frame = 0;
            s.shapes.clear();
            s.draft = None;
            s.undo.clear();
            s.redo.clear();
            s.text_anchor = (0.0, 0.0);
            s.result = None;
            s.base = None; // set after we build the Slint texture below
        }

        let w = AnnotateWindow::new()?;
        let buf = SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(image.as_raw(), iw, ih);
        w.set_snip_image(Image::from_rgba8(buf));
        self.shared.borrow_mut().base = Some(image);

        w.set_canvas_w(cw);
        w.set_canvas_h(ch);
        w.set_disp_scale(disp);
        w.set_tool(0);
        w.set_editing_text(false);
        w.set_text_disp_size(TEXT_DISPLAY_SIZE);
        w.set_shapes(ModelRc::new(VecModel::from(Vec::<AnnoShape>::new())));
        w.set_draft_commands(SharedString::new());
        w.set_drafting(false);
        w.set_zoom_label(SharedString::from(format!("{}%", (disp * 100.0).round() as i32)));
        w.set_wheel_image(make_color_wheel(WHEEL_TEX));
        apply_style(&w, &self.shared.borrow());

        self.wire(&w);

        // Size + centre the window. Chrome = two 66px pill rows (top + bottom);
        // the canvas sits between them. Min width keeps the bottom pill (cancel +
        // two sliders + save) from clipping for small snips.
        let chrome_h = 66.0 * 2.0;
        let win_w = (cw + 48.0).max(760.0);
        let win_h = ch + chrome_h + 24.0;
        w.window()
            .set_size(slint::LogicalSize::new(win_w, win_h));
        if let Some((px, py)) =
            centre_window((win_w * scale.max(0.1)) as i32, (win_h * scale.max(0.1)) as i32)
        {
            w.window().set_position(slint::PhysicalPosition::new(px, py));
        }

        // Closing the editor via its window frame discards (same as Cancel).
        let shared_close = self.shared.clone();
        w.window().on_close_requested(move || {
            shared_close.borrow_mut().id = None;
            slint::CloseRequestResponse::HideWindow
        });

        w.show()?;
        self.window = Some(w);
        Ok(())
    }

    /// Wire every editor callback to the shared geometry state.
    fn wire(&self, w: &AnnotateWindow) {
        let sh = &self.shared;

        // --- Tool / colour / width ---
        let s = sh.clone();
        let weak = w.as_weak();
        w.on_set_tool(move |n| {
            s.borrow_mut().tool = n;
            if let Some(c) = weak.upgrade() {
                c.set_tool(n);
                c.set_editing_text(false); // switching tools abandons unsent text
                apply_style(&c, &s.borrow()); // load this tool's width/opacity
            }
        });

        // Colour wheel: pointer (x, y) within a `size`-px wheel → hue + sat.
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
            }
            if let Some(c) = weak.upgrade() {
                apply_style(&c, &s.borrow());
            }
        });

        // Brightness (HSV value) slider, 0..100.
        let s = sh.clone();
        let weak = w.as_weak();
        w.on_set_brightness(move |v| {
            s.borrow_mut().val = (v / 100.0).clamp(0.0, 1.0);
            if let Some(c) = weak.upgrade() {
                apply_style(&c, &s.borrow());
            }
        });

        let s = sh.clone();
        let weak = w.as_weak();
        w.on_zoom_in(move || zoom_by(&s, &weak, 1.25));

        let s = sh.clone();
        let weak = w.as_weak();
        w.on_zoom_out(move || zoom_by(&s, &weak, 0.8));

        let s = sh.clone();
        let weak = w.as_weak();
        w.on_zoom_reset(move || zoom_reset(&s, &weak));

        // Stroke width (display px) for the current tool.
        let s = sh.clone();
        let weak = w.as_weak();
        w.on_set_width_px(move |v| {
            {
                let mut st = s.borrow_mut();
                let t = st.tool.clamp(0, 6) as usize;
                st.widths[t] = v.clamp(MIN_WIDTH, MAX_WIDTH);
            }
            if let Some(c) = weak.upgrade() {
                apply_style(&c, &s.borrow());
            }
        });

        // Opacity (slider gives 10..100) for the current tool.
        let s = sh.clone();
        let weak = w.as_weak();
        w.on_set_opacity(move |v| {
            {
                let mut st = s.borrow_mut();
                let t = st.tool.clamp(0, 6) as usize;
                st.opacities[t] = (v / 100.0).clamp(0.05, 1.0);
            }
            if let Some(c) = weak.upgrade() {
                apply_style(&c, &s.borrow());
            }
        });

        // --- Drawing ---
        let s = sh.clone();
        let weak = w.as_weak();
        w.on_draw_begin(move |x, y| {
            let mut st = s.borrow_mut();
            if st.tool == 6 {
                return;
            }
            let kind = Kind::from_tool(st.tool);
            let color = current_rgba(&st);
            let stroke = draft_stroke_img(&st);
            let mut pts = vec![(x, y)];
            if !kind.freehand() {
                pts.push((x, y));
            }
            st.draft = Some(Shape {
                kind,
                pts,
                color,
                stroke,
                text: String::new(),
                text_size: 0.0,
            });
            if let Some(c) = weak.upgrade() {
                push_draft(&c, &st);
            }
        });

        let s = sh.clone();
        let weak = w.as_weak();
        w.on_draw_move(move |x, y| {
            let mut st = s.borrow_mut();
            let mut moved = false;
            if let Some(d) = st.draft.as_mut() {
                if d.kind.freehand() {
                    let far = d
                        .pts
                        .last()
                        .map(|&(lx, ly)| (x - lx).hypot(y - ly) >= 1.0)
                        .unwrap_or(true);
                    if far {
                        d.pts.push((x, y));
                        moved = true;
                    }
                } else if let Some(last) = d.pts.last_mut() {
                    *last = (x, y);
                    moved = true;
                }
            }
            if moved {
                if let Some(c) = weak.upgrade() {
                    push_draft(&c, &st);
                }
            }
        });

        let s = sh.clone();
        let weak = w.as_weak();
        w.on_draw_end(move |x, y| {
            {
                let mut st = s.borrow_mut();
                if let Some(mut d) = st.draft.take() {
                    if !d.kind.freehand() {
                        if let Some(last) = d.pts.last_mut() {
                            *last = (x, y);
                        }
                    }
                    if shape_valid(&d) {
                        let snapshot = st.shapes.clone();
                        st.undo.push(snapshot);
                        st.redo.clear();
                        st.shapes.push(d);
                    }
                }
            }
            rebuild(&weak, &s);
        });

        // --- Text ---
        let s = sh.clone();
        let weak = w.as_weak();
        w.on_place_text(move |x, y| {
            let scale = {
                let mut st = s.borrow_mut();
                st.text_anchor = (x, y);
                st.scale
            };
            if let Some(c) = weak.upgrade() {
                c.set_text_x(x * scale);
                c.set_text_y(y * scale);
                c.set_text_color(brush(current_rgba(&s.borrow())));
                c.set_editing_text(true);
            }
        });

        let s = sh.clone();
        let weak = w.as_weak();
        w.on_commit_text(move |txt| {
            {
                let mut st = s.borrow_mut();
                let (ax, ay) = st.text_anchor;
                let color = current_rgba(&st);
                let size_img = (TEXT_DISPLAY_SIZE / st.base_disp).max(4.0);
                let text = txt.to_string();
                if !text.trim().is_empty() {
                    let snapshot = st.shapes.clone();
                    st.undo.push(snapshot);
                    st.redo.clear();
                    st.shapes.push(Shape {
                        kind: Kind::Text,
                        pts: vec![(ax, ay)],
                        color,
                        stroke: 0.0,
                        text,
                        text_size: size_img,
                    });
                }
            }
            if let Some(c) = weak.upgrade() {
                c.set_editing_text(false);
            }
            rebuild(&weak, &s);
        });

        // --- Undo / redo / clear ---
        let s = sh.clone();
        let weak = w.as_weak();
        w.on_undo(move || {
            {
                let mut st = s.borrow_mut();
                if let Some(prev) = st.undo.pop() {
                    let cur = std::mem::replace(&mut st.shapes, prev);
                    st.redo.push(cur);
                }
            }
            rebuild(&weak, &s);
        });

        let s = sh.clone();
        let weak = w.as_weak();
        w.on_redo(move || {
            {
                let mut st = s.borrow_mut();
                if let Some(next) = st.redo.pop() {
                    let cur = std::mem::replace(&mut st.shapes, next);
                    st.undo.push(cur);
                }
            }
            rebuild(&weak, &s);
        });

        let s = sh.clone();
        let weak = w.as_weak();
        w.on_clear_all(move || {
            {
                let mut st = s.borrow_mut();
                if !st.shapes.is_empty() {
                    let cur = std::mem::take(&mut st.shapes);
                    st.undo.push(cur);
                    st.redo.clear();
                }
            }
            rebuild(&weak, &s);
        });

        // --- Cancel / save ---
        let s = sh.clone();
        let weak = w.as_weak();
        w.on_cancel(move || {
            {
                let mut st = s.borrow_mut();
                st.id = None;
                st.base = None;
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
                match (st.id, st.base.as_ref()) {
                    (Some(id), Some(base)) => Some((id, flatten(base, &st.shapes))),
                    _ => None,
                }
            };
            {
                let mut st = s.borrow_mut();
                st.result = baked;
                st.base = None;
                st.id = None;
            }
            if let Some(c) = weak.upgrade() {
                let _ = c.hide();
            }
        });
    }

    /// Drained each app tick: the freshly flattened image to write back, if the
    /// user just saved. Returns once, then clears.
    pub fn take_result(&mut self) -> Option<(Uuid, image::RgbaImage)> {
        self.shared.borrow_mut().result.take()
    }

    /// Drive the blinking size-preview indicator. Called each app tick; while a
    /// slider is being dragged it pulses the preview's opacity so the size is
    /// easy to gauge against the image. A no-op otherwise.
    pub fn pulse_tick(&self) {
        let Some(c) = self.window.as_ref() else {
            return;
        };
        if c.get_size_previewing() {
            let f = {
                let mut s = self.shared.borrow_mut();
                s.pulse_frame = s.pulse_frame.wrapping_add(1);
                s.pulse_frame
            };
            // ~4 Hz blink between 0.25 and 1.0.
            let v = 0.25 + 0.75 * (0.5 + 0.5 * (f as f32 * 0.22).sin());
            c.set_preview_opacity(v);
        } else if self.shared.borrow().pulse_frame != 0 {
            self.shared.borrow_mut().pulse_frame = 0;
            c.set_preview_opacity(1.0);
        }
    }
}

// ---- UI model construction -------------------------------------------------

fn brush(c: [u8; 4]) -> Brush {
    Brush::SolidColor(Color::from_argb_u8(c[3], c[0], c[1], c[2]))
}

fn transparent() -> Brush {
    Brush::SolidColor(Color::from_argb_u8(0, 0, 0, 0))
}

/// The active draw colour (HSV → RGB) with the tool's opacity folded into alpha.
fn current_rgba(s: &Shared) -> [u8; 4] {
    let [r, g, b] = hsv_to_rgb(s.hue, s.sat, s.val);
    let op = s.opacities[s.tool.clamp(0, 6) as usize];
    [r, g, b, (255.0 * op).round().clamp(0.0, 255.0) as u8]
}

/// The current tool's stroke width in IMAGE px. Uses the BASE (fit) scale, not
/// the zoomed scale, so the stored stroke is the same no matter the zoom level —
/// zooming just magnifies the on-screen rendering.
fn draft_stroke_img(s: &Shared) -> f32 {
    let display = s.widths[s.tool.clamp(0, 6) as usize];
    (display / s.base_disp).max(0.3)
}

/// Reflect the current tool's width/opacity/colour into the editor chrome: the
/// sliders, the colour chip, the wheel's selector position + brightness slider,
/// and the text caret colour.
fn apply_style(c: &AnnotateWindow, s: &Shared) {
    let t = s.tool.clamp(0, 6) as usize;
    c.set_stroke_display(s.widths[t]);
    c.set_opacity_pct(s.opacities[t] * 100.0);
    c.set_fill_opacity(s.opacities[t]);
    let [r, g, b] = hsv_to_rgb(s.hue, s.sat, s.val);
    c.set_current_color(brush([r, g, b, 255]));
    c.set_text_color(brush(current_rgba(s)));
    c.set_brightness(s.val * 100.0);
    // Selector dot: centre + sat·R in the hue direction (matches the texture).
    let rr = WHEEL_DISP / 2.0;
    let ang = s.hue.to_radians();
    c.set_wheel_sel_x(rr + s.sat * rr * ang.cos());
    c.set_wheel_sel_y(rr + s.sat * rr * ang.sin());
}

/// HSV (h 0..360, s/v 0..1) → RGB.
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [u8; 3] {
    let h = h.rem_euclid(360.0);
    let c = v * s;
    let x = c * (1.0 - (((h / 60.0) % 2.0) - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h as u32) / 60 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    [
        ((r + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((g + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((b + m) * 255.0).round().clamp(0.0, 255.0) as u8,
    ]
}

/// Generate the HSV colour-wheel texture (hue around, saturation outward, full
/// value). Transparent outside the disc, with a 1.5px anti-aliased edge.
fn make_color_wheel(tex: u32) -> slint::Image {
    let mut buf = SharedPixelBuffer::<slint::Rgba8Pixel>::new(tex, tex);
    let r = tex as f32 / 2.0;
    let data = buf.make_mut_slice();
    for y in 0..tex {
        for x in 0..tex {
            let dx = x as f32 + 0.5 - r;
            let dy = y as f32 + 0.5 - r;
            let dist = (dx * dx + dy * dy).sqrt();
            let idx = (y * tex + x) as usize;
            data[idx] = if dist <= r {
                let hue = dy.atan2(dx).to_degrees().rem_euclid(360.0);
                let sat = (dist / r).min(1.0);
                let [rr, gg, bb] = hsv_to_rgb(hue, sat, 1.0);
                let a = if dist > r - 1.5 {
                    ((r - dist + 0.5).clamp(0.0, 1.0) * 255.0) as u8
                } else {
                    255
                };
                slint::Rgba8Pixel { r: rr, g: gg, b: bb, a }
            } else {
                slint::Rgba8Pixel { r: 0, g: 0, b: 0, a: 0 }
            };
        }
    }
    slint::Image::from_rgba8(buf)
}

/// Push the in-progress draft shape to the window for live preview.
fn push_draft(c: &AnnotateWindow, s: &Shared) {
    if let Some(d) = &s.draft {
        c.set_drafting(true);
        c.set_draft_stroke(brush(d.color));
        c.set_draft_width(d.stroke * s.scale);
        c.set_draft_fill(transparent());
        c.set_draft_commands(SharedString::from(geom_commands(&shape_geom(d), s.scale)));
    }
}

/// Rebuild the committed-shape model + undo/redo state, and clear the draft.
fn rebuild(weak: &slint::Weak<AnnotateWindow>, sh: &Rc<RefCell<Shared>>) {
    let Some(c) = weak.upgrade() else {
        return;
    };
    let s = sh.borrow();
    let scale = s.scale;
    let items: Vec<AnnoShape> = s.shapes.iter().map(|sp| build_anno(sp, scale)).collect();
    c.set_shapes(ModelRc::new(VecModel::from(items)));
    c.set_drafting(false);
    c.set_draft_commands(SharedString::new());
    c.set_can_undo(!s.undo.is_empty());
    c.set_can_redo(!s.redo.is_empty());
}

/// Recompute canvas size + scale + zoom label from the current zoom, then
/// rebuild shape paths. The label shows the EFFECTIVE size vs native pixels
/// (so a shrunk-to-fit large snip reads e.g. "48%", a 1:1 snip "100%").
fn refresh_zoom(sh: &Rc<RefCell<Shared>>, weak: &slint::Weak<AnnotateWindow>) {
    let (cw, ch, scale, pct) = {
        let s = sh.borrow();
        let (iw, ih) = s
            .base
            .as_ref()
            .map(|b| (b.width() as f32, b.height() as f32))
            .unwrap_or((1.0, 1.0));
        (iw * s.scale, ih * s.scale, s.scale, s.base_disp * s.zoom * 100.0)
    };
    if let Some(c) = weak.upgrade() {
        c.set_canvas_w(cw);
        c.set_canvas_h(ch);
        c.set_disp_scale(scale);
        c.set_zoom_label(SharedString::from(format!("{}%", pct.round() as i32)));
    }
    rebuild(weak, sh);
}

/// Multiply the zoom by `factor` (clamped 0.2–6×). Image-space geometry is
/// unchanged — only the on-screen magnification.
fn zoom_by(sh: &Rc<RefCell<Shared>>, weak: &slint::Weak<AnnotateWindow>, factor: f32) {
    {
        let mut s = sh.borrow_mut();
        s.zoom = (s.zoom * factor).clamp(0.2, 6.0);
        s.scale = s.base_disp * s.zoom;
    }
    refresh_zoom(sh, weak);
}

/// Reset to the fit zoom (1× of the base fit scale).
fn zoom_reset(sh: &Rc<RefCell<Shared>>, weak: &slint::Weak<AnnotateWindow>) {
    {
        let mut s = sh.borrow_mut();
        s.zoom = 1.0;
        s.scale = s.base_disp;
    }
    refresh_zoom(sh, weak);
}

fn build_anno(shape: &Shape, scale: f32) -> AnnoShape {
    if shape.kind == Kind::Text {
        let (x, y) = shape.pts.first().copied().unwrap_or((0.0, 0.0));
        AnnoShape {
            is_text: true,
            commands: SharedString::new(),
            stroke: brush(shape.color),
            width: 0.0,
            fill: transparent(),
            text: SharedString::from(shape.text.as_str()),
            text_x: x * scale,
            text_y: y * scale,
            text_size: shape.text_size * scale,
        }
    } else {
        AnnoShape {
            is_text: false,
            commands: SharedString::from(geom_commands(&shape_geom(shape), scale)),
            stroke: brush(shape.color),
            width: shape.stroke * scale,
            fill: transparent(),
            text: SharedString::new(),
            text_x: 0.0,
            text_y: 0.0,
            text_size: 0.0,
        }
    }
}

// ---- Geometry (one source of truth for preview + flatten) ------------------

/// A shape reduced to drawable geometry in IMAGE coordinates.
enum Geom {
    /// One or more polylines, each `(points, closed)`.
    Polylines(Vec<(Vec<(f32, f32)>, bool)>),
    /// An ellipse given by its bounding box (l, t, r, b).
    Oval(f32, f32, f32, f32),
}

fn bbox(shape: &Shape) -> (f32, f32, f32, f32) {
    let (x0, y0) = shape.pts[0];
    let (x1, y1) = shape.pts[1];
    (x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1))
}

fn arrow_parts(p0: (f32, f32), p1: (f32, f32), head: f32) -> (Vec<(f32, f32)>, Vec<(f32, f32)>) {
    let (x0, y0) = p0;
    let (x1, y1) = p1;
    let (dx, dy) = (x1 - x0, y1 - y0);
    let len = (dx * dx + dy * dy).sqrt().max(0.001);
    let (ux, uy) = (dx / len, dy / len);
    let head = head.min(len);
    let theta = 0.5_f32;
    let (c, s) = (theta.cos(), theta.sin());
    let (rx, ry) = (-ux, -uy); // reversed direction
    let h1 = (x1 + head * (rx * c - ry * s), y1 + head * (rx * s + ry * c));
    let h2 = (x1 + head * (rx * c + ry * s), y1 + head * (-rx * s + ry * c));
    (vec![p0, p1], vec![h1, p1, h2])
}

fn shape_geom(shape: &Shape) -> Geom {
    match shape.kind {
        Kind::Pen | Kind::Highlighter => Geom::Polylines(vec![(shape.pts.clone(), false)]),
        Kind::Line => Geom::Polylines(vec![(vec![shape.pts[0], shape.pts[1]], false)]),
        Kind::Rect => {
            let (l, t, r, b) = bbox(shape);
            Geom::Polylines(vec![(vec![(l, t), (r, t), (r, b), (l, b)], true)])
        }
        Kind::Ellipse => {
            let (l, t, r, b) = bbox(shape);
            Geom::Oval(l, t, r, b)
        }
        Kind::Arrow => {
            let head = shape.stroke * 3.5 + 6.0;
            let (shaft, wings) = arrow_parts(shape.pts[0], shape.pts[1], head);
            Geom::Polylines(vec![(shaft, false), (wings, false)])
        }
        Kind::Text => Geom::Polylines(vec![]),
    }
}

/// SVG `commands` string for the preview Path, in DISPLAY coordinates.
fn geom_commands(g: &Geom, scale: f32) -> String {
    let mut out = String::new();
    match g {
        Geom::Polylines(pls) => {
            for (pts, closed) in pls {
                for (i, &(x, y)) in pts.iter().enumerate() {
                    let lead = if i == 0 { "M" } else { "L" };
                    out.push_str(&format!("{} {:.2} {:.2} ", lead, x * scale, y * scale));
                }
                if *closed {
                    out.push_str("Z ");
                }
            }
        }
        Geom::Oval(l, t, r, b) => {
            let (l, t, r, b) = (l * scale, t * scale, r * scale, b * scale);
            let cy = (t + b) / 2.0;
            let rx = (r - l).abs() / 2.0;
            let ry = (b - t).abs() / 2.0;
            out.push_str(&format!("M {:.2} {:.2} ", l, cy));
            out.push_str(&format!("A {:.2} {:.2} 0 1 0 {:.2} {:.2} ", rx, ry, r, cy));
            out.push_str(&format!("A {:.2} {:.2} 0 1 0 {:.2} {:.2} Z ", rx, ry, l, cy));
        }
    }
    out
}

/// Whether a just-finished shape is worth keeping (rejects stray clicks).
fn shape_valid(shape: &Shape) -> bool {
    match shape.kind {
        Kind::Pen | Kind::Highlighter => shape.pts.len() >= 2,
        Kind::Text => !shape.text.trim().is_empty(),
        _ => {
            let (x0, y0) = shape.pts[0];
            let (x1, y1) = shape.pts[1];
            (x1 - x0).hypot(y1 - y0) >= 2.0
        }
    }
}

// ---- Flatten ---------------------------------------------------------------

fn premul(r: u8, g: u8, b: u8, a: u8) -> (u8, u8, u8) {
    let a = a as u16;
    (
        (r as u16 * a / 255) as u8,
        (g as u16 * a / 255) as u8,
        (b as u16 * a / 255) as u8,
    )
}

fn unpremul(r: u8, g: u8, b: u8, a: u8) -> (u8, u8, u8) {
    if a == 0 {
        return (0, 0, 0);
    }
    let a = a as u16;
    (
        ((r as u16 * 255 / a).min(255)) as u8,
        ((g as u16 * 255 / a).min(255)) as u8,
        ((b as u16 * 255 / a).min(255)) as u8,
    )
}

/// Bake all shapes onto a copy of `base` at full resolution.
fn flatten(base: &image::RgbaImage, shapes: &[Shape]) -> image::RgbaImage {
    let (w, h) = (base.width(), base.height());
    let mut pm = match tiny_skia::Pixmap::new(w, h) {
        Some(p) => p,
        None => return base.clone(),
    };
    {
        let dst = pm.data_mut();
        for (i, px) in base.pixels().enumerate() {
            let [r, g, b, a] = px.0;
            let (pr, pg, pb) = premul(r, g, b, a);
            let o = i * 4;
            dst[o] = pr;
            dst[o + 1] = pg;
            dst[o + 2] = pb;
            dst[o + 3] = a;
        }
    }

    for shape in shapes {
        if shape.kind == Kind::Text {
            continue;
        }
        let mut paint = tiny_skia::Paint::default();
        paint.anti_alias = true;
        let [r, g, b, a] = shape.color;
        paint.set_color_rgba8(r, g, b, a);
        // Highlighter is just a fat translucent stroke (same as the live
        // preview) — plain source-over so preview and output match exactly.
        let stroke = tiny_skia::Stroke {
            width: shape.stroke.max(0.5),
            line_cap: tiny_skia::LineCap::Round,
            line_join: tiny_skia::LineJoin::Round,
            ..Default::default()
        };
        match shape_geom(shape) {
            Geom::Polylines(pls) => {
                for (pts, closed) in pls {
                    if pts.len() < 2 {
                        continue;
                    }
                    let mut pb = tiny_skia::PathBuilder::new();
                    pb.move_to(pts[0].0, pts[0].1);
                    for &(x, y) in &pts[1..] {
                        pb.line_to(x, y);
                    }
                    if closed {
                        pb.close();
                    }
                    if let Some(path) = pb.finish() {
                        pm.stroke_path(
                            &path,
                            &paint,
                            &stroke,
                            tiny_skia::Transform::identity(),
                            None,
                        );
                    }
                }
            }
            Geom::Oval(l, t, r, b) => {
                if let Some(rect) = tiny_skia::Rect::from_ltrb(l, t, r, b) {
                    let mut pb = tiny_skia::PathBuilder::new();
                    pb.push_oval(rect);
                    if let Some(path) = pb.finish() {
                        pm.stroke_path(
                            &path,
                            &paint,
                            &stroke,
                            tiny_skia::Transform::identity(),
                            None,
                        );
                    }
                }
            }
        }
    }

    let mut out = image::RgbaImage::new(w, h);
    {
        let src = pm.data();
        for (i, px) in out.pixels_mut().enumerate() {
            let o = i * 4;
            let (r, g, b) = unpremul(src[o], src[o + 1], src[o + 2], src[o + 3]);
            px.0 = [r, g, b, src[o + 3]];
        }
    }

    // Text is baked last, straight-alpha, onto the un-premultiplied result.
    for shape in shapes {
        if shape.kind == Kind::Text {
            draw_text(&mut out, shape);
        }
    }
    out
}

fn blend_px(dst: &mut image::Rgba<u8>, color: [u8; 4], coverage: f32) {
    let a = (coverage * color[3] as f32 / 255.0).clamp(0.0, 1.0);
    if a <= 0.0 {
        return;
    }
    let inv = 1.0 - a;
    for c in 0..3 {
        dst.0[c] = (color[c] as f32 * a + dst.0[c] as f32 * inv).round() as u8;
    }
    dst.0[3] = 255;
}

fn draw_text(img: &mut image::RgbaImage, shape: &Shape) {
    use ab_glyph::{Font, ScaleFont};
    let Some(font) = load_font() else {
        return;
    };
    let (ax, ay) = shape.pts.first().copied().unwrap_or((0.0, 0.0));
    let scale = ab_glyph::PxScale::from(shape.text_size.max(6.0));
    let scaled = font.as_scaled(scale);
    let baseline = ay + scaled.ascent();
    let (iw, ih) = (img.width() as i32, img.height() as i32);
    let mut caret = ax;
    let mut prev: Option<ab_glyph::GlyphId> = None;
    for ch in shape.text.chars() {
        if ch == '\n' || ch == '\r' {
            continue;
        }
        let gid = font.glyph_id(ch);
        if let Some(p) = prev {
            caret += scaled.kern(p, gid);
        }
        let glyph = gid.with_scale_and_position(scale, ab_glyph::point(caret, baseline));
        if let Some(outlined) = font.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            outlined.draw(|gx, gy, cov| {
                let x = bounds.min.x as i32 + gx as i32;
                let y = bounds.min.y as i32 + gy as i32;
                if x >= 0 && y >= 0 && x < iw && y < ih {
                    blend_px(img.get_pixel_mut(x as u32, y as u32), shape.color, cov);
                }
            });
        }
        caret += scaled.h_advance(gid);
        prev = Some(gid);
    }
}

/// Lazily load a system UI font for baking text (Windows). Cached for reuse.
fn load_font() -> Option<&'static ab_glyph::FontVec> {
    static FONT: OnceLock<Option<ab_glyph::FontVec>> = OnceLock::new();
    FONT.get_or_init(|| {
        let candidates = [
            r"C:\Windows\Fonts\segoeui.ttf",
            r"C:\Windows\Fonts\arial.ttf",
            r"C:\Windows\Fonts\tahoma.ttf",
            r"C:\Windows\Fonts\verdana.ttf",
        ];
        for path in candidates {
            if let Ok(bytes) = std::fs::read(path) {
                if let Ok(font) = ab_glyph::FontVec::try_from_vec(bytes) {
                    return Some(font);
                }
            }
        }
        log::warn!("annotate: no system font found for text baking");
        None
    })
    .as_ref()
}

// ---- Window placement ------------------------------------------------------

/// Top-left (physical px) to centre a `wp`×`hp` window on the monitor under the
/// cursor. `None` off-Windows (let the OS place it).
#[cfg(target_os = "windows")]
fn centre_window(wp: i32, hp: i32) -> Option<(i32, i32)> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
    unsafe {
        let mut p = POINT::default();
        let _ = GetCursorPos(&mut p);
        let hmon = MonitorFromPoint(p, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(hmon, &mut mi).as_bool() {
            let r = mi.rcWork;
            let cx = (r.left + r.right) / 2;
            let cy = (r.top + r.bottom) / 2;
            Some((cx - wp / 2, (cy - hp / 2).max(r.top)))
        } else {
            None
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn centre_window(_wp: i32, _hp: i32) -> Option<(i32, i32)> {
    None
}
