use super::*;
use crate::i18n::t1;

impl AppState {
    pub(crate) fn handle_context_action(
        &mut self,
        id: Uuid,
        action: ContextAction,
        window: &MainWindow,
    ) {
        // Rotating, flipping or cropping a snip that is open in the editor
        // lasted only until the editor saved: it writes back the pixels it was
        // opened with, and the transform was gone. The editor is brought back
        // instead, to be finished or cancelled first.
        let rewrites_pixels = matches!(
            action,
            ContextAction::RotateLeft
                | ContextAction::RotateRight
                | ContextAction::Rotate180
                | ContextAction::FlipHorizontal
                | ContextAction::FlipVertical
                | ContextAction::Crop
        );
        if rewrites_pixels && self.annotate.editing() == Some(id) {
            self.annotate.raise();
            self.last_error = Some(t(
                "Finish or cancel the open annotation before starting another.",
            ));
            return;
        }
        match action {
            ContextAction::Close => {
                self.floating.close(id);
                if let Some(s) = self.gallery.get_mut(id) {
                    s.floating = false;
                }
                let _ = self.save_gallery();
                self.refresh_gallery(window);
            }
            ContextAction::Delete => {
                self.floating.close(id);
                self.gallery.remove(id);
                self.forget_thumb(id);
                let _ = self.save_gallery();
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
                                self.settings.translate_consented,
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
                // Opening an editor replaces the one already open, and that used
                // to throw its unsaved shapes away. Bring it back instead.
                if self.annotate.has_unsaved() {
                    self.annotate.raise();
                    self.last_error = Some(t(
                        "Finish or cancel the open annotation before starting another.",
                    ));
                    return;
                }
                if let Some(snip) = self.gallery.get(id).cloned() {
                    match snip.load_image() {
                        Ok(img) => {
                            if let Err(e) = self.annotate.open(id, img) {
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
            // Same picker as the toolbar, launched from the snip menu — through
            // the same checks, which this path used to skip. The main window is
            // left alone: context menus run on a floating snip.
            ContextAction::PickColor => self.start_color_pick(None),
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
                if let Err(e) = s.save_image(&image) {
                    self.last_error =
                        Some(t1("Failed to save image: {e}", "{e}", &e.to_string()));
                    return;
                }
            }
            None => return,
        }
        let _ = self.save_gallery();
        self.forget_thumb(id);
        if self.floating.is_shown(id) {
            self.floating.close(id);
            if let Some(s) = self.gallery.get(id).cloned() {
                let sf = self.scale_factor;
                let _ = self
                    .floating
                    .open(&s, &self.settings, sf);
            }
        }
        self.refresh_gallery(window);
    }

    /// Open the dynamic collage editor. It carries its own thumbnail picker, so
    /// we just hand it the gallery's snips (most-recent first, capped) and let
    /// the user pick which to combine; currently-floating snips start selected.
    pub(crate) fn open_collage(&mut self, window: &MainWindow) {
        const TRAY_CAP: usize = 80;
        let mut sources: Vec<CollageSource> = Vec::new();
        for s in self.gallery.snips.iter().take(TRAY_CAP) {
            match s.ensure_thumbnail() {
                Ok(thumb) => sources.push(CollageSource {
                    id: s.id,
                    thumb,
                    path: s.image_path(),
                }),
                Err(e) => log::warn!("collage: skipping {} (thumbnail: {e})", s.id),
            }
        }
        if sources.len() < 2 {
            self.last_error = Some(t("Add at least two snips to make a collage."));
            return;
        }
        let preselect: Vec<Uuid> = self
            .gallery
            .snips
            .iter()
            .filter(|s| self.floating.is_shown(s.id))
            .map(|s| s.id)
            .collect();
        if let Err(e) = self.collage.open(sources, preselect, self.scale_factor) {
            self.last_error = Some(t1("Failed to open collage: {e}", "{e}", &e.to_string()));
        }
        let _ = window; // the editor is its own window — nothing to refresh now
    }

    /// Commit a finished collage as a brand-new snip (image + thumbnail + DB),
    /// then refresh the gallery so it appears at the top.
    pub(crate) fn apply_collage(&mut self, image: image::RgbaImage, window: &MainWindow) {
        let (w, h) = image.dimensions();
        if w == 0 || h == 0 {
            return;
        }
        let mut snip = make_snip(ScreenRect::new(0, 0, w, h));
        snip.monitor_label = Some(t("Collage"));
        if let Err(e) = snip.save_image(&image) {
            self.last_error = Some(t1("Failed to save image: {e}", "{e}", &e.to_string()));
            return;
        }
        self.gallery.push_new(snip);
        let _ = self.save_gallery();
        self.last_snip_ids = self.gallery.snips.iter().map(|s| s.id).collect();
        self.refresh_gallery(window);
    }

    /// Open the About window — app info + a brand-level ReviseTouch funnel.
    /// The ReviseTouch link (where the app is published) and the GitHub link are
    /// public; the optional Support button's URL is injected at build time via
    /// the `SNIPDECK_DONATE_URL` env var, so the donation link ships in the
    /// binary ONLY and never appears in the public source.
    pub(crate) fn show_about(&mut self, _window: &MainWindow) {
        use crate::share::open_url;

        let w = match crate::AboutWindow::new() {
            Ok(w) => w,
            Err(e) => {
                self.last_error = Some(t1("Failed to open window: {e}", "{e}", &e.to_string()));
                return;
            }
        };
        w.set_version(SharedString::from(env!("CARGO_PKG_VERSION")));
        let donate = option_env!("SNIPDECK_DONATE_URL");
        w.set_has_donate(donate.is_some());
        w.on_open_revisetouch(|| open_url("https://revisetouch.com"));
        w.on_open_github(|| open_url("https://github.com/ReviseTouch/snipdeck"));
        if let Some(url) = donate {
            w.on_open_donate(move || open_url(url));
        }
        let weak = w.as_weak();
        w.on_close(move || {
            if let Some(c) = weak.upgrade() {
                let _ = c.hide();
            }
        });
        w.window()
            .on_close_requested(|| slint::CloseRequestResponse::HideWindow);
        w.window().set_size(slint::LogicalSize::new(480.0, 360.0));
        let _ = w.show();
        crate::window_env::set_frame_theme(&w, true);
        self.about = Some(w);
    }

    /// Make sure `id` has an open floating window. Used by gallery-context-menu
    /// actions (Crop / Resize) that operate on the on-screen window — a no-op
    /// when the snip is already floating.
    fn ensure_floating(&mut self, id: Uuid, window: &MainWindow) {
        if self.floating.is_shown(id) {
            return;
        }
        let Some(snip) = self.gallery.get(id).cloned() else {
            return;
        };
        let sf = self.scale_factor;
        if let Err(e) = self
            .floating
            .open(&snip, &self.settings, sf)
        {
            self.last_error =
                Some(t1("Failed to open floating window: {e}", "{e}", &e.to_string()));
            return;
        }
        if let Some(s) = self.gallery.get_mut(id) {
            s.floating = true;
        }
        let _ = self.save_gallery();
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

    /// Upload a snip to the chosen host (⋯ → Upload to). Opens the upload
    /// window, which asks first; nothing leaves the machine before Upload.
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
        let host = crate::upload::Host::from_settings(&self.settings);
        if let Err(e) = self.upload_ui.start(img, host, self.settings.dark_theme) {
            self.last_error = Some(t1("Failed to open window: {e}", "{e}", &e.to_string()));
        }
    }

    /// Fill ⋯ → Upload to: catbox, Imgur, then the imported hosts, with the
    /// current one ticked.
    pub(crate) fn refresh_upload_hosts(&self, window: &MainWindow) {
        let mut names: Vec<slint::SharedString> = vec!["catbox.moe".into(), "Imgur".into()];
        names.extend(self.settings.upload_hosts.iter().map(|u| u.display_name().into()));
        let service = self.settings.upload_service.trim();
        let current = if service.eq_ignore_ascii_case("imgur") {
            1
        } else if let Some(name) = service.strip_prefix(crate::upload::CUSTOM_PREFIX) {
            self.settings
                .upload_hosts
                .iter()
                .position(|u| u.display_name() == name)
                .map_or(0, |i| i as i32 + 2)
        } else {
            0
        };
        window.set_upload_hosts(slint::ModelRc::new(slint::VecModel::from(names)));
        window.set_upload_host_index(current);
    }

    pub(crate) fn pick_upload_host(&mut self, index: i32, window: &MainWindow) {
        self.settings.upload_service = match index {
            0 => "catbox".into(),
            1 => "imgur".into(),
            i => match self.settings.upload_hosts.get((i - 2).max(0) as usize) {
                Some(up) => format!("{}{}", crate::upload::CUSTOM_PREFIX, up.display_name()),
                None => return,
            },
        };
        let _ = self.settings.save();
        self.refresh_upload_hosts(window);
    }

    /// Remove the ticked host, if it is an imported one, and fall back to
    /// catbox.
    pub(crate) fn remove_upload_host(&mut self, window: &MainWindow) {
        let Some(name) = self
            .settings
            .upload_service
            .trim()
            .strip_prefix(crate::upload::CUSTOM_PREFIX)
            .map(str::to_string)
        else {
            return;
        };
        self.settings.upload_hosts.retain(|u| u.display_name() != name);
        self.settings.upload_service = "catbox".into();
        let _ = self.settings.save();
        self.refresh_upload_hosts(window);
    }

    /// Pick a `.sxcu` file and import it. The file is read and checked on a
    /// worker — the dialog blocks — and joins the list in
    /// `finish_upload_host_import`.
    pub(crate) fn add_upload_host(&mut self) {
        let tx = self.sxcu_imports.0.clone();
        let read = move |path: std::path::PathBuf| {
            let result = std::fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|text| crate::sxcu::Uploader::parse(&text).map_err(|e| e.to_string()));
            let _ = tx.send(result);
            crate::pump::wake();
        };
        let filter = ["sxcu", "json"];
        // Windows: the native dialog keeps this thread's windows responsive.
        #[cfg(target_os = "windows")]
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("ShareX custom uploader", &filter)
            .pick_file()
        {
            read(path);
        }
        // Elsewhere it is the desktop portal, waited on off the UI thread
        // (see `save_snip`).
        #[cfg(not(target_os = "windows"))]
        {
            let dialog = rfd::AsyncFileDialog::new().add_filter("ShareX custom uploader", &filter);
            std::thread::spawn(move || {
                if let Some(picked) = pollster::block_on(dialog.pick_file()) {
                    read(picked.path().to_path_buf());
                }
            });
        }
    }

    /// An imported host joins the list and becomes the one used; a host of
    /// the same name is replaced. A refused file says why in the banner.
    pub(crate) fn finish_upload_host_import(
        &mut self,
        import: Result<crate::sxcu::Uploader, String>,
        window: &MainWindow,
    ) {
        match import {
            Ok(up) => {
                let name = up.display_name();
                log::info!("imported upload host {name:?} ({})", up.domain());
                self.settings.upload_hosts.retain(|u| u.display_name() != name);
                self.settings.upload_hosts.push(up);
                self.settings.upload_service = format!("{}{name}", crate::upload::CUSTOM_PREFIX);
                let _ = self.settings.save();
                self.refresh_upload_hosts(window);
            }
            Err(why) => {
                self.last_error = Some(t1("Could not add the upload host: {why}", "{why}", &why));
            }
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
            if let Err(e) = s.save_image(&out) {
                self.last_error = Some(t1("Failed to save image: {e}", "{e}", &e.to_string()));
                return;
            }
            s.area.w = ow;
            s.area.h = oh;
        }
        let _ = self.save_gallery();
        // Force the gallery card + any floating window to pick up the new
        // pixels (the cached decoded thumbnail is now stale).
        self.forget_thumb(id);
        if self.floating.is_shown(id) {
            self.floating.close(id);
            if let Some(s) = self.gallery.get(id).cloned() {
                let sf = self.scale_factor;
                let _ = self
                    .floating
                    .open(&s, &self.settings, sf);
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
            if let Err(e) = s.save_image(&cropped) {
                self.last_error = Some(t1("Failed to save image: {e}", "{e}", &e.to_string()));
                return;
            }
            s.area.w = cw;
            s.area.h = ch;
            s.remember_position(new_pos);
        }
        let _ = self.save_gallery();
        self.forget_thumb(id);
        self.floating.close(id);
        if let Some(s) = self.gallery.get(id).cloned() {
            let sf = self.scale_factor;
            let _ = self
                .floating
                .open(&s, &self.settings, sf);
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
            s.remember_position((wx, wy));
        }
        let _ = self.save_gallery();
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
        let filter: &[&str] = &["png", "jpg", "jpeg", "bmp", "gif", "tif", "tiff", "webp"];
        let default_format = self.settings.default_format.clone();
        // A name typed without an extension gets the default format rather
        // than an "unsupported image type" error.
        let finish = move |mut path: std::path::PathBuf| -> anyhow::Result<()> {
            if path.extension().is_none() {
                path.set_extension(&default_format);
            }
            crate::image_files::export(&path, &snip.load_image()?)
        };

        // Windows: the native dialog runs a modal loop that keeps every window
        // of this thread responsive, so it can run right here.
        #[cfg(target_os = "windows")]
        {
            let dialog = rfd::FileDialog::new()
                .set_directory(&self.settings.save_dir)
                .set_file_name(default_name)
                .add_filter("Image", filter)
                .save_file();
            if let Some(path) = dialog {
                if let Err(e) = finish(path) {
                    self.last_error = Some(t1("Failed to save: {e}", "{e}", &format!("{e:#}")));
                }
            }
        }
        // Elsewhere the dialog lives in another process (the desktop portal)
        // and rfd waits for it by blocking the calling thread. On the UI thread
        // that froze every Snipdeck window while the user browsed, and after a
        // few seconds GNOME offered to force-quit the app. It runs on a worker
        // instead; a failure comes back as a banner through `background_errors`.
        #[cfg(not(target_os = "windows"))]
        {
            let dialog = rfd::AsyncFileDialog::new()
                .set_directory(&self.settings.save_dir)
                .set_file_name(default_name)
                .add_filter("Image", filter);
            let errors = self.background_errors.0.clone();
            std::thread::spawn(move || {
                let Some(picked) = pollster::block_on(dialog.save_file()) else {
                    return;
                };
                if let Err(e) = finish(picked.path().to_path_buf()) {
                    let _ = errors.send(t1("Failed to save: {e}", "{e}", &format!("{e:#}")));
                    crate::pump::wake();
                }
            });
        }
    }
}
