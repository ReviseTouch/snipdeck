//! OCR-translate result popup manager.
//!
//! Owns the `TranslatePopupWindow` (one reused borderless window) and the
//! worker-thread results channel. The flow:
//!   1. app calls `open(id, image, target, x, y)` → popup shows "translating…"
//!      and a worker thread runs OCR then translation.
//!   2. `poll()` (each tick) drains finished work and fills the popup.
//!   3. picking a language in the strip re-translates the cached OCR text on
//!      the fly and flags the choice for the app to persist into Settings —
//!      there is no settings window.

use crossbeam_channel::{unbounded, Receiver, Sender};
use slint::{ComponentHandle, ModelRc, PhysicalPosition, SharedString, VecModel};
use std::cell::RefCell;
use std::rc::Rc;
use uuid::Uuid;

use crate::translate::{self, Translation};
use crate::{TransLang, TranslatePopupWindow};

/// Worker → UI message: a finished OCR+translate (or re-translate) attempt.
struct TransResult {
    id: Uuid,
    /// OCR'd source text. Empty on a re-translate (source already known).
    source: String,
    /// Target code this attempt translated *to* (to detect a mid-flight switch).
    target: String,
    result: Result<Translation, String>,
}

/// Shared between the manager and the popup's Slint callbacks.
struct Shared {
    /// Snip the popup is currently showing (None once dismissed).
    current: Option<Uuid>,
    /// OCR text for the current popup, cached so a language switch
    /// re-translates without re-running OCR. Empty until OCR completes.
    source: String,
    /// Target code the user most recently asked for.
    target: String,
    /// Set when the user picks a new language; drained by the app to persist.
    pending_persist: Option<String>,
    /// Current favourite (quick-pick) language codes, in pill order.
    favs: Vec<String>,
    /// Set when the favourites change (star toggle); drained by the app to
    /// persist into Settings.
    pending_favs: Option<Vec<String>>,
}

pub struct TranslatePopup {
    window: Option<TranslatePopupWindow>,
    tx: Sender<TransResult>,
    rx: Receiver<TransResult>,
    shared: Rc<RefCell<Shared>>,
    /// Last (logical) window size, carried across re-opens so a resize sticks
    /// for the session. None until the user resizes (default size is used).
    last_size: Option<(f32, f32)>,
}

impl TranslatePopup {
    pub fn new() -> Self {
        let (tx, rx) = unbounded();
        Self {
            window: None,
            tx,
            rx,
            shared: Rc::new(RefCell::new(Shared {
                current: None,
                source: String::new(),
                target: "en".into(),
                pending_persist: None,
                favs: Vec::new(),
                pending_favs: None,
            })),
            last_size: None,
        }
    }

    /// Open the popup for `id` in the loading state and kick off OCR+translate
    /// on a worker thread. `anchor` is the snip's physical screen rect
    /// `(x, y, w, h)`; the popup opens to its right (or to its left if that
    /// would run off the screen) and is clamped to stay fully on the monitor —
    /// never clipped at an edge (the software renderer won't repaint a region
    /// that was off-screen). `scale` is the UI scale factor, used to size the
    /// popup in physical pixels for the clamp maths.
    /// True while the popup is actually on screen (not merely constructed —
    /// `window` lingers `Some` after close). Backed by the window's live
    /// visibility, so the pump can stay awake while async OCR/translate
    /// results are still landing and stop the moment the user closes it.
    pub fn is_open(&self) -> bool {
        self.window
            .as_ref()
            .map_or(false, |w| w.window().is_visible())
    }

    pub fn open(
        &mut self,
        id: Uuid,
        image: image::RgbaImage,
        target: &str,
        favs: &[String],
        anchor: (i32, i32, i32, i32),
        scale: f32,
    ) -> Result<(), slint::PlatformError> {
        // Carry the user's last window size over to the new popup so a resize
        // sticks across re-opens within the session.
        let carry = self
            .window
            .as_ref()
            .map(|w| (w.get_win_w(), w.get_win_h()))
            .or(self.last_size);
        // Drop any previous popup so we never strand a top-most window.
        if let Some(w) = self.window.take() {
            let _ = w.hide();
        }
        self.last_size = carry;
        {
            let mut sh = self.shared.borrow_mut();
            sh.current = Some(id);
            sh.source.clear();
            sh.target = target.to_string();
            sh.pending_persist = None;
            sh.favs = favs.to_vec();
            sh.pending_favs = None;
        }

        let w = TranslatePopupWindow::new()?;
        if let Some((cw, ch)) = carry {
            w.set_win_w(cw);
            w.set_win_h(ch);
        }
        let (fav_model, all_model) = build_models(favs);
        w.set_favs(fav_model);
        w.set_all_langs(all_model);
        w.set_loading(true);
        w.set_error_text(SharedString::from(""));
        w.set_source_text(SharedString::from(""));
        w.set_translated_text(SharedString::from(""));
        w.set_detected_lang(SharedString::from(""));
        w.set_target_lang(SharedString::from(target));
        w.set_target_label(SharedString::from(translate::lang_label(target)));

        // pick-lang: switch target, re-translate cached source (if any), and
        // flag the choice so the app persists it into Settings.
        let shared_pick = self.shared.clone();
        let tx_pick = self.tx.clone();
        let weak_pick = w.as_weak();
        w.on_pick_lang(move |code| {
            let code = code.to_string();
            let (cur, source) = {
                let mut sh = shared_pick.borrow_mut();
                sh.target = code.clone();
                sh.pending_persist = Some(code.clone());
                (sh.current, sh.source.clone())
            };
            if let Some(c) = weak_pick.upgrade() {
                c.set_target_lang(SharedString::from(code.as_str()));
                c.set_target_label(SharedString::from(translate::lang_label(&code)));
                c.set_error_text(SharedString::from(""));
                c.set_loading(true);
            }
            if let Some(id) = cur {
                if !source.trim().is_empty() {
                    spawn_retranslate(tx_pick.clone(), id, source, code);
                }
                // If OCR hasn't finished yet, the in-flight result is
                // re-translated to the new target on arrival (see `poll`).
            }
        });

        // copy: translated text → clipboard.
        let weak_copy = w.as_weak();
        w.on_copy(move || {
            if let Some(c) = weak_copy.upgrade() {
                let txt = c.get_translated_text().to_string();
                if !txt.is_empty() {
                    let _ = crate::clipboard::copy_text(&txt);
                }
            }
        });

        // toggle-fav: add/remove a language from the quick-pick pills. Keeps at
        // least one favourite so the pill strip is never empty, and flags the
        // new set for the app to persist into Settings.
        let shared_fav = self.shared.clone();
        let weak_fav = w.as_weak();
        w.on_toggle_fav(move |code| {
            let code = code.to_string();
            let favs = {
                let mut sh = shared_fav.borrow_mut();
                match sh.favs.iter().position(|f| f.eq_ignore_ascii_case(&code)) {
                    Some(pos) => {
                        if sh.favs.len() > 1 {
                            sh.favs.remove(pos);
                        }
                    }
                    None => sh.favs.push(code.clone()),
                }
                sh.pending_favs = Some(sh.favs.clone());
                sh.favs.clone()
            };
            if let Some(c) = weak_fav.upgrade() {
                let (fav_model, all_model) = build_models(&favs);
                c.set_favs(fav_model);
                c.set_all_langs(all_model);
            }
        });

        // copy-source: OCR text → clipboard.
        let weak_cs = w.as_weak();
        w.on_copy_source(move || {
            if let Some(c) = weak_cs.upgrade() {
                let txt = c.get_source_text().to_string();
                if !txt.is_empty() && txt != "—" {
                    let _ = crate::clipboard::copy_text(&txt);
                }
            }
        });

        let shared_dismiss = self.shared.clone();
        let weak_dismiss = w.as_weak();
        w.on_dismissed(move || {
            shared_dismiss.borrow_mut().current = None;
            if let Some(c) = weak_dismiss.upgrade() {
                let _ = c.hide();
            }
        });

        // Title-bar drag → move the OS window (same pointer-delta trick the
        // floating snips use; the title re-anchors so the delta self-zeroes).
        // Clamp to the monitor so it can never be dragged partly off-screen
        // (which the software renderer would leave half-drawn).
        let weak_move = w.as_weak();
        w.on_move_delta(move |dx, dy| {
            if let Some(c) = weak_move.upgrade() {
                let pos = c.window().position();
                let size = c.window().size();
                let nx = pos.x + dx as i32;
                let ny = pos.y + dy as i32;
                let (wl, wt, wr, wb) = work_area_at(nx, ny);
                let nx = nx.clamp(wl, (wr - size.width as i32).max(wl));
                let ny = ny.clamp(wt, (wb - size.height as i32).max(wt));
                c.window().set_position(PhysicalPosition::new(nx, ny));
            }
        });

        // Place next to the anchor, fully on-screen. Size in physical pixels is
        // logical (win-w/win-h, incl. any carried resize) × scale.
        let scale = if scale > 0.05 { scale } else { 1.0 };
        let pw = (w.get_win_w() * scale).round() as i32;
        let ph = (w.get_win_h() * scale).round() as i32;
        let (x, y) = place_popup(anchor, pw, ph);
        w.window().set_position(PhysicalPosition::new(x, y));
        w.show()?;
        self.window = Some(w);

        spawn_ocr_translate(self.tx.clone(), id, image, target.to_string());
        Ok(())
    }

    /// Drain worker results and update the popup. Called each tick.
    pub fn poll(&mut self) {
        while let Ok(res) = self.rx.try_recv() {
            // Ignore results for a popup that's been closed/replaced.
            if self.shared.borrow().current != Some(res.id) {
                continue;
            }
            let Some(w) = &self.window else { continue };
            match res.result {
                Ok(tr) => {
                    // Cache the OCR source the first time we learn it.
                    if !res.source.is_empty() {
                        self.shared.borrow_mut().source = res.source.clone();
                    }
                    let (want, source) = {
                        let sh = self.shared.borrow();
                        (sh.target.clone(), sh.source.clone())
                    };
                    // User switched language while this attempt was in flight —
                    // re-translate to the newest target and keep the spinner.
                    if want != res.target && !source.trim().is_empty() {
                        spawn_retranslate(self.tx.clone(), res.id, source.clone(), want.clone());
                        w.set_target_lang(SharedString::from(want.as_str()));
                        w.set_target_label(SharedString::from(translate::lang_label(&want)));
                        w.set_loading(true);
                        continue;
                    }
                    let shown_source = if res.source.is_empty() {
                        source
                    } else {
                        res.source
                    };
                    w.set_loading(false);
                    w.set_error_text(SharedString::from(""));
                    if !shown_source.is_empty() {
                        w.set_source_text(SharedString::from(shown_source.as_str()));
                    }
                    w.set_translated_text(SharedString::from(tr.text.as_str()));
                    w.set_detected_lang(SharedString::from(translate::lang_label(&tr.source_lang)));
                }
                Err(e) => {
                    // Even when translation failed, show the OCR text if we got it.
                    if !res.source.is_empty() {
                        self.shared.borrow_mut().source = res.source.clone();
                        w.set_source_text(SharedString::from(res.source.as_str()));
                    }
                    w.set_loading(false);
                    w.set_error_text(SharedString::from(e.as_str()));
                }
            }
        }
    }

    /// If the user picked a new target language, return it once so the app can
    /// persist it into Settings. Cleared after read.
    pub fn take_persist(&mut self) -> Option<String> {
        self.shared.borrow_mut().pending_persist.take()
    }

    /// If the favourites changed (star toggled), return the new set once so the
    /// app can persist it into Settings. Cleared after read.
    pub fn take_fav_persist(&mut self) -> Option<Vec<String>> {
        self.shared.borrow_mut().pending_favs.take()
    }
}

/// Compact pill label for a code: the part before any '-' upper-cased
/// ("en" → "EN", "zh-CN" → "ZH").
fn short_code(code: &str) -> String {
    code.split('-').next().unwrap_or(code).to_uppercase()
}

/// Build the two language models the popup needs: the favourite pills (in the
/// given order) and the full "More" list (each flagged with its fav state).
fn build_models(favs: &[String]) -> (ModelRc<TransLang>, ModelRc<TransLang>) {
    let fav_items: Vec<TransLang> = favs
        .iter()
        .map(|c| TransLang {
            code: SharedString::from(c.as_str()),
            label: SharedString::from(translate::target_label(c)),
            short: SharedString::from(short_code(c)),
            fav: true,
        })
        .collect();

    let all_items: Vec<TransLang> = translate::TARGET_LANGS
        .iter()
        .map(|(code, label)| TransLang {
            code: SharedString::from(*code),
            label: SharedString::from(*label),
            short: SharedString::from(short_code(code)),
            fav: favs.iter().any(|f| f.eq_ignore_ascii_case(code)),
        })
        .collect();

    (
        ModelRc::new(VecModel::from(fav_items)),
        ModelRc::new(VecModel::from(all_items)),
    )
}

fn spawn_ocr_translate(tx: Sender<TransResult>, id: Uuid, image: image::RgbaImage, target: String) {
    std::thread::spawn(move || {
        let result = match crate::ocr::extract_text(&image) {
            Ok(src) => match translate::translate(&src, &target) {
                Ok(tr) => TransResult {
                    id,
                    source: src,
                    target,
                    result: Ok(tr),
                },
                Err(e) => TransResult {
                    id,
                    source: src,
                    target,
                    result: Err(format!("Translation: {e}")),
                },
            },
            Err(e) => TransResult {
                id,
                source: String::new(),
                target,
                result: Err(format!("OCR: {e}")),
            },
        };
        let _ = tx.send(result);
        // Result is enqueued; wake the pump so `poll()` drains it into the
        // popup even if the loop had gone to sleep behind the spinner.
        crate::pump::wake();
    });
}

fn spawn_retranslate(tx: Sender<TransResult>, id: Uuid, source: String, target: String) {
    std::thread::spawn(move || {
        let result = match translate::translate(&source, &target) {
            Ok(tr) => TransResult {
                id,
                source: String::new(),
                target,
                result: Ok(tr),
            },
            Err(e) => TransResult {
                id,
                source: String::new(),
                target,
                result: Err(format!("Translation: {e}")),
            },
        };
        let _ = tx.send(result);
        crate::pump::wake();
    });
}

/// Choose the popup's top-left so it sits beside the snip and stays fully on
/// the monitor: prefer the snip's right side, fall back to its left if the
/// right would overflow, then clamp into the work area as a final guarantee.
fn place_popup(anchor: (i32, i32, i32, i32), pw: i32, ph: i32) -> (i32, i32) {
    let (ax, ay, aw, ah) = anchor;
    let (wl, wt, wr, wb) = work_area_at(ax + aw / 2, ay + ah / 2);
    let gap = 12;
    let mut x = ax + aw + gap;
    if x + pw > wr {
        x = ax - pw - gap; // would clip on the right → put it on the left
    }
    let x = x.clamp(wl, (wr - pw).max(wl));
    let y = ay.clamp(wt, (wb - ph).max(wt));
    (x, y)
}

/// The work area (screen minus taskbar) of the monitor nearest `(x, y)`, in
/// physical pixels, as `(left, top, right, bottom)`.
#[cfg(target_os = "windows")]
fn work_area_at(x: i32, y: i32) -> (i32, i32, i32, i32) {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    unsafe {
        let hmon = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(hmon, &mut mi).as_bool() {
            let r = mi.rcWork;
            (r.left, r.top, r.right, r.bottom)
        } else {
            (x - 4000, y - 4000, x + 4000, y + 4000)
        }
    }
}

/// Non-Windows fallback: a generous area so the clamp never moves the popup.
#[cfg(not(target_os = "windows"))]
fn work_area_at(x: i32, y: i32) -> (i32, i32, i32, i32) {
    (x - 4000, y - 4000, x + 4000, y + 4000)
}
