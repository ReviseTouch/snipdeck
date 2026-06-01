use slint::{ComponentHandle, Image, Model, ModelRc, SharedPixelBuffer, SharedString, VecModel};
use std::rc::Rc;
use uuid::Uuid;

use crate::annotate::Annotator;
use crate::capture::{self, CapturedScreen};
use crate::clipboard;
use crate::context_menu::{ContextAction, ContextMenuManager};
use crate::floating::FloatingManager;
use crate::hotkeys::{HotkeyAction, HotkeyService};
use crate::mouse_trigger::{MouseTriggerEvent, MouseTriggerService};
use crate::settings::Settings;
use crate::slint_overlay::SlintOverlayManager;
use crate::snip::{make_snip, GalleryState, ScreenRect};
use crate::translate_popup::TranslatePopup;
use crate::i18n::{t, t1};

use crate::LangItem;
use crate::MainWindow;
use crate::SnipItem;

/// Active in-progress mouse-hook selection.
struct ActiveSelection {
    captured: CapturedScreen,
    start: (i32, i32),
    end: (i32, i32),
    to_clipboard: bool,
    keep_floating: bool,
    source_window: Option<crate::window_metadata::WindowInfo>,
}

#[derive(Debug, Clone, Default)]
struct GalleryFilters {
    status: i32,
    content: i32,
    size: i32,
    date: i32,
    sort: i32,
}

/// Where a "Share" action sends the snip image.
#[derive(Debug, Clone, Copy)]
enum ShareTarget {
    /// Windows native share sheet (image attached directly).
    System,
    /// New mail message with the snip attached, via Simple MAPI.
    Mail,
}

pub struct AppState {
    pub settings: Settings,
    pub gallery: GalleryState,
    pub search: String,
    filters: GalleryFilters,

    hotkey_service: Option<HotkeyService>,
    pub mouse_trigger: MouseTriggerService,
    pub overlay: SlintOverlayManager,
    pub floating: FloatingManager,
    pub context_menu: ContextMenuManager,
    pub translate_popup: TranslatePopup,
    pub annotate: Annotator,

    active: Option<ActiveSelection>,
    /// Win+Shift+Space "freeze first": the screen captured the instant Space
    /// was pressed (with a timestamp), shown frozen in the overlay and reused
    /// when the drag begins. Expired by a watchdog so a stale frozen frame
    /// (~33 MB for a 4K monitor) can't linger or get reused by a later snip.
    frozen_pending: Option<(std::time::Instant, CapturedScreen)>,
    last_error: Option<String>,
    hotkey_error: Option<String>,

    // Decoded-thumbnail cache, bounded (LRU-ish) so total memory stays flat
    // no matter how many snips accumulate; visible cards fetch from here.
    thumb_cache: std::collections::HashMap<Uuid, Image>,
    thumb_order: std::collections::VecDeque<Uuid>,
    /// Flat, already-built cards (id + SnipItem) for the current filter/sort.
    /// Kept so a column-count change can re-chunk into rows cheaply (no
    /// re-filter / re-decode) and survive the rebuild with loaded thumbnails.
    gallery_cards: Vec<(Uuid, SnipItem)>,
    /// Inner per-row models, kept so a lazily-decoded thumbnail can be
    /// written back into the exact card that requested it.
    gallery_rows: Vec<Rc<VecModel<SnipItem>>>,
    /// snip id -> flat index into `gallery_cards`, for thumbnail write-back.
    card_index: std::collections::HashMap<Uuid, usize>,
    /// Cards per row, reported by the UI; drives how the flat list is chunked.
    gallery_cols: usize,
    last_snip_ids: Vec<Uuid>,
    /// Pending snip ids whose floating window should be (re)opened on
    /// next tick. We defer the open by one tick because the snip just
    /// got pushed and Slint can't reliably open a brand-new window from
    /// inside a callback that came from the same event-loop pump.
    pending_open: Vec<Uuid>,
    /// Main window scale factor, refreshed each tick. Floating
    /// windows reuse it so they end up the exact captured pixel size.
    scale_factor: f32,
}

mod input;
mod actions;
mod gallery;

impl AppState {
    pub fn new(window: &MainWindow) -> Self {
        let settings = Settings::load();
        let gallery = GalleryState::load();
        crate::window_tamer::set_floating_in_taskbar(settings.floating_in_taskbar);

        let (hotkey_service, hotkey_error) = match HotkeyService::start(&settings) {
            Ok(hk) => (Some(hk), None),
            Err(e) => (
                None,
                Some(t1("Failed to register hotkeys: {e}", "{e}", &e.to_string())),
            ),
        };

        // The mouse trigger needs a way to wake the UI thread. We use
        // Slint's invoke-from-event-loop mechanism inside `tick`, so the
        // service can stay backend-agnostic; we just poll its channel.
        let _ = window; // (unused for now, kept for symmetry)
                        // Re-use the same service constructor signature as the eframe app:
                        // it accepts an egui::Context purely to wake the UI; we pass a
                        // dummy context-like by going through the alternate constructor.
        let mouse_trigger = MouseTriggerService::start_polling();

        let mut overlay = SlintOverlayManager::new();
        let [r, g, b] = [
            settings.border_color_active[0],
            settings.border_color_active[1],
            settings.border_color_active[2],
        ];
        overlay.set_border_color(r, g, b);

        if let Some(err) = &hotkey_error {
            log::warn!("{err}");
        }

        Self {
            settings,
            gallery,
            search: String::new(),
            filters: GalleryFilters::default(),
            hotkey_service,
            mouse_trigger,
            overlay,
            floating: FloatingManager::new(),
            context_menu: ContextMenuManager::new(),
            translate_popup: TranslatePopup::new(),
            annotate: Annotator::new(),
            active: None,
            frozen_pending: None,
            last_error: None,
            hotkey_error,
            thumb_cache: std::collections::HashMap::new(),
            thumb_order: std::collections::VecDeque::new(),
            gallery_cards: Vec::new(),
            gallery_rows: Vec::new(),
            card_index: std::collections::HashMap::new(),
            gallery_cols: 4,
            last_snip_ids: Vec::new(),
            pending_open: Vec::new(),
            scale_factor: 1.0,
        }
    }

    pub fn wire_callbacks(&mut self, window: &MainWindow) {
        // Theme-aware checkerboard backdrops for the gallery preview wells.
        // Generated once at well aspect (240×148, 2×) with 8px logical cells.
        window.set_checker_light(make_checker(
            480,
            296,
            16,
            [0xe9, 0xe5, 0xdc, 0xff],
            [0xdc, 0xd7, 0xcc, 0xff],
        ));
        window.set_checker_dark(make_checker(
            480,
            296,
            16,
            [0x1a, 0x1a, 0x1a, 0xff],
            [0x21, 0x21, 0x21, 0xff],
        ));

        // We can't easily share &mut self into Slint closures, so all
        // callbacks just push intent flags that `tick` will process by
        // re-reading the model. For one-shot actions we use shared
        // RefCells captured into the closure.
        let weak = window.as_weak();
        let pending: Rc<std::cell::RefCell<Vec<Action>>> =
            Rc::new(std::cell::RefCell::new(Vec::new()));
        // Park the pending queue on the window's user-data — simplest:
        // we attach via weak by stashing it inside the closures. But
        // since `wire_callbacks` runs once and `tick` runs later, we
        // need a shared place. We expose the queue via a global slot.
        ACTION_QUEUE.with(|q| *q.borrow_mut() = Some(pending.clone()));

        let p = pending.clone();
        window.on_new_snip(move || {
            p.borrow_mut().push(Action::NewSnip {
                to_clip: false,
                keep_float: true,
            })
        });
        let p = pending.clone();
        window.on_new_snip_clip(move || {
            p.borrow_mut().push(Action::NewSnip {
                to_clip: true,
                keep_float: true,
            })
        });
        let p = pending.clone();
        window.on_new_snip_clip_only(move || {
            p.borrow_mut().push(Action::NewSnip {
                to_clip: true,
                keep_float: false,
            })
        });
        let p = pending.clone();
        window.on_toggle_all_visibility(move || p.borrow_mut().push(Action::ToggleAllVisibility));
        let p = pending.clone();
        window.on_toggle_floating_in_taskbar(move || {
            p.borrow_mut().push(Action::ToggleFloatingInTaskbar)
        });
        let p = pending.clone();
        window.on_clear_unpinned(move || p.borrow_mut().push(Action::ClearUnpinned));
        let p = pending.clone();
        window.on_clear_all(move || p.borrow_mut().push(Action::ClearAll));
        let p = pending.clone();
        window.on_toggle_autostart(move || p.borrow_mut().push(Action::ToggleAutostart));
        window.set_autostart_enabled(crate::autostart::is_enabled());
        let p = pending.clone();
        window.on_toggle_theme(move || p.borrow_mut().push(Action::ToggleTheme));
        // Populate the toolbar language menu and reflect the active language.
        let langs: Vec<LangItem> = crate::i18n::LANGUAGES
            .iter()
            .map(|(code, name)| LangItem {
                code: SharedString::from(*code),
                name: SharedString::from(*name),
            })
            .collect();
        window.set_languages(ModelRc::new(VecModel::from(langs)));
        window.set_current_language(SharedString::from(crate::i18n::current()));
        let p = pending.clone();
        window.on_set_language(move |code| {
            p.borrow_mut().push(Action::SetLanguage(code.to_string()))
        });
        let p = pending.clone();
        window.on_snip_copy(move |id| p.borrow_mut().push(Action::Copy(uuid_from(&id))));
        let p = pending.clone();
        window.on_snip_save(move |id| p.borrow_mut().push(Action::Save(uuid_from(&id))));
        let p = pending.clone();
        window.on_snip_toggle_pin(move |id| p.borrow_mut().push(Action::TogglePin(uuid_from(&id))));
        let p = pending.clone();
        window.on_snip_delete(move |id| p.borrow_mut().push(Action::Delete(uuid_from(&id))));
        let p = pending.clone();
        window.on_snip_clicked(move |id| p.borrow_mut().push(Action::SnipClicked(uuid_from(&id))));
        let p = pending.clone();
        window.on_snip_context_menu(move |id| {
            p.borrow_mut().push(Action::SnipContextMenu(uuid_from(&id)))
        });
        let p = pending.clone();
        window
            .on_search_changed(move |s| p.borrow_mut().push(Action::SearchChanged(s.to_string())));
        let p = pending.clone();
        window.on_status_filter_changed(move |idx| {
            p.borrow_mut().push(Action::StatusFilterChanged(idx))
        });
        let p = pending.clone();
        window.on_content_filter_changed(move |idx| {
            p.borrow_mut().push(Action::ContentFilterChanged(idx))
        });
        let p = pending.clone();
        window
            .on_size_filter_changed(move |idx| p.borrow_mut().push(Action::SizeFilterChanged(idx)));
        let p = pending.clone();
        window
            .on_date_filter_changed(move |idx| p.borrow_mut().push(Action::DateFilterChanged(idx)));
        let p = pending.clone();
        window.on_sort_changed(move |idx| p.borrow_mut().push(Action::SortChanged(idx)));
        let p = pending.clone();
        window.on_reset_filters(move || p.borrow_mut().push(Action::ResetFilters));
        let p = pending.clone();
        window.on_close_error(move || p.borrow_mut().push(Action::CloseError));
        let p = pending.clone();
        window
            .on_request_thumb(move |id| p.borrow_mut().push(Action::RequestThumb(uuid_from(&id))));
        // Note: `on_gallery_cols_changed` is wired in `main` against the
        // shared state so it can re-chunk synchronously — the timer-driven
        // tick is frozen during a live window-resize (Win32 modal loop), so a
        // queued action wouldn't run until the user released the mouse.
        let _ = weak;
    }

    pub fn tick(&mut self, window: &MainWindow) {
        // Pull the current OS scale factor so floating windows opened
        // during this tick get sized correctly under HiDPI.
        self.scale_factor = window.window().scale_factor() as f32;

        // 1. Drain UI action queue.
        let actions: Vec<Action> = ACTION_QUEUE.with(|q| {
            q.borrow()
                .as_ref()
                .map(|p| std::mem::take(&mut *p.borrow_mut()))
                .unwrap_or_default()
        });
        let had_actions = !actions.is_empty();
        for a in actions {
            self.handle_action(a, window);
        }

        // 2. Hotkeys
        if let Some(hk) = &self.hotkey_service {
            for action in hk.poll() {
                self.handle_hotkey(action, window);
            }
        }

        // 3. Mouse trigger events
        let events: Vec<MouseTriggerEvent> = self.mouse_trigger.rx.try_iter().collect();
        let had_events = !events.is_empty();
        for ev in events {
            self.handle_mouse_event(ev);
        }

        // Watchdog: a freeze-first arm (Win+Shift+Space) the user pressed then
        // abandoned would otherwise leave the hook armed (next click snips) and
        // a full-screen frame pinned. Expire it after a few seconds.
        if let Some((armed_at, _)) = &self.frozen_pending {
            if armed_at.elapsed() > std::time::Duration::from_secs(12) {
                self.frozen_pending = None;
                crate::mouse_trigger::clear_freeze();
                self.overlay.hide();
            }
        }

        // 4. Open any pending floating windows (deferred one tick from
        //    creation so Slint can establish the window cleanly).
        let to_open: Vec<Uuid> = std::mem::take(&mut self.pending_open);
        let sf = self.scale_factor;
        let opened_any = !to_open.is_empty();
        for id in to_open {
            if let Some(snip) = self.gallery.get(id).cloned() {
                if let Err(e) = self
                    .floating
                    .open(&snip, &self.settings, sf, Some(window.window()))
                {
                    self.last_error =
                        Some(t1("Failed to open floating window: {e}", "{e}", &e.to_string()));
                }
            }
        }
        if opened_any {
            self.refresh_gallery(window);
        }

        // 5. Drain floating-window callbacks (drag positions, menu, close, resize).
        let drained = self.floating.drain();
        let had_drain = !drained.position_updates.is_empty()
            || !drained.close_requests.is_empty()
            || !drained.menu_requests.is_empty()
            || !drained.resize_ends.is_empty();
        let mut monitor_changed = false;
        for (id, pos) in drained.position_updates {
            if let Some(s) = self.gallery.get_mut(id) {
                if s.last_position != Some(pos) {
                    s.last_position = Some(pos);
                }
                // Detect monitor changes as the window crosses screens.
                let cx = pos.0 + s.area.w as i32 / 2;
                let cy = pos.1 + s.area.h as i32 / 2;
                if let Some(label) = crate::capture::monitor_label_at(cx, cy) {
                    if s.monitor_label.as_deref() != Some(label.as_str()) {
                        s.monitor_label = Some(label);
                        monitor_changed = true;
                    }
                }
            }
        }
        let had_closes = !drained.close_requests.is_empty();
        for id in drained.close_requests {
            self.floating.close(id);
            if let Some(s) = self.gallery.get_mut(id) {
                s.floating = false;
            }
        }
        if had_closes || monitor_changed {
            self.refresh_gallery(window);
        }
        for id in drained.resize_ends {
            self.handle_resize_end(id, window);
        }
        // Highlight the most recently pressed snip (runs after the close
        // handling above, and only if the window still exists, so a
        // middle-click that both presses and closes doesn't leave the
        // selection pointing at a window that's already gone).
        if let Some(&id) = drained.select_requests.last() {
            if self.floating.windows.contains_key(&id) {
                self.floating.select(id);
            }
        }
        // The selection highlight is shown only while Snipdeck is the active
        // (foreground) app; whenever the user is in another application every
        // snip is reset to its default border.
        self.floating
            .refresh_selection_display(crate::window_tamer::app_is_foreground());

        for (id, sx, sy) in drained.menu_requests {
            if let Err(e) = self.context_menu.show_for(id, sx, sy, true) {
                self.last_error = Some(t1("Failed to open menu: {e}", "{e}", &e.to_string()));
            }
        }
        // Keep a spare menu window warm so the next right-click opens instantly.
        self.context_menu.prewarm();
        // Process any context menu picks queued since last tick.
        let menu_picks = self.context_menu.drain();
        if !menu_picks.is_empty() {
            for (id, action) in menu_picks {
                self.handle_context_action(id, action, window);
            }
            // Any pick is terminal: the menu hid itself when clicked, so fully
            // reset the manager (clears its open/armed state) for a clean open.
            self.context_menu.hide();
        }

        // Drain finished OCR/translate work into the popup, and persist the
        // last target language the user picked (no settings window exists).
        self.translate_popup.poll();
        if let Some(code) = self.translate_popup.take_persist() {
            if self.settings.translate_target != code {
                self.settings.translate_target = code;
                let _ = self.settings.save();
            }
        }
        if let Some(favs) = self.translate_popup.take_fav_persist() {
            self.settings.translate_favs = favs;
            let _ = self.settings.save();
        }

        // Blink the editor's size-preview indicator while a slider is dragged.
        self.annotate.pulse_tick();
        // Apply a finished annotation (the editor's Save) back onto the snip.
        if let Some((id, img)) = self.annotate.take_result() {
            self.apply_annotation(id, img, window);
        }

        // A context menu belongs to exactly one snip. If that snip's
        // floating window has since closed — via middle-click, a hotkey,
        // clear-all, or even the menu's own Close/Delete action — the menu
        // would otherwise linger on screen pointing at nothing. Dismiss it
        // once its target is gone. (Runs after every close path above, so
        // it catches all of them uniformly.)
        if let Some(target) = self.context_menu.floating_target() {
            if !self.floating.windows.contains_key(&target) {
                self.context_menu.hide();
            }
        }
        // Native context-menu dismissal: a mouse press anywhere outside the
        // open menu closes it (the first click elsewhere). No-op when closed.
        self.context_menu.poll_auto_close();

        if had_drain {
            let _ = self.gallery.save();
        }

        // Refresh gallery model if mouse-hook produced a new snip.
        if had_events && self.gallery.snips.len() != self.last_snip_ids.len() {
            self.last_snip_ids = self.gallery.snips.iter().map(|s| s.id).collect();
            self.refresh_gallery(window);
        }

        // 6. Push error/hotkey banners and reflect floating state.
        let banner = self
            .last_error
            .clone()
            .or_else(|| self.hotkey_error.clone())
            .unwrap_or_default();
        window.set_error_text(SharedString::from(banner));
        window.set_search_text(SharedString::from(self.search.clone()));
        window.set_status_filter_index(self.filters.status);
        window.set_content_filter_index(self.filters.content);
        window.set_size_filter_index(self.filters.size);
        window.set_date_filter_index(self.filters.date);
        window.set_sort_index(self.filters.sort);
        window.set_any_floating(!self.floating.windows.is_empty());
        window.set_floating_in_taskbar(self.settings.floating_in_taskbar);
        window.set_dark_theme(self.settings.dark_theme);

        let _ = (had_actions, had_events);
    }
}

/// Build a checkerboard tile image of `w`×`h` pixels with `cell`-sized
/// squares alternating between `c0` and `c1` (RGBA). Used as the gallery
/// preview backdrop so letterboxed snips read against a neutral ground.
fn make_checker(w: u32, h: u32, cell: u32, c0: [u8; 4], c1: [u8; 4]) -> Image {
    let mut buf = SharedPixelBuffer::<slint::Rgba8Pixel>::new(w, h);
    let px = buf.make_mut_slice();
    for y in 0..h {
        for x in 0..w {
            let on = ((x / cell) % 2) == ((y / cell) % 2);
            let c = if on { c0 } else { c1 };
            px[(y * w + x) as usize] = slint::Rgba8Pixel {
                r: c[0],
                g: c[1],
                b: c[2],
                a: c[3],
            };
        }
    }
    Image::from_rgba8(buf)
}

fn spawn_ocr_index(id: Uuid, image: image::RgbaImage) {
    std::thread::spawn(move || match crate::ocr::extract_text(&image) {
        Ok(text) => {
            if let Err(err) = crate::snip::save_ocr_text(id, crate::ocr::engine_name(), None, &text)
            {
                log::warn!("Failed to write OCR text to DB: {err}");
            }
        }
        Err(err) => {
            log::debug!("OCR index skipped for {id}: {err}");
        }
    });
}

fn spawn_ocr_copy(id: Uuid, snip: crate::snip::Snip) {
    std::thread::spawn(move || match snip.load_image() {
        Ok(img) => match crate::ocr::extract_text(&img) {
            Ok(text) => {
                if let Err(err) =
                    crate::snip::save_ocr_text(id, crate::ocr::engine_name(), None, &text)
                {
                    log::warn!("Failed to write OCR text to DB: {err}");
                }
                if let Err(err) = crate::clipboard::copy_text(&text) {
                    log::warn!("Failed to copy OCR text to clipboard: {err}");
                }
            }
            Err(err) => {
                log::warn!("OCR copy failed: {err}");
            }
        },
        Err(err) => {
            log::warn!("Failed to read image for OCR: {err}");
        }
    });
}

fn uuid_from(s: &SharedString) -> Uuid {
    Uuid::parse_str(s.as_str()).unwrap_or_else(|_| Uuid::nil())
}

/// Current mouse cursor position in physical screen coordinates, used to place
/// the gallery-card right-click menu at the pointer.
#[cfg(target_os = "windows")]
fn cursor_screen_pos() -> Option<(i32, i32)> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
    let mut pt = POINT::default();
    unsafe { GetCursorPos(&mut pt).ok()? };
    Some((pt.x, pt.y))
}

#[cfg(not(target_os = "windows"))]
fn cursor_screen_pos() -> Option<(i32, i32)> {
    None
}

#[derive(Debug)]
pub(crate) enum Action {
    NewSnip { to_clip: bool, keep_float: bool },
    ToggleAllVisibility,
    ToggleFloatingInTaskbar,
    ClearUnpinned,
    ClearAll,
    ToggleAutostart,
    ToggleTheme,
    SetLanguage(String),
    Copy(Uuid),
    Save(Uuid),
    TogglePin(Uuid),
    Delete(Uuid),
    SnipClicked(Uuid),
    SnipContextMenu(Uuid),
    SearchChanged(String),
    StatusFilterChanged(i32),
    ContentFilterChanged(i32),
    SizeFilterChanged(i32),
    DateFilterChanged(i32),
    SortChanged(i32),
    ResetFilters,
    CloseError,
    /// A card scrolled into view and wants its thumbnail decoded + shown.
    RequestThumb(Uuid),
}

fn snip_pixels(snip: &crate::snip::Snip) -> u64 {
    snip.area.w as u64 * snip.area.h as u64
}

fn short_date(dt: chrono::DateTime<chrono::Utc>) -> String {
    let local = dt.with_timezone(&chrono::Local);
    let months = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let month = months
        .get(
            local
                .format("%m")
                .to_string()
                .parse::<usize>()
                .unwrap_or(1)
                .saturating_sub(1),
        )
        .copied()
        .unwrap_or("May");
    format!("{} {} {}", local.format("%d"), t(month), local.format("%H:%M"))
}

fn snip_source(snip: &crate::snip::Snip, has_window: bool) -> String {
    let monitor = snip
        .monitor_label
        .clone()
        .unwrap_or_else(|| t("Monitor"));
    if has_window {
        format!("{monitor} · {}", t("window"))
    } else {
        monitor
    }
}

fn snip_preview(
    snip: &crate::snip::Snip,
    has_ocr: bool,
    has_window: bool,
) -> (&'static str, String) {
    if let Some(label) = snip
        .label
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return ("TAG", label.to_string());
    }
    if has_ocr {
        return ("OCR", t("OCR text indexed"));
    }
    if has_window {
        return ("WIN", t("Window info indexed"));
    }
    ("", String::new())
}

fn storage_size_text(snips: &[crate::snip::Snip]) -> String {
    let bytes: u64 = snips
        .iter()
        .filter_map(|snip| std::fs::metadata(snip.image_path()).ok())
        .map(|meta| meta.len())
        .sum();
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

thread_local! {
    static ACTION_QUEUE: std::cell::RefCell<Option<Rc<std::cell::RefCell<Vec<Action>>>>> =
        std::cell::RefCell::new(None);
}
