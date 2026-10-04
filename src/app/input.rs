use super::*;
use crate::i18n::t1;

/// How long to let the compositor repaint after we hide our own window, before
/// grabbing the screen.
///
/// `window.hide()` only *requests* the unmap; the window stays on screen until
/// the compositor processes it and repaints what was behind. Capturing in the
/// same tick photographed Snipdeck's own window.
///
/// The wait has to outlast the compositor's close animation, not just the
/// unmap: GNOME fades a closing window over roughly 200 ms, and anything
/// shorter catches it mid-fade — a ghost of the window in the shot. 250 ms
/// clears that with margin and still reads as immediate.
#[cfg(not(target_os = "windows"))]
const FREEZE_AFTER_HIDE: std::time::Duration = std::time::Duration::from_millis(250);

impl AppState {
    pub(crate) fn handle_action(&mut self, action: Action, window: &MainWindow) {
        match action {
            Action::NewSnip {
                to_clip,
                keep_float,
                to_gallery,
            } => self.begin_region_snip(to_clip, keep_float, to_gallery, window),
            Action::PickColor => self.start_color_pick(Some(window)),
            Action::ToggleFloatingInTaskbar => {
                self.settings.floating_in_taskbar = !self.settings.floating_in_taskbar;
                let _ = self.settings.save();
                crate::window_tamer::set_floating_in_taskbar(self.settings.floating_in_taskbar);
                // Close + reopen every currently-floating snip so the
                // fresh windows pick up the new WS_EX_TOOLWINDOW state
                // at creation (toggling it at runtime is fragile).
                let to_reopen: Vec<crate::snip::Snip> = self
                    .gallery
                    .snips
                    .iter()
                    .filter(|s| self.floating.is_shown(s.id))
                    .cloned()
                    .collect();
                self.floating.close_all();
                let sf = self.scale_factor;
                for s in to_reopen {
                    if let Err(e) =
                        self.floating
                            .open(&s, &self.settings, sf, Some(window.window()))
                    {
                        self.last_error =
                            Some(t1("Failed to reopen floating window: {e}", "{e}", &e.to_string()));
                    }
                }
                self.refresh_gallery(window);
            }
            Action::ToggleAllVisibility => {
                if !self.floating.any_shown() {
                    // Reopen all floating snips (those marked `floating: true`).
                    let to_open: Vec<crate::snip::Snip> = self
                        .gallery
                        .snips
                        .iter()
                        .filter(|s| s.floating)
                        .cloned()
                        .collect();
                    let sf = self.scale_factor;
                    for s in to_open {
                        let _ = self
                            .floating
                            .open(&s, &self.settings, sf, Some(window.window()));
                    }
                } else {
                    // Temporary group hide: close the windows but KEEP each
                    // snip's `floating` mark, so toggling back on reopens the
                    // same set (and they still restore on next launch).
                    self.floating.close_all();
                }
                let _ = self.save_gallery();
                self.refresh_gallery(window);
            }
            Action::ClearUnpinned => {
                let unpinned: Vec<Uuid> = self
                    .gallery
                    .snips
                    .iter()
                    .filter(|s| !s.pinned)
                    .map(|s| s.id)
                    .collect();
                for id in unpinned {
                    self.floating.close(id);
                    self.gallery.remove(id);
                    self.forget_thumb(id);
                }
                let _ = self.save_gallery();
                self.refresh_gallery(window);
            }
            Action::ClearAll => {
                self.floating.close_all();
                self.gallery.clear_all();
                self.forget_all_thumbs();
                let _ = self.save_gallery();
                self.refresh_gallery(window);
            }
            Action::ToggleAutostart => match crate::autostart::toggle() {
                Ok(on) => window.set_autostart_enabled(on),
                Err(e) => log::warn!("autostart toggle failed: {e}"),
            },
            Action::ToggleTheme => {
                self.settings.dark_theme = !self.settings.dark_theme;
                let _ = self.settings.save();
                window.set_dark_theme(self.settings.dark_theme);
            }
            Action::SetLanguage(code) => {
                self.settings.language = code.clone();
                let _ = self.settings.save();
                crate::i18n::set_language(&code);
                // Re-translate the bundled Slint UI live; the Rust-provided card
                // text (months / source labels) is rebuilt by refresh_gallery.
                if let Err(e) = slint::select_bundled_translation(&code) {
                    log::warn!("select_bundled_translation({code}) failed: {e:?}");
                }
                window.set_current_language(SharedString::from(crate::i18n::current()));
                self.refresh_gallery(window);
            }
            Action::Copy(id) => {
                if let Some(snip) = self.gallery.get(id) {
                    if let Ok(img) = snip.load_image() {
                        if let Err(e) = clipboard::copy_image(&img) {
                            self.last_error = Some(t1("Failed to copy to clipboard: {e}", "{e}", &e.to_string()));
                        }
                    }
                }
            }
            Action::Save(id) => self.save_snip_to_file(id),
            Action::TogglePin(id) => {
                let new_state = if let Some(s) = self.gallery.get_mut(id) {
                    s.pinned = !s.pinned;
                    s.pinned
                } else {
                    false
                };
                self.floating.refresh_pin(id, new_state);
                let _ = self.save_gallery();
                self.refresh_gallery(window);
            }
            Action::Delete(id) => {
                self.floating.close(id);
                self.gallery.remove(id);
                self.forget_thumb(id);
                let _ = self.save_gallery();
                self.refresh_gallery(window);
            }
            Action::SnipClicked(id) => {
                if let Some(snip) = self.gallery.get(id).cloned() {
                    self.floating.toggle(
                        &snip,
                        &self.settings,
                        self.scale_factor,
                        Some(window.window()),
                    );
                    if let Some(s) = self.gallery.get_mut(id) {
                        s.floating = self.floating.is_shown(id);
                    }
                    let _ = self.save_gallery();
                    self.refresh_gallery(window);
                }
            }
            Action::SnipContextMenu(id) => {
                // Windows places the menu at the cursor. Elsewhere the menu
                // covers the screen and anchors itself where the pointer reports
                // in (`ContextMenuManager::sync_to_window`), so it needs no
                // position — and Wayland has none to give. Requiring one meant a
                // right-click on a gallery card did nothing at all on Linux, and
                // annotate, crop, rotate, share and delete were out of reach for
                // every snip not floating on screen.
                let (x, y) = cursor_screen_pos().unwrap_or((0, 0));
                if let Err(e) = self.context_menu.show_for(id, x, y, false) {
                    self.last_error =
                        Some(t1("Failed to open menu: {e}", "{e}", &e.to_string()));
                }
            }
            Action::SearchChanged(s) => {
                self.search = s;
                self.refresh_gallery(window);
            }
            Action::StatusFilterChanged(idx) => {
                self.filters.status = idx.max(0);
                self.refresh_gallery(window);
            }
            Action::ContentFilterChanged(idx) => {
                self.filters.content = idx.max(0);
                self.refresh_gallery(window);
            }
            Action::SizeFilterChanged(idx) => {
                self.filters.size = idx.max(0);
                self.refresh_gallery(window);
            }
            Action::DateFilterChanged(idx) => {
                self.filters.date = idx.max(0);
                self.refresh_gallery(window);
            }
            Action::SortChanged(idx) => {
                self.filters.sort = idx.max(0);
                self.refresh_gallery(window);
            }
            Action::ThumbSizeChanged(idx) => {
                // Persist the new size step. The grid reflow itself is driven by
                // the UI: a wider/narrower card changes the cards-per-row, which
                // fires `gallery-cols-changed` and re-chunks the rows.
                self.settings.thumb_size_index = idx.clamp(0, 2);
                let _ = self.settings.save();
            }
            Action::ResetFilters => {
                self.search.clear();
                self.filters = GalleryFilters::default();
                self.refresh_gallery(window);
            }
            Action::CloseError => {
                self.last_error = None;
                self.hotkey_error = None;
            }
            Action::OpenCollage => self.open_collage(window),
            Action::ShowAbout => self.show_about(window),
            Action::RequestThumb(id) => {
                self.load_thumb_into_model(id);
            }
        }
    }

    /// Every "capture a region now" trigger funnels through here: the toolbar
    /// button, the tray item and the three capture hotkeys.
    ///
    /// Off Windows there is no global mouse hook to wait on — Wayland forbids
    /// one outright — so the overlay drives the drag itself: freeze the screen
    /// and let its own input capture take over. That freeze is *deferred*; see
    /// `pending_freeze_arm` for why firing it here would photograph the window
    /// we just hid.
    pub(crate) fn begin_region_snip(
        &mut self,
        to_clip: bool,
        keep_float: bool,
        to_gallery: bool,
        window: &MainWindow,
    ) {
        // A second trigger while a snip is already under way — the hotkey
        // pressed twice — is ignored. Going ahead photographed the frozen
        // overlay of the first, dim layer and all, and re-read the main
        // window's visibility after the first trigger had hidden it, so the
        // window never came back.
        if self.snip_in_progress() {
            log::info!("snip trigger ignored: a snip is already in progress");
            return;
        }
        // On Windows the drag is read by the pointer hook. Without one, the
        // window would hide and wait for a click that never comes; say so with
        // the window still on screen, as the colour picker does.
        #[cfg(target_os = "windows")]
        if crate::mouse_trigger::pointer_hook_failed() {
            self.last_error = Some(t(
                "Snipping needs pointer access this session does not grant.",
            ));
            return;
        }
        // Arm a one-shot region drag: the next left-button drag starts a snip
        // with NO modifier held, routed the way the matching Win+drag would.
        self.armed_routing = Some((to_clip, keep_float, to_gallery));
        crate::mouse_trigger::arm_snip();
        // Stamp when we armed so the tick watchdog can recover if the user arms
        // a snip, hides the main window, then never clicks (Esc already
        // cancels; this covers the "walk away" case).
        self.armed_snip_at = Some(std::time::Instant::now());

        // Read the geometry BEFORE hiding: the overlay path uses the window's
        // centre to pick which monitor to freeze (no global cursor without a
        // hook), and a hidden window reports nothing useful.
        #[cfg(not(target_os = "windows"))]
        let centre = {
            let pos = window.window().position();
            let size = window.window().size();
            (
                pos.x + size.width as i32 / 2,
                pos.y + size.height as i32 / 2,
            )
        };

        // Hide Snipdeck's own window so it is neither captured in the shot nor
        // in the way of the region the user wants. It comes back when the snip
        // finishes (Release) or is cancelled (Esc) — see
        // `restore_main_after_snip` / `pending_show_main`.
        // Only a window that was on screen comes back afterwards. A hotkey snip
        // taken while Snipdeck sat in the tray used to throw the gallery up over
        // whatever the user was doing.
        self.restore_main_after_snip = window.window().is_visible();
        let _ = window.hide();
        // Clear any stale banner so a previous error doesn't linger over the
        // armed snip.
        self.last_error = None;

        #[cfg(not(target_os = "windows"))]
        {
            self.pending_freeze_arm =
                Some((centre.0, centre.1, std::time::Instant::now() + FREEZE_AFTER_HIDE));
        }
    }

    /// The screen captured for a freeze-first snip: show it frozen under the
    /// overlay, for the drag that follows (`StartSnip` takes it from
    /// `frozen_pending`).
    pub(crate) fn freeze_captured(&mut self, result: anyhow::Result<crate::capture::CapturedScreen>) {
        match result {
            Ok(captured) => {
                let area = captured.area;
                if let Err(e) =
                    self.overlay
                        .show_for_monitor(area, self.scale_factor, Some(&captured.image))
                {
                    log::warn!("frozen overlay show failed: {e}");
                }
                self.frozen_pending = Some((std::time::Instant::now(), captured));
            }
            Err(e) => {
                self.frozen_pending = None;
                // Capture failed — clear the hook's freeze flag too, or it
                // stays armed and the next plain click starts a snip.
                self.abandon_snip();
                // The banner only has room for the outermost context
                // ("capturing screen"), which on Wayland hides whatever the
                // compositor actually refused. Log the whole chain.
                log::warn!("freeze capture failed: {e:#}");
                self.last_error = Some(t1("Failed to freeze screen: {e}", "{e}", &e.to_string()));
            }
        }
    }

    /// A snip that cannot go ahead — its capture failed — gives everything
    /// back at once: the arm and its routing, the hook's freeze, the overlay,
    /// and the main window if the snip hid it. Left to the watchdogs in `tick`,
    /// the window stayed hidden for twelve seconds, with the banner explaining
    /// the failure inside it, and the arm waited to turn the next ordinary
    /// click into another attempt.
    fn abandon_snip(&mut self) {
        self.armed_snip_at = None;
        self.armed_routing = None;
        crate::mouse_trigger::consume_armed();
        crate::mouse_trigger::clear_freeze();
        self.overlay.hide();
        if std::mem::take(&mut self.restore_main_after_snip) {
            self.pending_show_main = true;
        }
    }

    /// Start the colour picker — from the toolbar, which hides the main window
    /// while picking (`hide`), or from a snip's menu, which leaves it be.
    pub(crate) fn start_color_pick(&mut self, hide: Option<&MainWindow>) {
        // Linux has no pointer hook and no way to read the screen under the
        // cursor; the desktop portal does the picking (see `ColorPicker`).
        #[cfg(target_os = "linux")]
        {
            let _ = hide;
            self.color_picker.pick_via_portal();
            self.last_error = None;
        }
        #[cfg(not(target_os = "linux"))]
        {
            // Both the pick and its Esc arrive as events from the pointer hook.
            // With no hook there is no pick and no cancel — and the toolbar path
            // hides the main window before waiting, so "does nothing" would
            // present as the app vanishing. Say so instead, with the window
            // still on screen to say it in.
            if crate::mouse_trigger::pointer_hook_failed() {
                self.last_error = Some(t(
                    "The colour picker needs pointer access this session does not grant.",
                ));
                return;
            }
            // The loupe follows the cursor; the next click grabs the pixel
            // (ColorPick event), Esc cancels. The watchdog in `tick` ends a pick
            // that never finishes — both entry points stamp it, the snip menu's
            // included, which used to skip it and could leave the loupe up.
            self.color_picker.begin();
            if let Some(window) = hide {
                let _ = window.hide();
                self.color_pick_hid_main = true;
            }
            self.color_pick_started_at = Some(std::time::Instant::now());
            self.last_error = None;
        }
    }

    /// A snip is under way: a drag, a frozen frame, a freeze about to be
    /// taken, or a toolbar / hotkey arm waiting for its drag.
    fn snip_in_progress(&self) -> bool {
        self.active.is_some()
            || self.frozen_pending.is_some()
            || self.pending_freeze_arm.is_some()
            || self.armed_routing.is_some()
            || self.freeze_capture_in_flight()
    }

    /// A freeze capture still running on its thread (Linux; see `FreezeArm`).
    pub(crate) fn freeze_capture_in_flight(&self) -> bool {
        #[cfg(target_os = "linux")]
        {
            self.freeze_capture.is_some()
        }
        #[cfg(not(target_os = "linux"))]
        {
            false
        }
    }

    pub(crate) fn handle_hotkey(&mut self, action: HotkeyAction, window: &MainWindow) {
        match action {
            // The three capture hotkeys go through the same entry point as the
            // toolbar button and the tray item. They used to only `arm_snip()`,
            // which is enough on Windows (the global hook delivers the drag) but
            // a dead end everywhere else: Wayland has no global hook, so the
            // armed click never arrived and the hotkeys did nothing at all.
            HotkeyAction::Snip => self.begin_region_snip(false, true, true, window),
            HotkeyAction::SnipToClipboard => self.begin_region_snip(true, true, true, window),
            HotkeyAction::ClipboardOnly => self.begin_region_snip(true, false, false, window),
            // Acts on the existing windows — no capture flow needed.
            HotkeyAction::ToggleVisibility => {
                self.handle_action(Action::ToggleAllVisibility, window);
            }
            HotkeyAction::Gallery => {
                let _ = window.show();
                crate::tray::focus_main_window();
            }
        }
    }

    /// A finished Super+Alt+drag from the GNOME extension.
    ///
    /// The extension has already done the drag and the capture, so there is no
    /// overlay to show and nothing left to select: the image *is* the region.
    /// It goes in as a selection covering exactly its own pixels, and Release
    /// does the rest — routing, clipboard, gallery, OCR, the floating snip
    /// pinned where it was taken.
    #[cfg(target_os = "linux")]
    pub(crate) fn handle_region_snip(&mut self, snip: crate::bus_service::RegionSnip) {
        // Another snip is under way. Its Release would find this one in
        // `active`, or this one would take the restore flag that brings the
        // main window back — refuse rather than mix the two.
        if self.snip_in_progress() {
            log::warn!("region snip dropped: another snip is in progress");
            return;
        }

        let (w, h) = snip.image.dimensions();
        let (x, y) = (snip.x, snip.y);
        // Alt is what starts the gesture, not a modifier the user picked. Read
        // as one, every Super+Alt snip would be "clipboard only".
        let mods = crate::mouse_trigger::ModifierSet {
            ctrl: snip.ctrl,
            alt: false,
            shift: snip.shift,
        };
        let (to_clipboard, keep_floating, to_gallery) = routing_for_modifiers(mods);
        let monitor_label = capture::monitor_label_or_primary(x, y).unwrap_or_default();

        self.active = Some(ActiveSelection {
            captured: CapturedScreen {
                area: ScreenRect::new(x, y, w, h),
                image: snip.image,
                monitor_label,
            },
            start: (x, y),
            end: (x, y),
            to_clipboard,
            keep_floating,
            to_gallery,
            source_window: None,
        });
        self.handle_mouse_event(MouseTriggerEvent::Release {
            x: x + w as i32,
            y: y + h as i32,
        });
    }

    pub(crate) fn handle_mouse_event(&mut self, event: MouseTriggerEvent) {
        // Every drag step arrives here, ~125 a second with coordinates. At info
        // they filled the journal — where a desktop-launched app's stderr goes —
        // with a trace of the pointer; moves are trace, the rest debug.
        match event {
            MouseTriggerEvent::Move { .. } => log::trace!("mouse event: {event:?}"),
            _ => log::debug!("mouse event: {:?}, active={}", event, self.active.is_some()),
        }
        match event {
            MouseTriggerEvent::StartSnip { x, y, mods } => {
                if self.active.is_some() {
                    log::warn!("StartSnip ignored: active selection in progress");
                    return;
                }
                // Freeze-first (Win+Shift+Space): the screen was already
                // captured and shown frozen on FreezeArm. Reuse it (don't
                // recapture); the result is a normal floating snip that stays on
                // screen, just like Win+click (not clipboard-only — copying is
                // Win+Alt's job). The frozen overlay is already up.
                if let Some((armed_at, captured)) = self.frozen_pending.take() {
                    // Ignore a stale frozen frame (armed long ago, never used) so
                    // an old click can't snip outdated screen content; fall
                    // through to a fresh live capture instead.
                    if armed_at.elapsed() < std::time::Duration::from_secs(12) {
                        let source_window = crate::window_metadata::capture_at_point(x, y);
                        self.overlay.update_sel(x, y, x, y);
                        // A hotkey/toolbar arm may have chosen an explicit
                        // routing (the overlay-driven Linux path always comes
                        // through here); plain Win+Shift+Space keeps the old
                        // default: floating, no clipboard.
                        let (to_clipboard, keep_floating, to_gallery) =
                            self.armed_routing.take().unwrap_or((false, true, true));
                        self.active = Some(ActiveSelection {
                            captured,
                            start: (x, y),
                            end: (x, y),
                            to_clipboard,
                            keep_floating,
                            to_gallery,
                            source_window,
                        });
                        return;
                    }
                    crate::mouse_trigger::clear_freeze();
                    self.overlay.hide();
                }
                // Modifier matrix:
                //   Win+LMouse              → snip + floating window
                //   Win+Ctrl+LMouse         → snip + floating + clipboard
                //   Win+Alt+LMouse          → clipboard only (no gallery)
                //   Win+Shift+LMouse        → gallery only, no floating
                //   Win+Ctrl+Shift+LMouse   → gallery + clipboard, no floating
                // A global hotkey may have armed an explicit routing; otherwise
                // derive it from the modifiers held during the drag.
                //
                // The third flag is what makes Alt different from Ctrl+Shift.
                // With only (to_clipboard, keep_floating) both rows read
                // `(true, false)` — the matrix above promised two routings the
                // data could not tell apart, and the one that lost was the one
                // whose whole point was NOT to keep the snip.
                let (to_clipboard, keep_floating, to_gallery) = self
                    .armed_routing
                    .take()
                    .unwrap_or_else(|| routing_for_modifiers(mods));
                let source_window = crate::window_metadata::capture_at_point(x, y);
                // Debug, not info: a window title is often the very thing being
                // snipped — a document name, a chat, an e-mail subject — and the
                // clipboard-only routing promises to keep nothing.
                if let Some(window) = &source_window {
                    log::debug!(
                        "source window: title={:?} app={:?} pid={:?} bounds={:?}",
                        window.title,
                        window.app_name,
                        window.pid,
                        window.bounds
                    );
                }
                // Capture the monitor the cursor is on (not just the
                // primary), otherwise dragging on a secondary monitor
                // crops out of bounds and the snip is silently dropped.
                match capture::capture_at(x, y) {
                    Ok(captured) => {
                        let area = captured.area;
                        // Slint overlay covers exactly one monitor.
                        // freeze_screen_during_snip is a TODO on the
                        // Slint backend (Win32 backend had it).
                        if let Err(e) =
                            self.overlay.show_for_monitor(area, self.scale_factor, None)
                        {
                            log::warn!("overlay show failed: {e}");
                        }
                        self.overlay.update_sel(x, y, x, y);
                        self.active = Some(ActiveSelection {
                            captured,
                            start: (x, y),
                            end: (x, y),
                            to_clipboard,
                            keep_floating,
                            to_gallery,
                            source_window,
                        });
                    }
                    Err(e) => {
                        self.abandon_snip();
                        log::warn!("snip capture failed: {e:#}");
                        self.last_error = Some(t1("Failed to capture screen: {e}", "{e}", &e.to_string()));
                    }
                }
            }
            MouseTriggerEvent::FreezeArm { x, y } => {
                // Win+Shift+Space: capture the screen NOW (before any click, so
                // hover tooltips are still visible) and show it frozen + dimmed
                // as the overlay backdrop. The drag that starts the selection
                // arrives as the next StartSnip, which reuses this capture.
                if self.active.is_some() {
                    // The hook armed the freeze before sending this; disarm it,
                    // or with no frozen frame for the watchdog to expire, the
                    // next plain click would start a snip.
                    crate::mouse_trigger::clear_freeze();
                    return;
                }
                // Off the UI thread on Linux. A Wayland capture goes through
                // the ScreenCast portal, and the first one waits on its consent
                // dialog — for as long as the user takes to answer. Taken here,
                // that froze the UI thread for up to the portal timeout.
                #[cfg(target_os = "linux")]
                {
                    if self.freeze_capture.is_some() {
                        return;
                    }
                    let (tx, rx) = crossbeam_channel::bounded(1);
                    let spawned = std::thread::Builder::new()
                        .name("freeze-capture".into())
                        .spawn(move || {
                            let _ = tx.send(capture::capture_at(x, y));
                            crate::pump::wake();
                        });
                    match spawned {
                        Ok(_) => self.freeze_capture = Some(rx),
                        Err(e) => self.freeze_captured(Err(e.into())),
                    }
                }
                #[cfg(not(target_os = "linux"))]
                self.freeze_captured(capture::capture_at(x, y));
            }
            MouseTriggerEvent::Move { x, y } => {
                if let Some(sel) = self.active.as_mut() {
                    sel.end = (x, y);
                    self.overlay.update_sel(sel.start.0, sel.start.1, x, y);
                }
            }
            MouseTriggerEvent::Release { x, y } => {
                if let Some(mut sel) = self.active.take() {
                    sel.end = (x, y);
                    // Overlay handoff: when the result is a floating snip, the
                    // overlay (with the selection border) stays up until the
                    // floating window is actually shown — `tick` hides it right
                    // after processing `pending_open`. Hiding it here opened a
                    // visible gap (crop + PNG write + SQLite + one tick) where
                    // the selection blinked out before the snip appeared.
                    // Every non-floating / error path below hides explicitly.
                    //
                    // A toolbar/tray/hotkey snip hid the main window. Whether it
                    // comes back depends on what the snip leaves behind: a
                    // floating snip IS the result, and throwing the gallery up
                    // over it would bury what the user just captured. So the
                    // window stays hidden on that path and the snip owns the
                    // screen.
                    //
                    // Every failure path below re-checks `restore_main` and
                    // restores explicitly — a snip that produced nothing must
                    // never strand the user with no window at all.
                    let restore_main = std::mem::take(&mut self.restore_main_after_snip);
                    if restore_main && !sel.keep_floating {
                        self.pending_show_main = true;
                    }
                    // The armed snip has now started/finished; drop the watchdog
                    // timestamp so it can't fire a spurious recovery later.
                    self.armed_snip_at = None;
                    // Mask the Win key (Ctrl tap) so a lone Win press can't
                    // open the Start menu — but keep any modifier the user
                    // is still physically holding. That's what lets you hold
                    // Win and drag several times to take snips back-to-back.
                    crate::mouse_trigger::force_release_modifiers();
                    let (sx, sy) = sel.start;
                    let (ex, ey) = sel.end;
                    let region = ScreenRect {
                        x: sx.min(ex),
                        y: sy.min(ey),
                        w: (sx - ex).unsigned_abs(),
                        h: (sy - ey).unsigned_abs(),
                    };
                    log::info!("Release: region={:?}", region);
                    if region.w < 4 || region.h < 4 {
                        log::info!("Region too small, ignoring");
                        self.overlay.hide();
                        // Nothing was captured, so nothing will be left on
                        // screen — bring the window back even on the floating
                        // path we skipped it for above.
                        if restore_main {
                            self.pending_show_main = true;
                        }
                        return;
                    }
                    let monitor_label = sel.captured.monitor_label.clone();
                    match capture::crop(&sel.captured, region) {
                        Ok(image) => {
                            // Clipboard before disk. On the clipboard-only
                            // routing it is the whole result, and on every
                            // other one it has no reason to be contingent on
                            // the PNG write having worked first.
                            if sel.to_clipboard {
                                if let Err(e) = clipboard::copy_image(&image) {
                                    self.last_error = Some(t1("Failed to copy to clipboard: {e}", "{e}", &e.to_string()));
                                }
                            }
                            if !sel.to_gallery {
                                // Stop before anything is written down: no PNG
                                // in the cache directory, no row in SQLite, no
                                // OCR text in the search index. A user who
                                // reaches for "clipboard only" to grab a
                                // password is relying on exactly that, and
                                // until now got the opposite — the snip was
                                // saved and indexed like any other.
                                //
                                // `restore_main` was already honoured above:
                                // this routing never floats, so the branch
                                // there has run.
                                self.overlay.hide();
                                return;
                            }
                            let mut snip = make_snip(region);
                            snip.monitor_label = Some(monitor_label);
                            if let Err(e) = snip.save_image(&image) {
                                self.overlay.hide();
                                self.last_error = Some(t1("Failed to save image: {e}", "{e}", &e.to_string()));
                                // The floating window can't open without the
                                // PNG, so the screen would be left empty with
                                // the error nowhere to be seen.
                                if restore_main {
                                    self.pending_show_main = true;
                                }
                                return;
                            }
                            if sel.keep_floating {
                                snip.floating = true;
                            }
                            let snip_id = snip.id;
                            let keep_float = sel.keep_floating;
                            self.gallery.push_new(snip);
                            // Surface a persistence failure instead of dropping
                            // it. The busy_timeout in `open_db` makes a
                            // SQLITE_BUSY loss rare now, but if the row still
                            // fails to commit the PNG is orphaned and the snip
                            // vanishes on next launch — tell the user rather than
                            // lose it silently.
                            if let Err(e) = self.save_gallery() {
                                self.last_error =
                                    Some(t1("Failed to save snip: {e}", "{e}", &e.to_string()));
                            }
                            if let Some(window_info) = sel.source_window.take() {
                                let window_info = crate::window_metadata::with_selection_confidence(
                                    window_info,
                                    region,
                                );
                                if let Err(err) =
                                    crate::snip::save_window_info(snip_id, &window_info)
                                {
                                    log::debug!("source window metadata save skipped: {err}");
                                }
                            }
                            spawn_ocr_index(snip_id, image.clone());
                            log::info!(
                                "Snip pushed to gallery, total={}, keep_float={}",
                                self.gallery.snips.len(),
                                keep_float
                            );
                            if keep_float {
                                // Floating result: the overlay stays visible
                                // until tick opens this window (seamless
                                // handoff — see the comment at Release above).
                                self.pending_open.push(snip_id);
                            } else {
                                self.overlay.hide();
                            }
                            self.last_snip_ids.clear();
                        }
                        Err(e) => {
                            self.overlay.hide();
                            log::warn!("crop failed: {e:#}");
                            self.last_error = Some(t1("Failed to crop region: {e}", "{e}", &e.to_string()));
                            // No snip was produced, so nothing is left on
                            // screen — the window has to come back or the
                            // error has nowhere to appear.
                            if restore_main {
                                self.pending_show_main = true;
                            }
                        }
                    }
                }
            }
            MouseTriggerEvent::ColorPick { x, y } => {
                // The eyedropper click landed: grab the pixel, copy the HEX, show
                // the result popup, and (only if we hid it) bring the main window
                // back. A context-menu-triggered pick leaves the window as-is.
                self.color_picker.finish(x, y);
                self.color_pick_started_at = None;
                if std::mem::take(&mut self.color_pick_hid_main) {
                    self.pending_show_main = true;
                }
            }
            MouseTriggerEvent::Cancel => {
                // Esc during colour-pick: dismiss the loupe and restore the window.
                if self.color_picker.is_active() {
                    self.color_picker.cancel();
                    self.color_pick_started_at = None;
                    if std::mem::take(&mut self.color_pick_hid_main) {
                        self.pending_show_main = true;
                    }
                    return;
                }
                self.active = None;
                self.frozen_pending = None;
                // Esc between "hide the window" and "freeze the screen" must
                // drop the pending capture, or the overlay springs up moments
                // after the user cancelled.
                self.pending_freeze_arm = None;
                // Drop any capture routing armed by a toolbar/tray/hotkey
                // trigger, plus its watchdog timestamp; otherwise a cancelled
                // arm's stale routing (e.g. clipboard-only) would mis-route a
                // later, unrelated Win+drag.
                self.armed_routing = None;
                self.armed_snip_at = None;
                crate::mouse_trigger::clear_freeze();
                self.overlay.hide();
                crate::mouse_trigger::force_release_modifiers();
                // A toolbar-button snip hid the main window; the user cancelled
                // (Esc), so bring it back rather than leaving it hidden.
                if self.restore_main_after_snip {
                    self.restore_main_after_snip = false;
                    self.pending_show_main = true;
                }
            }
        }
    }
}

/// The modifier matrix, as a function of the modifiers alone.
///
/// Returns `(to_clipboard, keep_floating, to_gallery)`:
///
/// | Held                  | Result                              |
/// |-----------------------|-------------------------------------|
/// | (none)                | snip + floating window              |
/// | `Ctrl`                | snip + floating + clipboard         |
/// | `Alt`                 | clipboard only — nothing persisted  |
/// | `Shift`               | gallery only, no floating           |
/// | `Ctrl`+`Shift`        | gallery + clipboard, no floating    |
///
/// Split out of the event handler so it can be tested. The bug that motivated
/// that: `Alt` and `Ctrl`+`Shift` returned the identical pair when the routing
/// was two booleans, so "clipboard only" quietly saved and OCR-indexed the
/// snip like any other. Buried in a match arm inside a 600-line handler, there
/// was nothing that could have caught it.
fn routing_for_modifiers(mods: crate::mouse_trigger::ModifierSet) -> (bool, bool, bool) {
    if mods.alt {
        (true, false, false)
    } else if mods.ctrl && mods.shift {
        (true, false, true)
    } else if mods.shift {
        (false, false, true)
    } else if mods.ctrl {
        (true, true, true)
    } else {
        (false, true, true)
    }
}

#[cfg(test)]
mod tests {
    use super::routing_for_modifiers;
    use crate::mouse_trigger::ModifierSet;

    fn mods(ctrl: bool, alt: bool, shift: bool) -> ModifierSet {
        ModifierSet { ctrl, alt, shift }
    }

    #[test]
    fn alt_is_the_only_routing_that_keeps_nothing() {
        let (clip, float, gallery) = routing_for_modifiers(mods(false, true, false));
        assert!(clip, "the clipboard is the entire point");
        assert!(!float);
        assert!(
            !gallery,
            "\"clipboard only\" has to mean no PNG, no row, no OCR index"
        );
    }

    /// The regression itself: these two rows of the matrix are different
    /// routings and must not collapse onto the same answer again.
    #[test]
    fn alt_and_ctrl_shift_are_distinguishable() {
        let alt = routing_for_modifiers(mods(false, true, false));
        let ctrl_shift = routing_for_modifiers(mods(true, false, true));
        assert_ne!(alt, ctrl_shift);
        assert!(ctrl_shift.2, "Ctrl+Shift is a gallery routing");
    }

    #[test]
    fn every_routing_other_than_alt_is_persisted() {
        for (c, a, s) in [
            (false, false, false),
            (true, false, false),
            (false, false, true),
            (true, false, true),
        ] {
            let (_, _, gallery) = routing_for_modifiers(mods(c, a, s));
            assert!(gallery, "ctrl={c} alt={a} shift={s} should reach the gallery");
        }
    }

    #[test]
    fn plain_drag_floats_without_touching_the_clipboard() {
        assert_eq!(
            routing_for_modifiers(mods(false, false, false)),
            (false, true, true)
        );
    }
}
