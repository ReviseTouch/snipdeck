use super::*;
use crate::i18n::t1;

impl AppState {
    pub(crate) fn handle_context_action(
        &mut self,
        id: Uuid,
        action: ContextAction,
        window: &MainWindow,
    ) {
        match action {
            ContextAction::Close => {
                self.floating.close(id);
                if let Some(s) = self.gallery.get_mut(id) {
                    s.floating = false;
                }
                let _ = self.gallery.save();
                self.refresh_gallery(window);
            }
            ContextAction::Delete => {
                self.floating.close(id);
                self.gallery.remove(id);
                self.thumb_cache.remove(&id);
                let _ = self.gallery.save();
                self.refresh_gallery(window);
            }
            ContextAction::Copy => {
                if let Some(snip) = self.gallery.get(id) {
                    match snip.load_image() {
                        Ok(img) => {
                            if let Err(e) = clipboard::copy_image(&img) {
                                self.last_error = Some(t1("Failed to copy to clipboard: {e}", "{e}", &e.to_string()));
                            }
                        }
                        Err(e) => {
                            self.last_error = Some(t1("Failed to read image: {e}", "{e}", &e.to_string()));
                        }
                    }
                }
            }
            ContextAction::ResizeAgain => {
                self.ensure_floating(id, window);
                self.floating.enable_resize_handles(id);
            }
            ContextAction::CopyOcr => {
                if let Some(snip) = self.gallery.get(id).cloned() {
                    match crate::snip::cached_ocr_text(id) {
                        Ok(Some(text)) => {
                            if let Err(e) = clipboard::copy_text(&text) {
                                self.last_error = Some(t1("Failed to copy text to clipboard: {e}", "{e}", &e.to_string()));
                            }
                        }
                        _ => {
                            spawn_ocr_copy(id, snip);
                        }
                    }
                }
            }
            ContextAction::RotateLeft => {
                self.transform_snip(id, window, |img| image::imageops::rotate270(img));
            }
            ContextAction::RotateRight => {
                self.transform_snip(id, window, |img| image::imageops::rotate90(img));
            }
            ContextAction::Rotate180 => {
                self.transform_snip(id, window, |img| image::imageops::rotate180(img));
            }
            ContextAction::FlipHorizontal => {
                self.transform_snip(id, window, |img| image::imageops::flip_horizontal(img));
            }
            ContextAction::FlipVertical => {
                self.transform_snip(id, window, |img| image::imageops::flip_vertical(img));
            }
            ContextAction::Crop => {
                // Crop operates on the image's real pixels, so seed crop mode
                // with the actual decoded dimensions (which differ from
                // area.w/h if the snip was stretched via the resize handles).
                if let Some(snip) = self.gallery.get(id).cloned() {
                    match snip.load_image() {
                        Ok(img) => {
                            // May be opened straight from a gallery-card right-
                            // click, so ensure there's a floating window to crop.
                            self.ensure_floating(id, window);
                            self.floating.enable_crop(id, img.width(), img.height());
                        }
                        Err(e) => self.last_error = Some(t1("Failed to read image: {e}", "{e}", &e.to_string())),
                    }
                }
            }
            ContextAction::OcrTranslate => {
                if let Some(snip) = self.gallery.get(id).cloned() {
                    match snip.load_image() {
                        Ok(img) => {
                            // Anchor the popup on the snip's floating window (or
                            // the main window if it isn't floating). The popup
                            // picks right/left placement and clamps on-screen.
                            let anchor = match self.floating.handle(id) {
                                Some(h) => h.physical_rect(),
                                None => {
                                    let p = window.window().position();
                                    (p.x, p.y, 0, 0)
                                }
                            };
                            let target = self.settings.translate_target.clone();
                            let favs = self.settings.translate_favs.clone();
                            if let Err(e) = self.translate_popup.open(
                                id,
                                img,
                                &target,
                                &favs,
                                anchor,
                                self.scale_factor,
                            ) {
                                self.last_error = Some(t1("Failed to open translation window: {e}", "{e}", &e.to_string()));
                            }
                        }
                        Err(e) => self.last_error = Some(t1("Failed to read image: {e}", "{e}", &e.to_string())),
                    }
                }
            }
            ContextAction::ShareSystem => self.share_snip(id, ShareTarget::System, window),
            ContextAction::ShareMail => self.share_snip(id, ShareTarget::Mail, window),
            ContextAction::OpenEditor => self.open_in_editor(id),
            ContextAction::Upload => self.upload_snip(id),
            ContextAction::ShowApp => {
                let _ = window.show();
                crate::tray::focus_main_window();
            }
            ContextAction::Annotate => {
                if let Some(snip) = self.gallery.get(id).cloned() {
                    match snip.load_image() {
                        Ok(img) => {
                            if let Err(e) = self.annotate.open(id, img, self.scale_factor) {
                                self.last_error = Some(t1(
                                    "Failed to open annotation editor: {e}",
                                    "{e}",
                                    &e.to_string(),
                                ));
                            }
                        }
                        Err(e) => {
                            self.last_error =
                                Some(t1("Failed to read image: {e}", "{e}", &e.to_string()))
                        }
                    }
                }
            }
        }
    }

    /// Commit a finished annotation: bake the new pixels onto the snip (image +
    /// thumbnail), drop the cached thumbnail, and reopen its floating window (if
    /// any) at the new image. Same shape as `apply_crop` minus the size/position
    /// change — annotation keeps the snip's dimensions.
    pub(crate) fn apply_annotation(
        &mut self,
        id: Uuid,
        image: image::RgbaImage,
        window: &MainWindow,
    ) {
        // Annotation never changes the pixel dimensions, so `area.w/h` (which
        // may legitimately differ from the native size for a resize-stretched
        // snip) is left untouched — only the pixels are rewritten.
        match self.gallery.get_mut(id) {
            Some(s) => {
                if let Err(e) = s.save_image_png(&image) {
                    self.last_error =
                        Some(t1("Failed to save image: {e}", "{e}", &e.to_string()));
                    return;
                }
            }
            None => return,
        }
        let _ = self.gallery.save();
        self.thumb_cache.remove(&id);
        if self.floating.windows.contains_key(&id) {
            self.floating.close(id);
            if let Some(s) = self.gallery.get(id).cloned() {
                let sf = self.scale_factor;
                let _ = self
                    .floating
                    .open(&s, &self.settings, sf, Some(window.window()));
            }
        }
        self.refresh_gallery(window);
    }

    /// Make sure `id` has an open floating window. Used by gallery-context-menu
    /// actions (Crop / Resize) that operate on the on-screen window — a no-op
    /// when the snip is already floating.
    fn ensure_floating(&mut self, id: Uuid, window: &MainWindow) {
        if self.floating.windows.contains_key(&id) {
            return;
        }
        let Some(snip) = self.gallery.get(id).cloned() else {
            return;
        };
        let sf = self.scale_factor;
        if let Err(e) = self
            .floating
            .open(&snip, &self.settings, sf, Some(window.window()))
        {
            self.last_error =
                Some(t1("Failed to open floating window: {e}", "{e}", &e.to_string()));
            return;
        }
        if let Some(s) = self.gallery.get_mut(id) {
            s.floating = true;
        }
        let _ = self.gallery.save();
        self.refresh_gallery(window);
    }

    /// Open a snip in the system image editor (Paint) for a quick edit.
    fn open_in_editor(&mut self, id: Uuid) {
        let Some(snip) = self.gallery.get(id) else {
            return;
        };
        let img = match snip.load_image() {
            Ok(i) => i,
            Err(e) => {
                self.last_error = Some(t1("Failed to read image: {e}", "{e}", &e.to_string()));
                return;
            }
        };
        if let Err(e) = crate::share::open_in_editor(&img) {
            self.last_error = Some(t1("Failed to open editor: {e}", "{e}", &e.to_string()));
        }
    }

    /// Upload a snip to the configured image host (catbox.moe by default, or
    /// Imgur) and copy the resulting link to the clipboard. Imgur needs a
    /// Client-ID in settings; if it's missing, point the user at how to get one
    /// rather than failing silently. The blocking HTTP upload runs on a worker
    /// thread (see `crate::upload`), which also copies the link and shows a
    /// notification when it finishes.
    fn upload_snip(&mut self, id: Uuid) {
        let Some(snip) = self.gallery.get(id) else {
            return;
        };
        let img = match snip.load_image() {
            Ok(i) => i,
            Err(e) => {
                self.last_error = Some(t1("Failed to read image: {e}", "{e}", &e.to_string()));
                return;
            }
        };
        // A missing Client-ID is surfaced by `upload` with a visible (topmost)
        // notice, not just the gallery error banner — the action is usually
        // fired from a floating snip's menu while the gallery is hidden.
        if let Err(e) = crate::upload::upload(
            &img,
            &self.settings.upload_service,
            &self.settings.imgur_client_id,
        ) {
            self.last_error = Some(t1("Failed to start upload: {e}", "{e}", &e.to_string()));
        }
    }

    /// Load a snip's image and hand it to a share target. The system share
    /// sheet attaches the image directly; mail attaches it via Simple MAPI.
    fn share_snip(&mut self, id: Uuid, target: ShareTarget, window: &MainWindow) {
        let Some(snip) = self.gallery.get(id) else {
            return;
        };
        let img = match snip.load_image() {
            Ok(i) => i,
            Err(e) => {
                self.last_error = Some(t1("Failed to read image: {e}", "{e}", &e.to_string()));
                return;
            }
        };
        if matches!(target, ShareTarget::System) {
            // The Win11 share flyout needs a normal, visible, foreground host
            // window — the gallery. Surface it so the sheet actually appears
            // (anchoring on a hidden window or a topmost tool window swallows it).
            let _ = window.show();
            crate::tray::focus_main_window();
        }
        let r = match target {
            ShareTarget::System => crate::share::share_system(&img),
            ShareTarget::Mail => crate::share::share_mail(&img),
        };
        if let Err(e) = r {
            self.last_error = Some(t1("Share failed: {e}", "{e}", &e.to_string()));
        }
    }

    /// Apply an in-place image transform (rotate / flip) to a snip: rewrite
    /// the full image + thumbnail on disk, update its stored dimensions (90°
    /// rotations swap width/height), drop the cached thumbnail, reopen its
    /// floating window at the new aspect, and refresh the gallery.
    fn transform_snip(
        &mut self,
        id: Uuid,
        window: &MainWindow,
        op: impl Fn(&image::RgbaImage) -> image::RgbaImage,
    ) {
        let Some(snip) = self.gallery.get(id) else {
            return;
        };
        let img = match snip.load_image() {
            Ok(i) => i,
            Err(e) => {
                self.last_error = Some(t1("Failed to read image: {e}", "{e}", &e.to_string()));
                return;
            }
        };
        let out = op(&img);
        let (ow, oh) = (out.width(), out.height());
        if let Some(s) = self.gallery.get_mut(id) {
            if let Err(e) = s.save_image_png(&out) {
                self.last_error = Some(t1("Failed to save image: {e}", "{e}", &e.to_string()));
                return;
            }
            s.area.w = ow;
            s.area.h = oh;
        }
        let _ = self.gallery.save();
        // Force the gallery card + any floating window to pick up the new
        // pixels (the cached decoded thumbnail is now stale).
        self.thumb_cache.remove(&id);
        if self.floating.windows.contains_key(&id) {
            self.floating.close(id);
            if let Some(s) = self.gallery.get(id).cloned() {
                let sf = self.scale_factor;
                let _ = self
                    .floating
                    .open(&s, &self.settings, sf, Some(window.window()));
            }
        }
        self.refresh_gallery(window);
    }

    /// Commit a crop chosen with the inward-only handles: crop the image to
    /// the clip rect (in image pixels), rewrite image + thumbnail, update the
    /// stored size + position, and reopen the floating window at the cropped
    /// size (which also exits crop mode).
    #[allow(clippy::too_many_arguments)]
    fn apply_crop(
        &mut self,
        id: Uuid,
        cx: u32,
        cy: u32,
        cw: u32,
        ch: u32,
        new_pos: (i32, i32),
        window: &MainWindow,
    ) {
        let Some(snip) = self.gallery.get(id) else {
            return;
        };
        let img = match snip.load_image() {
            Ok(i) => i,
            Err(e) => {
                self.last_error = Some(t1("Failed to read image: {e}", "{e}", &e.to_string()));
                return;
            }
        };
        let (iw, ih) = (img.width(), img.height());
        let cx = cx.min(iw.saturating_sub(1));
        let cy = cy.min(ih.saturating_sub(1));
        let cw = cw.min(iw - cx).max(1);
        let ch = ch.min(ih - cy).max(1);
        let cropped = image::imageops::crop_imm(&img, cx, cy, cw, ch).to_image();
        if let Some(s) = self.gallery.get_mut(id) {
            if let Err(e) = s.save_image_png(&cropped) {
                self.last_error = Some(t1("Failed to save image: {e}", "{e}", &e.to_string()));
                return;
            }
            s.area.w = cw;
            s.area.h = ch;
            s.last_position = Some(new_pos);
        }
        let _ = self.gallery.save();
        self.thumb_cache.remove(&id);
        self.floating.close(id);
        if let Some(s) = self.gallery.get(id).cloned() {
            let sf = self.scale_factor;
            let _ = self
                .floating
                .open(&s, &self.settings, sf, Some(window.window()));
        }
        self.refresh_gallery(window);
    }

    /// Persist the new outer size after a resize-handle gesture. The
    /// snip image itself is unchanged — image-fit: fill in the Slint
    /// view stretches it to whatever the user dragged to (so the
    /// handles act as a zoom, not a re-capture).
    pub(crate) fn handle_resize_end(&mut self, id: Uuid, window: &MainWindow) {
        let Some(handle) = self.floating.handle(id) else {
            return;
        };
        // In crop mode the same end-gesture commits a real pixel crop rather
        // than persisting a stretched resize.
        if handle.window.get_cropping() {
            let cx = handle.window.get_clip_x().max(0) as u32;
            let cy = handle.window.get_clip_y().max(0) as u32;
            let cw = handle.window.get_clip_w().max(1) as u32;
            let ch = handle.window.get_clip_h().max(1) as u32;
            let (wx, wy, _, _) = handle.physical_rect();
            self.apply_crop(id, cx, cy, cw, ch, (wx, wy), window);
            return;
        }
        let (wx, wy, wp, hp) = handle.physical_rect();
        let border: i32 = 3;
        let new_w = (wp - border * 2).max(1) as u32;
        let new_h = (hp - border * 2).max(1) as u32;
        if let Some(s) = self.gallery.get_mut(id) {
            s.area.w = new_w;
            s.area.h = new_h;
            s.last_position = Some((wx, wy));
        }
        let _ = self.gallery.save();
        self.refresh_gallery(window);
    }

    pub(crate) fn save_snip_to_file(&mut self, id: Uuid) {
        let Some(snip) = self.gallery.get(id).cloned() else {
            return;
        };
        let default_name = format!(
            "{} ({}x{}).{}",
            snip.created_at.format("%Y-%m-%d_%H-%M-%S"),
            snip.area.w,
            snip.area.h,
            self.settings.default_format
        );
        let dialog = rfd::FileDialog::new()
            .set_directory(&self.settings.save_dir)
            .set_file_name(default_name)
            .add_filter(
                "Image",
                &["png", "jpg", "jpeg", "bmp", "gif", "tif", "tiff"],
            )
            .save_file();
        if let Some(path) = dialog {
            let _ = std::fs::create_dir_all(path.parent().unwrap_or(std::path::Path::new(".")));
            if let Ok(img) = snip.load_image() {
                if let Err(e) = img.save(&path) {
                    self.last_error = Some(t1("Failed to save: {e}", "{e}", &e.to_string()));
                }
            }
        }
    }
}
