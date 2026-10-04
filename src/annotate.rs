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
//!      (tiny-skia for vectors, parley + skrifa for text) and stashes the result.
//!   4. `take_result()` (drained each app tick) hands the new image to the app,
//!      which rewrites the snip exactly like a crop.

use slint::{Brush, Color, ComponentHandle, Image, ModelRc, SharedPixelBuffer, SharedString, VecModel};
use std::cell::RefCell;
use std::rc::Rc;
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
    /// True while the editor window is on screen. Backed by live window
    /// visibility (the `window` field lingers `Some` after close), so the
    /// pump stays awake while the user is annotating and sleeps once closed.
    pub fn is_open(&self) -> bool {
        self.window
            .as_ref()
            .map_or(false, |w| w.window().is_visible())
    }

    /// An open editor holding work that closing it would lose.
    pub fn has_unsaved(&self) -> bool {
        self.window
            .as_ref()
            .filter(|w| w.window().is_visible())
            .is_some_and(|w| has_unsaved(&self.shared.borrow(), w))
    }

    /// The snip the open editor is working on, if one is open.
    pub fn editing(&self) -> Option<Uuid> {
        if self.is_open() {
            self.shared.borrow().id
        } else {
            None
        }
    }

    /// Bring the open editor back to the front.
    pub fn raise(&self) {
        if let Some(w) = &self.window {
            let _ = w.show();
        }
    }

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

        // The window's own close button goes through the same confirmation as
        // Cancel and Esc: it used to discard every shape without asking.
        let shared_close = self.shared.clone();
        let weak_close = w.as_weak();
        w.window().on_close_requested(move || {
            let unsaved = weak_close
                .upgrade()
                .is_some_and(|c| has_unsaved(&shared_close.borrow(), &c));
            if unsaved {
                if let Some(c) = weak_close.upgrade() {
                    c.set_confirm_discard(true);
                }
                return slint::CloseRequestResponse::KeepWindowShown;
            }
            let mut st = shared_close.borrow_mut();
            st.id = None;
            st.base = None;
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

/// Shapes drawn, or text typed but not yet committed.
fn has_unsaved(s: &Shared, w: &AnnotateWindow) -> bool {
    !s.shapes.is_empty() || (w.get_editing_text() && !w.get_pending_text().is_empty())
}

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
            draw_text(&mut pm, shape);
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

    out
}

// ---- Text --------------------------------------------------------------

/// Draw a text shape into the pixmap, the way the editor shows it.
///
/// The preview is a Slint `Text`, laid out by parley over fontique: the
/// system's sans-serif and its fallbacks, bold, `text_size` as the em size.
/// The same parley does the layout here, so the saved image gets the face,
/// weight, size and glyph positions the user placed — and with it fallback for
/// scripts the main face lacks, shaping for the ones that need it (Arabic,
/// Devanagari…) and bidi. Glyph outlines come from skrifa and are filled by
/// tiny-skia along with the rest of the shapes. Colour glyphs (emoji) have no
/// outline and are left out.
fn draw_text(pm: &mut tiny_skia::Pixmap, shape: &Shape) {
    use skrifa::instance::{LocationRef, NormalizedCoord, Size};
    use skrifa::outline::DrawSettings;
    use skrifa::MetadataProvider;

    if shape.text.trim().is_empty() {
        return;
    }
    let (ax, ay) = shape.pts.first().copied().unwrap_or((0.0, 0.0));
    let [r, g, b, a] = shape.color;
    let mut paint = tiny_skia::Paint {
        anti_alias: true,
        ..Default::default()
    };
    paint.set_color_rgba8(r, g, b, a);

    TEXT_LAYOUT.with(|cx| {
        let (fonts, layouts) = &mut *cx.borrow_mut();
        let mut builder = layouts.ranged_builder(fonts, &shape.text, 1.0, true);
        builder.push_default(parley::FontFamily::List(std::borrow::Cow::Borrowed(&[
            parley::FontFamilyName::Generic(parley::GenericFamily::SansSerif),
            parley::FontFamilyName::Generic(parley::GenericFamily::SystemUi),
        ])));
        builder.push_default(parley::StyleProperty::FontSize(shape.text_size.max(1.0)));
        builder.push_default(parley::StyleProperty::FontWeight(parley::FontWeight::BOLD));
        let mut layout: parley::Layout<()> = builder.build(&shape.text);
        layout.break_all_lines(None);

        for line in layout.lines() {
            for item in line.items() {
                let parley::PositionedLayoutItem::GlyphRun(glyphs) = item else {
                    continue;
                };
                let run = glyphs.run();
                let font_data = run.font();
                let Ok(font) =
                    skrifa::FontRef::from_index(font_data.data.as_ref(), font_data.index)
                else {
                    continue;
                };
                let outlines = font.outline_glyphs();
                let coords: Vec<NormalizedCoord> = run
                    .normalized_coords()
                    .iter()
                    .map(|c| NormalizedCoord::from_bits(*c))
                    .collect();
                let size = Size::new(run.font_size());
                // A face with no bold of its own gets one drawn thicker, as
                // the renderer does on screen.
                let embolden = run.synthesis().embolden().then(|| tiny_skia::Stroke {
                    width: run.font_size() / 24.0,
                    line_join: tiny_skia::LineJoin::Round,
                    ..Default::default()
                });
                for glyph in glyphs.positioned_glyphs() {
                    let Some(outline) = outlines.get(skrifa::GlyphId::new(glyph.id)) else {
                        continue;
                    };
                    let mut pen = GlyphPen {
                        path: tiny_skia::PathBuilder::new(),
                        x: ax + glyph.x,
                        y: ay + glyph.y,
                    };
                    let settings = DrawSettings::unhinted(size, LocationRef::new(&coords));
                    if outline.draw(settings, &mut pen).is_err() {
                        continue;
                    }
                    let Some(path) = pen.path.finish() else {
                        continue;
                    };
                    let identity = tiny_skia::Transform::identity();
                    pm.fill_path(&path, &paint, tiny_skia::FillRule::Winding, identity, None);
                    if let Some(stroke) = &embolden {
                        pm.stroke_path(&path, &paint, stroke, identity, None);
                    }
                }
            }
        }
    });
}

thread_local! {
    /// The font index takes a moment to build (fontconfig on Linux,
    /// DirectWrite on Windows), so it is built on the first save with text and
    /// kept for the next.
    static TEXT_LAYOUT: RefCell<(parley::FontContext, parley::LayoutContext<()>)> =
        RefCell::new((parley::FontContext::new(), parley::LayoutContext::new()));
}

/// Turns a glyph outline (font units scaled to px, y up, origin on the
/// baseline) into a tiny-skia path placed at the glyph's spot in the image.
struct GlyphPen {
    path: tiny_skia::PathBuilder,
    x: f32,
    y: f32,
}

impl skrifa::outline::OutlinePen for GlyphPen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.path.move_to(self.x + x, self.y - y);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.path.line_to(self.x + x, self.y - y);
    }
    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.path.quad_to(self.x + cx0, self.y - cy0, self.x + x, self.y - y);
    }
    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.path.cubic_to(
            self.x + cx0,
            self.y - cy0,
            self.x + cx1,
            self.y - cy1,
            self.x + x,
            self.y - y,
        );
    }
    fn close(&mut self) {
        self.path.close();
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn text(text: &str, at: (f32, f32), size: f32) -> Shape {
        Shape {
            kind: Kind::Text,
            pts: vec![at],
            color: [255, 0, 0, 255],
            stroke: 0.0,
            text: text.into(),
            text_size: size,
        }
    }

    /// Bounding box of the pixels that are no longer white.
    fn inked(img: &image::RgbaImage) -> Option<(u32, u32, u32, u32)> {
        let mut bbox: Option<(u32, u32, u32, u32)> = None;
        for (x, y, px) in img.enumerate_pixels() {
            if px.0 != [255, 255, 255, 255] {
                let b = bbox.get_or_insert((x, y, x, y));
                *b = (b.0.min(x), b.1.min(y), b.2.max(x), b.3.max(y));
            }
        }
        bbox
    }

    #[test]
    fn text_is_baked_where_it_was_placed() {
        let base = image::RgbaImage::from_pixel(400, 200, image::Rgba([255, 255, 255, 255]));
        let out = flatten(&base, &[text("Snipdeck", (50.0, 40.0), 40.0)]);
        let (l, t, r, b) = inked(&out).expect("text left no mark — no system font?");
        // The text box starts at the anchor and is one line tall (Noto Sans's
        // line is 1.36 em): the glyphs sit inside it, not above or left of the
        // point placed.
        assert!((50..60).contains(&l), "left edge {l}");
        assert!((40..60).contains(&t), "top edge {t}");
        assert!(b < 40 + 60, "bottom edge {b}");
        // Eight bold letters at 40 px run well past 100 px.
        assert!(r - l > 100, "width {}", r - l);
    }

    #[test]
    fn blank_text_leaves_the_image_alone() {
        let base = image::RgbaImage::from_pixel(64, 64, image::Rgba([255, 255, 255, 255]));
        let out = flatten(&base, &[text("   ", (10.0, 10.0), 20.0)]);
        assert!(inked(&out).is_none());
    }
}
