use super::*;

impl AppState {
    pub fn refresh_gallery(&mut self, window: &MainWindow) {
        let search = self.search.trim().to_string();
        let search_lower = search.to_lowercase();
        // Clone/query matches up-front so we can mutably borrow `self`
        // for thumbnail caching while iterating. Non-empty search goes
        // through SQLite so OCR text is searchable without decoding images.
        let mut items: Vec<crate::snip::Snip> = if search.is_empty() {
            self.gallery.snips.clone()
        } else {
            self.gallery.search(&search).unwrap_or_else(|err| {
                log::warn!("SQLite gallery search failed, using in-memory fallback: {err}");
                self.gallery
                    .snips
                    .iter()
                    .filter(|s| {
                        let label_ok = s
                            .label
                            .as_deref()
                            .map(|l| l.to_lowercase().contains(&search_lower))
                            .unwrap_or(false);
                        let dim = format!("{}x{}", s.area.w, s.area.h);
                        let monitor_ok = s
                            .monitor_label
                            .as_deref()
                            .map(|m| m.to_lowercase().contains(&search_lower))
                            .unwrap_or(false);
                        label_ok || monitor_ok || dim.contains(&search_lower)
                    })
                    .cloned()
                    .collect()
            })
        };
        let metadata = crate::snip::metadata_index().unwrap_or_else(|err| {
            log::debug!("snip metadata index unavailable for filters: {err}");
            crate::snip::SnipMetadataIndex::default()
        });
        items.retain(|s| self.matches_filters(s, &metadata));
        self.sort_items(&mut items);

        // Build the flat card list once. Thumbnails are NOT decoded here
        // (that would be O(n) per refresh and unbounded memory); each card
        // asks for its thumbnail when it scrolls into view (RequestThumb).
        // We seed from the decoded cache so already-loaded thumbs don't flash.
        self.gallery_cards = items
            .iter()
            .map(|s| {
                let is_floating = self.floating.is_shown(s.id);
                let has_ocr = metadata.ocr_ids.contains(&s.id);
                let has_window = metadata.window_ids.contains(&s.id);
                let (preview_tag, preview) = snip_preview(s, has_ocr, has_window);
                let thumb = self.thumb_cache.get(&s.id).cloned().unwrap_or_default();
                let item = SnipItem {
                    id: SharedString::from(s.id.to_string()),
                    label: SharedString::from(s.label.clone().unwrap_or_default()),
                    preview: SharedString::from(preview),
                    preview_tag: SharedString::from(preview_tag),
                    dims: SharedString::from(format!("{}×{} px", s.area.w, s.area.h)),
                    size_text: SharedString::from(format!("{}×{}", s.area.w, s.area.h)),
                    pinned: s.pinned,
                    floating: is_floating,
                    has_ocr,
                    has_window,
                    monitor: SharedString::from(s.monitor_label.clone().unwrap_or_default()),
                    source: SharedString::from(snip_source(s, has_window)),
                    created: SharedString::from(short_date(s.created_at)),
                    thumb,
                    aspect: s.area.w as f32 / (s.area.h.max(1) as f32),
                };
                (s.id, item)
            })
            .collect();

        self.rechunk_rows(window);
        window.set_gallery_total(self.gallery.snips.len() as i32);
        window.set_pinned_count(self.gallery.snips.iter().filter(|s| s.pinned).count() as i32);
        window.set_floating_count(self.floating.shown_count() as i32);
        window.set_filtered_count(items.len() as i32);
        window.set_storage_size(SharedString::from(storage_size_text(&self.gallery.snips)));
    }

    /// Chunk the already-built `gallery_cards` into rows of `gallery_cols`
    /// and hand them to the UI. Cheap (just clones + model assembly, no
    /// filtering or decoding), so it's safe to call on every column-count
    /// change during a live resize.
    fn rechunk_rows(&mut self, window: &MainWindow) {
        let cols = self.gallery_cols.max(1);
        let n = self.gallery_cards.len();
        self.gallery_rows = Vec::with_capacity(n / cols + 1);
        self.card_index = std::collections::HashMap::with_capacity(n);
        let mut rows: Vec<ModelRc<SnipItem>> = Vec::with_capacity(n / cols + 1);

        for (i, (id, item)) in self.gallery_cards.iter().enumerate() {
            self.card_index.insert(*id, i);
            if i % cols == 0 {
                let inner: Rc<VecModel<SnipItem>> = Rc::new(VecModel::default());
                self.gallery_rows.push(inner.clone());
                rows.push(ModelRc::from(inner));
            }
            self.gallery_rows[i / cols].push(item.clone());
        }

        window.set_rows(ModelRc::new(VecModel::from(rows)));
    }

    /// Called from the UI (synchronously, even during a live resize) when the
    /// number of cards that fit per row changes; re-chunks without a full
    /// rebuild so the grid reflows as the window is dragged.
    pub fn set_gallery_cols(&mut self, cols: usize, window: &MainWindow) {
        let cols = cols.max(1);
        if cols != self.gallery_cols {
            self.gallery_cols = cols;
            self.rechunk_rows(window);
        }
    }

    fn matches_filters(
        &self,
        snip: &crate::snip::Snip,
        metadata: &crate::snip::SnipMetadataIndex,
    ) -> bool {
        self.matches_status_filter(snip)
            && self.matches_content_filter(snip, metadata)
            && self.matches_size_filter(snip)
            && self.matches_date_filter(snip)
    }

    fn matches_status_filter(&self, snip: &crate::snip::Snip) -> bool {
        let is_floating = self.floating.is_shown(snip.id);
        match self.filters.status {
            1 => snip.pinned,
            2 => !snip.pinned,
            3 => is_floating,
            4 => !is_floating,
            _ => true,
        }
    }

    fn matches_content_filter(
        &self,
        snip: &crate::snip::Snip,
        metadata: &crate::snip::SnipMetadataIndex,
    ) -> bool {
        match self.filters.content {
            1 => metadata.ocr_ids.contains(&snip.id),
            2 => !metadata.ocr_ids.contains(&snip.id),
            3 => metadata.window_ids.contains(&snip.id),
            4 => !metadata.window_ids.contains(&snip.id),
            5 => snip
                .label
                .as_deref()
                .map(|label| !label.trim().is_empty())
                .unwrap_or(false),
            _ => true,
        }
    }

    fn matches_size_filter(&self, snip: &crate::snip::Snip) -> bool {
        let pixels = snip.area.w as u64 * snip.area.h as u64;
        match self.filters.size {
            1 => pixels < 250_000,
            2 => (250_000..1_000_000).contains(&pixels),
            3 => pixels >= 1_000_000,
            4 => snip.area.w as f32 > snip.area.h as f32 * 1.30,
            5 => snip.area.h as f32 > snip.area.w as f32 * 1.30,
            6 => {
                let min_side = snip.area.w.min(snip.area.h).max(1) as f32;
                let max_side = snip.area.w.max(snip.area.h) as f32;
                max_side / min_side <= 1.20
            }
            _ => true,
        }
    }

    fn matches_date_filter(&self, snip: &crate::snip::Snip) -> bool {
        let now = chrono::Utc::now();
        match self.filters.date {
            1 => {
                snip.created_at.with_timezone(&chrono::Local).date_naive()
                    == chrono::Local::now().date_naive()
            }
            2 => snip.created_at >= now - chrono::Duration::hours(24),
            3 => snip.created_at >= now - chrono::Duration::days(7),
            4 => snip.created_at >= now - chrono::Duration::days(30),
            _ => true,
        }
    }

    fn sort_items(&self, items: &mut [crate::snip::Snip]) {
        items.sort_by(|a, b| match self.filters.sort {
            1 => a.created_at.cmp(&b.created_at),
            2 => b
                .pinned
                .cmp(&a.pinned)
                .then(b.created_at.cmp(&a.created_at)),
            3 => snip_pixels(b).cmp(&snip_pixels(a)),
            4 => snip_pixels(a).cmp(&snip_pixels(b)),
            5 => b
                .area
                .w
                .cmp(&a.area.w)
                .then(b.created_at.cmp(&a.created_at)),
            6 => b
                .area
                .h
                .cmp(&a.area.h)
                .then(b.created_at.cmp(&a.created_at)),
            7 => a
                .monitor_label
                .cmp(&b.monitor_label)
                .then(b.created_at.cmp(&a.created_at)),
            _ => b
                .pinned
                .cmp(&a.pinned)
                .then(b.created_at.cmp(&a.created_at)),
        });
    }

    /// Decode (or fetch from the bounded cache) the thumbnail for `id`.
    /// Returns a blank image if the snip or its thumbnail can't be loaded.
    fn thumb_image(&mut self, id: Uuid) -> Image {
        if let Some(img) = self.thumb_cache.get(&id) {
            return img.clone();
        }
        let Some(snip) = self.gallery.get(id) else {
            return Image::default();
        };
        let Ok(raw) = snip.ensure_thumbnail() else {
            return Image::default();
        };
        let buf = SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
            raw.as_raw(),
            raw.width(),
            raw.height(),
        );
        let img = Image::from_rgba8(buf);
        self.cache_thumb(id, img.clone());
        img
    }

    /// Insert a decoded thumbnail, evicting the oldest entries so the cache
    /// (and thus memory) stays bounded no matter how many snips exist.
    fn cache_thumb(&mut self, id: Uuid, img: Image) {
        const CAP: usize = 400;
        if self.thumb_cache.insert(id, img).is_none() {
            self.thumb_order.push_back(id);
        }
        while self.thumb_cache.len() > CAP {
            match self.thumb_order.pop_front() {
                Some(old) if old != id => {
                    self.thumb_cache.remove(&old);
                }
                Some(_) => {}
                None => break,
            }
        }
    }

    /// Lazy thumbnail write-back: a card scrolled into view, so decode its
    /// thumbnail (if needed) and push it into that exact card's model item.
    pub(crate) fn load_thumb_into_model(&mut self, id: Uuid) {
        let Some(&idx) = self.card_index.get(&id) else {
            return;
        };
        let img = self.thumb_image(id);
        // Persist into the flat cache so a later re-chunk (column change)
        // keeps the loaded thumbnail instead of reverting to blank.
        if let Some((_, card)) = self.gallery_cards.get_mut(idx) {
            card.thumb = img.clone();
        }
        let cols = self.gallery_cols.max(1);
        if let Some(row) = self.gallery_rows.get(idx / cols) {
            if let Some(mut item) = row.row_data(idx % cols) {
                item.thumb = img;
                row.set_row_data(idx % cols, item);
            }
        }
    }
}
