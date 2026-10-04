use anyhow::Context;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::OnceLock;
use uuid::Uuid;

use crate::paths;
use crate::window_metadata::WindowInfo;

const THUMB_MAX_DIM: u32 = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ScreenRect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

impl ScreenRect {
    pub fn new(x: i32, y: i32, w: u32, h: u32) -> Self {
        Self { x, y, w, h }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Snip {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub area: ScreenRect,
    pub image_file: String, // file name inside images_dir
    #[serde(default)]
    pub thumb_file: Option<String>, // file name inside thumbs_dir
    pub label: Option<String>,
    #[serde(default)]
    pub pinned: bool,
    /// Human-readable monitor name the snip came from, e.g. "Monitor 1".
    #[serde(default)]
    pub monitor_label: Option<String>,
    /// Persisted across sessions: should this snip be opened as a floating window on launch?
    #[serde(default)]
    pub floating: bool,
    /// Last user-chosen position for the floating window (screen coordinates).
    #[serde(default)]
    pub last_position: Option<(i32, i32)>,
}

impl Snip {
    /// Record where the floating window ended up — but only where that answer
    /// is real.
    ///
    /// Every caller gets its position from `Window::position()`, directly or
    /// through a drag delta built on it, and on Wayland that answers 0,0 for
    /// the lifetime of the window: a client is never told where the compositor
    /// put it. Persisting it stored junk — 0,0 for resize and crop, small or
    /// negative drift for drags — and `floating.rs` reads `last_position` back
    /// *in preference to* the capture rect, so a snip that had ever been
    /// dragged or resized reopened in the corner instead of where it was taken.
    ///
    /// `query_own_position` is the capability that says whether the answer can
    /// be trusted. Where it is false the field simply stays `None`, and the
    /// snip reopens where it was captured — which is the correct behaviour
    /// there, not a degradation.
    pub fn remember_position(&mut self, pos: (i32, i32)) {
        if crate::window_env::caps().query_own_position {
            self.last_position = Some(pos);
        }
    }

    pub fn image_path(&self) -> PathBuf {
        paths::images_dir().join(&self.image_file)
    }

    pub fn thumb_path(&self) -> PathBuf {
        let file = self
            .thumb_file
            .clone()
            .unwrap_or_else(|| default_thumb_file(self.id));
        paths::thumbs_dir().join(file)
    }

    /// Write the snip's image and thumbnail.
    ///
    /// The file name follows the format the image is stored in
    /// (`image_files::file_name`) — lossless WebP, or PNG for one too large for
    /// it. The snip's file names only change once both new files are on disk,
    /// so a failed write leaves it pointing at its old, intact image rather
    /// than at a file that was never written; the old files, if the names
    /// changed, are removed after. Callers persist the gallery afterwards,
    /// which records the new names.
    pub fn save_image(&mut self, image: &image::RgbaImage) -> anyhow::Result<()> {
        let image_file = crate::image_files::file_name(&self.id.to_string(), image);
        let thumb_file = default_thumb_file(self.id);
        let image_path = paths::images_dir().join(&image_file);
        let thumb_path = paths::thumbs_dir().join(&thumb_file);
        crate::image_files::write(&image_path, image)
            .with_context(|| format!("saving snip image to {}", image_path.display()))?;
        crate::image_files::write(&thumb_path, &thumbnail_from(image, THUMB_MAX_DIM))
            .with_context(|| format!("saving snip thumbnail to {}", thumb_path.display()))?;

        let old_image = (!self.image_file.is_empty()).then(|| self.image_path());
        let old_thumb = self.thumb_path();
        self.image_file = image_file;
        self.thumb_file = Some(thumb_file);
        for (old, new) in [(old_image, &image_path), (Some(old_thumb), &thumb_path)] {
            if let Some(old) = old.filter(|old| old != new) {
                crate::image_files::remove(&old);
            }
        }
        Ok(())
    }

    pub fn load_image(&self) -> anyhow::Result<image::RgbaImage> {
        let path = self.image_path();
        let img = image::open(&path)
            .with_context(|| format!("loading snip image from {}", path.display()))?
            .to_rgba8();
        Ok(img)
    }

    pub fn load_thumbnail(&self) -> anyhow::Result<image::RgbaImage> {
        let path = self.thumb_path();
        let img = image::open(&path)
            .with_context(|| format!("loading snip thumbnail from {}", path.display()))?
            .to_rgba8();
        Ok(img)
    }

    pub fn ensure_thumbnail(&self) -> anyhow::Result<image::RgbaImage> {
        if let Ok(img) = self.load_thumbnail() {
            return Ok(img);
        }
        let full = self.load_image()?;
        let thumb = thumbnail_from(&full, THUMB_MAX_DIM);
        crate::image_files::write(&self.thumb_path(), &thumb)?;
        Ok(thumb)
    }

    /// Remove the snip's files — under every name it may have on disk, not
    /// just the recorded one: a background conversion from PNG may be writing
    /// the WebP sibling right now, and removing it bumps its generation so that
    /// write is discarded instead of left behind.
    pub fn delete_files(&self) {
        let id = self.id;
        let mut paths = vec![
            self.image_path(),
            self.thumb_path(),
            paths::images_dir().join(format!("{id}.webp")),
            paths::images_dir().join(format!("{id}.png")),
            paths::thumbs_dir().join(format!("{id}.thumb.webp")),
            paths::thumbs_dir().join(format!("{id}.thumb.png")),
        ];
        paths.sort();
        paths.dedup();
        for path in paths {
            crate::image_files::remove(&path);
        }
    }
}

pub fn make_snip(area: ScreenRect) -> Snip {
    let id = Uuid::new_v4();
    Snip {
        id,
        created_at: Utc::now(),
        area,
        // Placeholder until `save_image` writes the file and records its name.
        image_file: format!("{id}.webp"),
        thumb_file: Some(default_thumb_file(id)),
        label: None,
        pinned: false,
        monitor_label: None,
        floating: false,
        last_position: None,
    }
}

/// A snip moved from PNG to lossless WebP in the background, ready to be
/// recorded by [`GalleryState::apply_conversion`].
pub struct Converted {
    id: Uuid,
    /// The PNG the conversion read; the snip must still point at it.
    from: String,
    image_file: String,
    /// `None` when the thumbnail lost a race and kept its old file.
    thumb_file: Option<String>,
}

/// Convert snips still stored as PNG — written before snips were stored as
/// WebP — on a background thread, one after another. Each finished snip comes
/// back on the receiver. `None` when there is nothing to convert.
///
/// Losslessly: the WebP decodes to exactly the PNG's pixels. Every write goes
/// through `image_files::write_webp_if_unchanged`, so an edit or a delete that
/// lands while a snip is being converted always wins over the conversion.
pub fn convert_legacy_pngs(snips: &[Snip]) -> Option<crossbeam_channel::Receiver<Converted>> {
    let todo: Vec<(Uuid, String)> = snips
        .iter()
        .filter(|s| s.image_file.to_ascii_lowercase().ends_with(".png"))
        .map(|s| (s.id, s.image_file.clone()))
        .collect();
    if todo.is_empty() {
        return None;
    }
    let (tx, rx) = crossbeam_channel::unbounded();
    std::thread::Builder::new()
        .name("webp-convert".into())
        .spawn(move || {
            let mut done = 0;
            for (id, from) in todo {
                match convert_one(id, &from) {
                    Ok(Some(converted)) => {
                        done += 1;
                        if tx.send(converted).is_err() {
                            return;
                        }
                        crate::pump::wake();
                    }
                    Ok(None) => {}
                    Err(err) => log::debug!("converting snip {id} to WebP skipped: {err:#}"),
                }
            }
            log::info!("converted {done} snips from PNG to lossless WebP");
            // Once more after the channel closes, so the UI notices it is done
            // (and refreshes) even if the pump went to sleep after the last one.
            drop(tx);
            crate::pump::wake();
        })
        .ok()?;
    Some(rx)
}

fn convert_one(id: Uuid, from: &str) -> anyhow::Result<Option<Converted>> {
    let image_file = format!("{id}.webp");
    let thumb_file = default_thumb_file(id);
    let image_path = paths::images_dir().join(&image_file);
    let thumb_path = paths::thumbs_dir().join(&thumb_file);
    // Read before decoding: anything that touches these paths from here on
    // makes the conversion stand down.
    let image_generation = crate::image_files::generation(&image_path);
    let thumb_generation = crate::image_files::generation(&thumb_path);
    let image = image::open(paths::images_dir().join(from))?.to_rgba8();
    if crate::image_files::file_name(&id.to_string(), &image) != image_file {
        return Ok(None); // too large for WebP; it stays a PNG
    }
    if !crate::image_files::write_webp_if_unchanged(&image_path, &image, image_generation)? {
        return Ok(None);
    }
    let thumb = thumbnail_from(&image, THUMB_MAX_DIM);
    let thumb_written =
        crate::image_files::write_webp_if_unchanged(&thumb_path, &thumb, thumb_generation)?;
    Ok(Some(Converted {
        id,
        from: from.to_string(),
        image_file,
        thumb_file: thumb_written.then_some(thumb_file),
    }))
}

/// The gallery: every snip, in memory, kept in step with the database.
#[derive(Debug, Clone, Default)]
pub struct GalleryState {
    /// Newest first.
    pub snips: Vec<Snip>,
    /// Each snip as the database last stored it, so `save` can tell what
    /// changed — however the caller changed it.
    persisted: HashMap<Uuid, Snip>,
}

impl GalleryState {
    /// Read the gallery. If the database cannot be read the gallery starts
    /// empty, which costs nothing stored: `save` writes only snips that are
    /// new or changed and deletes nothing, so the stored ones come back on the
    /// next launch that can read them.
    pub fn load() -> Self {
        match Self::load_from_db() {
            Ok(state) => state,
            Err(err) => {
                log::error!("the gallery could not be read: {err:#}");
                Self::default()
            }
        }
    }

    fn load_from_db() -> anyhow::Result<Self> {
        with_db(|conn| {
            purge_orphans(conn);
            let mut stmt = conn.prepare(
                "SELECT id, created_at, x, y, w, h, image_file, thumb_file, label, pinned,
                        monitor_label, floating, last_x, last_y
                 FROM snips
                 ORDER BY pinned DESC, created_at DESC",
            )?;
            let snips = stmt
                .query_map([], snip_from_row)?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            let persisted = snips.iter().map(|s| (s.id, s.clone())).collect();
            Ok(Self { snips, persisted })
        })
    }

    /// Write the snips that are new or changed since the last save, and
    /// nothing else; with nothing changed the database is not touched.
    ///
    /// It used to rewrite every snip and its search row on every call — and
    /// nearly every action calls it. With each search row's delete scanning the
    /// whole index, that was quadratic in the gallery's size, on the UI thread:
    /// a few hundred snips and a click spent most of its time in SQLite.
    /// Comparing against what was last stored is a pass over memory.
    pub fn save(&mut self) -> anyhow::Result<()> {
        let persisted = &self.persisted;
        let changed: Vec<Snip> = self
            .snips
            .iter()
            .filter(|s| persisted.get(&s.id) != Some(*s))
            .cloned()
            .collect();
        if changed.is_empty() {
            return Ok(());
        }
        with_db(|conn| {
            let tx = conn.transaction()?;
            upsert_snips(&tx, &changed)?;
            tx.commit()?;
            Ok(())
        })?;
        for snip in changed {
            self.persisted.insert(snip.id, snip);
        }
        Ok(())
    }

    pub fn push_new(&mut self, snip: Snip) {
        self.snips.insert(0, snip);
    }

    /// Drop a snip: row first, then the files it points at.
    ///
    /// The order is the whole point. Deleting the files first and then failing
    /// to delete the row — SQLITE_BUSY outlasting the timeout, the case
    /// `open_db` documents as having actually happened — leaves a gallery card
    /// whose PNG is already gone, and `save()` only ever upserts, so nothing
    /// removes it afterwards: a permanent ghost. Doing it this way, a failed
    /// delete leaves the snip whole, and it simply comes back on the next
    /// launch as though the delete had never been asked for.
    pub fn remove(&mut self, id: Uuid) -> Option<Snip> {
        let pos = self.snips.iter().position(|s| s.id == id)?;
        let snip = self.snips.remove(pos);
        match delete_snip_from_db(id) {
            Ok(()) => {
                self.persisted.remove(&id);
                snip.delete_files();
            }
            Err(err) => log::warn!("deleting snip from db failed, keeping its files: {err}"),
        }
        Some(snip)
    }

    /// Same ordering argument as [`GalleryState::remove`].
    pub fn clear_all(&mut self) {
        match clear_db_snips() {
            Ok(()) => {
                self.persisted.clear();
                for snip in &self.snips {
                    snip.delete_files();
                }
            }
            Err(err) => log::warn!("clearing snip db failed, keeping the files: {err}"),
        }
        self.snips.clear();
    }

    /// Point a converted snip at its WebP files. Returns the old files, to be
    /// removed only once the gallery has been saved with the new names — the
    /// other order, a crash in between left the database pointing at a PNG
    /// that no longer existed. Returns nothing when the snip was deleted or
    /// edited since: an edit already moved it, and removed the PNG itself.
    pub fn apply_conversion(&mut self, converted: Converted) -> Vec<PathBuf> {
        let Some(snip) = self.get_mut(converted.id) else {
            return Vec::new();
        };
        if snip.image_file != converted.from {
            return Vec::new();
        }
        let mut stale = vec![snip.image_path()];
        snip.image_file = converted.image_file;
        if let Some(thumb_file) = converted.thumb_file {
            let old_thumb = snip.thumb_path();
            snip.thumb_file = Some(thumb_file);
            if old_thumb != snip.thumb_path() {
                stale.push(old_thumb);
            }
        }
        stale
    }

    pub fn get(&self, id: Uuid) -> Option<&Snip> {
        self.snips.iter().find(|s| s.id == id)
    }

    pub fn get_mut(&mut self, id: Uuid) -> Option<&mut Snip> {
        self.snips.iter_mut().find(|s| s.id == id)
    }

    pub fn search(&self, query: &str) -> anyhow::Result<Vec<Snip>> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Ok(self.snips.clone());
        }
        with_db(|conn| {
            if let Some(match_query) = fts_match_query(trimmed) {
                match search_fts(conn, &match_query) {
                    Ok(snips) => return Ok(snips),
                    Err(err) => {
                        log::debug!("SQLite FTS search failed, falling back to LIKE: {err}");
                    }
                }
            }
            search_like(conn, trimmed)
        })
    }
}

fn search_fts(conn: &Connection, match_query: &str) -> anyhow::Result<Vec<Snip>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT
                s.id, s.created_at, s.x, s.y, s.w, s.h, s.image_file, s.thumb_file,
                s.label, s.pinned, s.monitor_label, s.floating, s.last_x, s.last_y
         FROM snip_fts
         JOIN snips s ON s.rowid = snip_fts.rowid
         WHERE snip_fts MATCH ?1
         ORDER BY s.pinned DESC, s.created_at DESC",
    )?;
    let rows = stmt.query_map([match_query], snip_from_row)?;
    let mut snips = Vec::new();
    for row in rows {
        snips.push(row?);
    }
    Ok(snips)
}

/// Make a string safe to drop inside a LIKE pattern.
///
/// Backslash first: escaping it after `%` and `_` would put a backslash in
/// front of the backslashes this function had just added.
fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn search_like(conn: &Connection, query: &str) -> anyhow::Result<Vec<Snip>> {
    // Fold the query the same way as the indexed columns (see `tr_fold`)
    // so the fallback is case-, diacritic- and dotless-`ı` insensitive too.
    //
    // `%` and `_` are LIKE's own wildcards, so a query containing either was
    // read as a pattern rather than as text: searching "100%" matched every
    // snip with "100" anywhere in it, and "a_b" matched "axb". Escaped here
    // (backslash first, or it would double-escape the escapes it adds) and
    // declared with ESCAPE on every clause below.
    let like = format!("%{}%", escape_like(&search_fold(query)));
    let mut stmt = conn.prepare(
        r"SELECT DISTINCT
                s.id, s.created_at, s.x, s.y, s.w, s.h, s.image_file, s.thumb_file,
                s.label, s.pinned, s.monitor_label, s.floating, s.last_x, s.last_y
         FROM snips s
         LEFT JOIN snip_ocr o ON o.snip_id = s.id
         LEFT JOIN snip_window win ON win.snip_id = s.id
         WHERE tr_fold(s.label) LIKE ?1 ESCAPE '\'
            OR tr_fold(s.monitor_label) LIKE ?1 ESCAPE '\'
            OR tr_fold(o.text) LIKE ?1 ESCAPE '\'
            OR tr_fold(win.title) LIKE ?1 ESCAPE '\'
            OR tr_fold(win.app_name) LIKE ?1 ESCAPE '\'
            OR tr_fold(win.process_name) LIKE ?1 ESCAPE '\'
            OR tr_fold(win.exe_path) LIKE ?1 ESCAPE '\'
            OR (CAST(s.w AS TEXT) || 'x' || CAST(s.h AS TEXT)) LIKE ?1 ESCAPE '\'
         ORDER BY s.pinned DESC, s.created_at DESC",
    )?;
    let rows = stmt.query_map([like], snip_from_row)?;
    let mut snips = Vec::new();
    for row in rows {
        snips.push(row?);
    }
    Ok(snips)
}

fn fts_match_query(query: &str) -> Option<String> {
    let tokens: Vec<String> = search_fold(query)
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(|token| format!("{token}*"))
        .collect();
    if tokens.is_empty() {
        None
    } else {
        Some(tokens.join(" AND "))
    }
}

/// Normalise text for search so matching is case-, diacritic- and
/// dotted/dotless-`i` insensitive.
///
/// SQLite's `unicode61` FTS tokenizer already lowercases and strips most
/// Latin diacritics, but it does *not* bridge Turkish's dotless `ı`
/// (U+0131) and dotted `i` — so an OCR'd "ışık" could never be found by
/// typing "isik", and vice versa. We fold the Turkish letters to their
/// ASCII base ourselves and apply this identically to both the FTS index
/// content (`upsert_fts_for_snip`) and the query (`fts_match_query`) so
/// matches stay symmetric. The same fold backs the `tr_fold()` SQL
/// function used by the `LIKE` fallback, whose `COLLATE NOCASE` only ever
/// folded ASCII `A`–`Z`.
fn search_fold(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            'ı' | 'İ' | 'I' | 'i' => out.push('i'),
            'ş' | 'Ş' => out.push('s'),
            'ğ' | 'Ğ' => out.push('g'),
            'ç' | 'Ç' => out.push('c'),
            'ö' | 'Ö' => out.push('o'),
            'ü' | 'Ü' => out.push('u'),
            other => out.extend(other.to_lowercase()),
        }
    }
    out
}

fn default_thumb_file(id: Uuid) -> String {
    format!("{id}.thumb.webp")
}

fn thumbnail_from(image: &image::RgbaImage, max_dim: u32) -> image::RgbaImage {
    let (w, h) = image.dimensions();
    if w.max(h) <= max_dim {
        return image.clone();
    }
    let ratio = max_dim as f32 / w.max(h) as f32;
    let tw = ((w as f32) * ratio).round().max(1.0) as u32;
    let th = ((h as f32) * ratio).round().max(1.0) as u32;
    image::imageops::resize(image, tw, th, image::imageops::FilterType::Triangle)
}

thread_local! {
    // This thread's connection, opened on first use and kept. Opening one ran
    // the whole schema script and the search-index checks again, and the UI
    // thread opened one per keystroke in the search box and per save. Worker
    // threads (OCR, window details) get their own, closed when they end.
    static DB: RefCell<Option<Connection>> = const { RefCell::new(None) };
}

/// Run `f` on this thread's connection. `f` must not call back into
/// `with_db`: the connection is borrowed for the duration.
fn with_db<T>(f: impl FnOnce(&mut Connection) -> anyhow::Result<T>) -> anyhow::Result<T> {
    DB.with(|cell| {
        let mut slot = cell.borrow_mut();
        let conn = match slot.as_mut() {
            Some(conn) => conn,
            None => slot.insert(open_db()?),
        };
        f(conn)
    })
}

fn open_db() -> anyhow::Result<Connection> {
    paths::ensure_dirs()?;
    let conn = Connection::open(paths::db_file())?;
    // WAL allows a single writer at a time; the detached OCR-indexing threads
    // (`save_ocr_text`, one per snip) write on their own connections
    // concurrently with the main thread's `gallery.save()`. rusqlite's default
    // busy handling fails a losing writer instantly with SQLITE_BUSY — and
    // because the save call sites discarded the error, the snip's DB row never
    // committed (its PNG left orphaned on disk, the snip gone on next launch).
    // Wait up to 5 s for the lock instead of racing.
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    // The schema declares snip_ocr and snip_window as REFERENCES snips(id) ON
    // DELETE CASCADE, but SQLite enforces none of that unless asked, per
    // connection. Without it a detached OCR thread finishing after the user
    // deleted its snip wrote the OCR text back for a snip that no longer
    // existed, and nothing ever removed it.
    conn.pragma_update(None, "foreign_keys", true)?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    register_search_functions(&conn)?;
    // The schema once per process: every statement in it is "if not exists",
    // and running it for each connection only cost time.
    static SCHEMA: OnceLock<()> = OnceLock::new();
    if SCHEMA.get().is_none() {
        init_db(&conn)?;
        let _ = SCHEMA.set(());
    }
    Ok(conn)
}

/// Register the `tr_fold(text)` SQL function so the `LIKE` search
/// fallback can match with the same Turkish-aware folding as
/// [`search_fold`]. Idempotent — safe to call on every connection.
fn register_search_functions(conn: &Connection) -> anyhow::Result<()> {
    use rusqlite::functions::FunctionFlags;
    conn.create_scalar_function(
        "tr_fold",
        1,
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
        |ctx| {
            let value: Option<String> = ctx.get(0)?;
            Ok(value.map(|s| search_fold(&s)))
        },
    )?;
    Ok(())
}

fn init_db(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(
        "
        PRAGMA journal_mode = WAL;
        CREATE TABLE IF NOT EXISTS meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS snips (
            id TEXT PRIMARY KEY,
            created_at TEXT NOT NULL,
            x INTEGER NOT NULL,
            y INTEGER NOT NULL,
            w INTEGER NOT NULL,
            h INTEGER NOT NULL,
            image_file TEXT NOT NULL,
            thumb_file TEXT,
            label TEXT,
            pinned INTEGER NOT NULL DEFAULT 0,
            monitor_label TEXT,
            floating INTEGER NOT NULL DEFAULT 0,
            last_x INTEGER,
            last_y INTEGER
        );
        CREATE INDEX IF NOT EXISTS idx_snips_created_at ON snips(created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_snips_pinned_created ON snips(pinned DESC, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_snips_monitor ON snips(monitor_label);
        CREATE INDEX IF NOT EXISTS idx_snips_size ON snips(w, h);
        CREATE TABLE IF NOT EXISTS snip_ocr (
            snip_id TEXT PRIMARY KEY REFERENCES snips(id) ON DELETE CASCADE,
            engine TEXT NOT NULL,
            language TEXT,
            text TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_snip_ocr_updated ON snip_ocr(updated_at DESC);
        CREATE TABLE IF NOT EXISTS snip_window (
            snip_id TEXT PRIMARY KEY REFERENCES snips(id) ON DELETE CASCADE,
            platform TEXT NOT NULL,
            window_id TEXT NOT NULL,
            title TEXT,
            app_name TEXT,
            process_name TEXT,
            pid INTEGER,
            exe_path TEXT,
            x INTEGER NOT NULL,
            y INTEGER NOT NULL,
            w INTEGER NOT NULL,
            h INTEGER NOT NULL,
            confidence REAL NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_snip_window_title ON snip_window(title);
        CREATE INDEX IF NOT EXISTS idx_snip_window_app ON snip_window(app_name);
        CREATE INDEX IF NOT EXISTS idx_snip_window_process ON snip_window(process_name);
        ",
    )?;
    if let Err(err) = init_fts(conn).and_then(|_| rebuild_fts_if_needed(conn)) {
        static WARNED: OnceLock<()> = OnceLock::new();
        if WARNED.set(()).is_ok() {
            log::warn!("SQLite FTS setup failed; search will use LIKE fallback: {err}");
        }
    }
    Ok(())
}

/// The search index: one row per snip, its rowid the snip's rowid in
/// `snips`, holding a folded copy of everything searchable.
///
/// It used to carry the snip's id in an UNINDEXED column instead, and finding
/// a snip's row by that meant reading the whole index: every update and every
/// delete of a search row was a full scan.
fn init_fts(conn: &Connection) -> anyhow::Result<()> {
    if fts_needs_recreate(conn)? {
        conn.execute("DROP TABLE IF EXISTS snip_fts", [])?;
        conn.execute("DELETE FROM meta WHERE key LIKE 'fts5_index_built_%'", [])?;
    }
    conn.execute_batch(
        "
        CREATE VIRTUAL TABLE IF NOT EXISTS snip_fts USING fts5(
            label,
            monitor_label,
            dims,
            ocr_text,
            window_title,
            window_app,
            window_process,
            window_path
        );
        ",
    )?;
    Ok(())
}

/// An index in an older layout: keyed by an id column, or without the window
/// columns.
fn fts_needs_recreate(conn: &Connection) -> anyhow::Result<bool> {
    let columns = table_columns(conn, "snip_fts")?;
    Ok(!columns.is_empty()
        && (columns.iter().any(|col| col == "snip_id")
            || !columns.iter().any(|col| col == "window_title")))
}

fn table_columns(conn: &Connection, table: &str) -> anyhow::Result<Vec<String>> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
    let mut columns = Vec::new();
    for row in rows {
        columns.push(row?);
    }
    Ok(columns)
}

fn rebuild_fts_if_needed(conn: &Connection) -> anyhow::Result<()> {
    // Bumping the key forces one rebuild: v3 folded the text through
    // `search_fold`, v4 keys the rows by the snips' rowid.
    let built: Option<String> = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'fts5_index_built_v4'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    if built.as_deref() == Some("1") {
        return Ok(());
    }

    rebuild_fts(conn)?;
    conn.execute(
        "INSERT OR REPLACE INTO meta(key, value) VALUES('fts5_index_built_v4', '1')",
        [],
    )?;
    Ok(())
}

fn rebuild_fts(conn: &Connection) -> anyhow::Result<()> {
    conn.execute("DELETE FROM snip_fts", [])?;
    let snips = {
        let mut stmt = conn.prepare(
            "SELECT id, created_at, x, y, w, h, image_file, thumb_file, label, pinned,
                    monitor_label, floating, last_x, last_y
             FROM snips
             ORDER BY pinned DESC, created_at DESC",
        )?;
        let rows = stmt.query_map([], snip_from_row)?;
        let mut snips = Vec::new();
        for row in rows {
            snips.push(row?);
        }
        snips
    };

    for snip in snips {
        upsert_fts_for_snip(conn, &snip, None)?;
    }
    Ok(())
}

fn upsert_snips(conn: &Connection, snips: &[Snip]) -> anyhow::Result<()> {
    let mut stmt = conn.prepare(
        "
        INSERT INTO snips (
            id, created_at, x, y, w, h, image_file, thumb_file, label, pinned,
            monitor_label, floating, last_x, last_y
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
        ON CONFLICT(id) DO UPDATE SET
            created_at=excluded.created_at,
            x=excluded.x,
            y=excluded.y,
            w=excluded.w,
            h=excluded.h,
            image_file=excluded.image_file,
            thumb_file=excluded.thumb_file,
            label=excluded.label,
            pinned=excluded.pinned,
            monitor_label=excluded.monitor_label,
            floating=excluded.floating,
            last_x=excluded.last_x,
            last_y=excluded.last_y
        ",
    )?;
    for snip in snips {
        let (last_x, last_y) = snip
            .last_position
            .map(|(x, y)| (Some(x), Some(y)))
            .unwrap_or((None, None));
        stmt.execute(params![
            snip.id.to_string(),
            snip.created_at.to_rfc3339(),
            snip.area.x,
            snip.area.y,
            snip.area.w as i64,
            snip.area.h as i64,
            snip.image_file.as_str(),
            snip.thumb_file.as_deref(),
            snip.label.as_deref(),
            snip.pinned as i32,
            snip.monitor_label.as_deref(),
            snip.floating as i32,
            last_x,
            last_y,
        ])?;
        if let Err(err) = upsert_fts_for_snip(conn, snip, None) {
            log::debug!("snip FTS metadata update skipped for {}: {err}", snip.id);
        }
    }
    Ok(())
}

/// Rows left behind for snips that no longer exist — OCR text a late OCR
/// thread wrote after a delete, from before foreign keys were enforced. Text
/// from a snip the user deleted has no business staying on disk.
fn purge_orphans(conn: &Connection) {
    for sql in [
        "DELETE FROM snip_ocr WHERE snip_id NOT IN (SELECT id FROM snips)",
        "DELETE FROM snip_window WHERE snip_id NOT IN (SELECT id FROM snips)",
        "DELETE FROM snip_fts WHERE rowid NOT IN (SELECT rowid FROM snips)",
    ] {
        match conn.execute(sql, []) {
            Ok(0) => {}
            Ok(n) => log::info!("removed {n} rows left behind by deleted snips"),
            Err(err) => log::debug!("orphan cleanup skipped: {err}"),
        }
    }
}

/// One transaction, so a failure leaves the snip whole rather than half
/// gone. Its OCR text and window details go with it (ON DELETE CASCADE).
fn delete_snip_from_db(id: Uuid) -> anyhow::Result<()> {
    with_db(|conn| {
        let tx = conn.transaction()?;
        let id_text = id.to_string();
        tx.execute(
            "DELETE FROM snip_fts WHERE rowid = (SELECT rowid FROM snips WHERE id = ?1)",
            [id_text.as_str()],
        )?;
        tx.execute("DELETE FROM snips WHERE id = ?1", [id_text.as_str()])?;
        tx.commit()?;
        Ok(())
    })
}

fn clear_db_snips() -> anyhow::Result<()> {
    with_db(|conn| {
        let tx = conn.transaction()?;
        tx.execute("DELETE FROM snip_fts", [])?;
        tx.execute("DELETE FROM snips", [])?;
        tx.commit()?;
        Ok(())
    })
}

fn snip_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Snip> {
    let id_text: String = row.get(0)?;
    let created_text: String = row.get(1)?;
    let id = Uuid::parse_str(&id_text).unwrap_or_else(|_| Uuid::nil());
    let created_at = DateTime::parse_from_rfc3339(&created_text)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    let last_x: Option<i32> = row.get(12)?;
    let last_y: Option<i32> = row.get(13)?;
    Ok(Snip {
        id,
        created_at,
        area: ScreenRect {
            x: row.get(2)?,
            y: row.get(3)?,
            w: row.get::<_, i64>(4)? as u32,
            h: row.get::<_, i64>(5)? as u32,
        },
        image_file: row.get(6)?,
        thumb_file: row.get(7)?,
        label: row.get(8)?,
        pinned: row.get::<_, i32>(9)? != 0,
        monitor_label: row.get(10)?,
        floating: row.get::<_, i32>(11)? != 0,
        last_position: match (last_x, last_y) {
            (Some(x), Some(y)) => Some((x, y)),
            _ => None,
        },
    })
}

fn load_snip_by_id(conn: &Connection, id: Uuid) -> anyhow::Result<Option<Snip>> {
    let mut stmt = conn.prepare(
        "SELECT id, created_at, x, y, w, h, image_file, thumb_file, label, pinned,
                monitor_label, floating, last_x, last_y
         FROM snips
         WHERE id = ?1",
    )?;
    let snip = stmt.query_row([id.to_string()], snip_from_row).optional()?;
    Ok(snip)
}

fn cached_ocr_text_with_conn(conn: &Connection, id: Uuid) -> anyhow::Result<Option<String>> {
    let text = conn
        .query_row(
            "SELECT text FROM snip_ocr WHERE snip_id = ?1",
            [id.to_string()],
            |row| row.get(0),
        )
        .optional()?;
    Ok(text)
}

#[derive(Default)]
struct WindowFtsText {
    title: Option<String>,
    app_name: Option<String>,
    process_name: Option<String>,
    exe_path: Option<String>,
}

fn cached_window_fts_text_with_conn(conn: &Connection, id: Uuid) -> anyhow::Result<WindowFtsText> {
    let text = conn
        .query_row(
            "SELECT title, app_name, process_name, exe_path
             FROM snip_window
             WHERE snip_id = ?1",
            [id.to_string()],
            |row| {
                Ok(WindowFtsText {
                    title: row.get(0)?,
                    app_name: row.get(1)?,
                    process_name: row.get(2)?,
                    exe_path: row.get(3)?,
                })
            },
        )
        .optional()?
        .unwrap_or_default();
    Ok(text)
}

fn upsert_fts_for_snip(
    conn: &Connection,
    snip: &Snip,
    ocr_text: Option<&str>,
) -> anyhow::Result<()> {
    let ocr_owned;
    let ocr_text = if let Some(ocr_text) = ocr_text {
        ocr_text
    } else {
        ocr_owned = cached_ocr_text_with_conn(conn, snip.id)?.unwrap_or_default();
        ocr_owned.as_str()
    };
    let window = cached_window_fts_text_with_conn(conn, snip.id)?;
    let Some(rowid) = conn
        .query_row(
            "SELECT rowid FROM snips WHERE id = ?1",
            [snip.id.to_string()],
            |row| row.get::<_, i64>(0),
        )
        .optional()?
    else {
        return Ok(());
    };
    let dims = format!("{}x{}", snip.area.w, snip.area.h);
    // Store a search-folded copy of every matchable column. `snip_fts` is
    // a pure index (searches JOIN back to `snips` by rowid for the displayed
    // rows), so folding here never touches what the user sees — it just lets
    // the query fold the same way. See `search_fold`.
    let label = snip.label.as_deref().map(search_fold);
    let monitor_label = snip.monitor_label.as_deref().map(search_fold);
    let ocr_text = search_fold(ocr_text);
    let window_title = window.title.as_deref().map(search_fold);
    let window_app = window.app_name.as_deref().map(search_fold);
    let window_process = window.process_name.as_deref().map(search_fold);
    let window_path = window.exe_path.as_deref().map(search_fold);
    conn.execute("DELETE FROM snip_fts WHERE rowid = ?1", [rowid])?;
    conn.execute(
        "
        INSERT INTO snip_fts(
            rowid, label, monitor_label, dims, ocr_text,
            window_title, window_app, window_process, window_path
        )
        VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        ",
        params![
            rowid,
            label,
            monitor_label,
            dims,
            ocr_text,
            window_title,
            window_app,
            window_process,
            window_path,
        ],
    )?;
    Ok(())
}

pub fn cached_ocr_text(id: Uuid) -> anyhow::Result<Option<String>> {
    with_db(|conn| cached_ocr_text_with_conn(conn, id))
}

#[derive(Debug, Default)]
pub struct SnipMetadataIndex {
    pub ocr_ids: HashSet<Uuid>,
    pub window_ids: HashSet<Uuid>,
}

pub fn metadata_index() -> anyhow::Result<SnipMetadataIndex> {
    with_db(|conn| {
        Ok(SnipMetadataIndex {
            ocr_ids: uuid_set(conn, "SELECT snip_id FROM snip_ocr")?,
            window_ids: uuid_set(conn, "SELECT snip_id FROM snip_window")?,
        })
    })
}

fn uuid_set(conn: &Connection, sql: &str) -> anyhow::Result<HashSet<Uuid>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut set = HashSet::new();
    for row in rows {
        if let Ok(id) = Uuid::parse_str(&row?) {
            set.insert(id);
        }
    }
    Ok(set)
}

pub fn save_window_info(id: Uuid, info: &WindowInfo) -> anyhow::Result<()> {
    with_db(|conn| {
        conn.execute(
            "
            INSERT INTO snip_window(
                snip_id, platform, window_id, title, app_name, process_name, pid, exe_path,
                x, y, w, h, confidence, updated_at
            )
            VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
            ON CONFLICT(snip_id) DO UPDATE SET
                platform=excluded.platform,
                window_id=excluded.window_id,
                title=excluded.title,
                app_name=excluded.app_name,
                process_name=excluded.process_name,
                pid=excluded.pid,
                exe_path=excluded.exe_path,
                x=excluded.x,
                y=excluded.y,
                w=excluded.w,
                h=excluded.h,
                confidence=excluded.confidence,
                updated_at=excluded.updated_at
            ",
            params![
                id.to_string(),
                info.platform.as_str(),
                info.window_id.as_str(),
                info.title.as_deref(),
                info.app_name.as_deref(),
                info.process_name.as_deref(),
                info.pid.map(|pid| pid as i64),
                info.exe_path.as_deref(),
                info.bounds.x,
                info.bounds.y,
                info.bounds.w as i64,
                info.bounds.h as i64,
                info.confidence as f64,
                Utc::now().to_rfc3339(),
            ],
        )?;
        match load_snip_by_id(conn, id) {
            Ok(Some(snip)) => {
                if let Err(err) = upsert_fts_for_snip(conn, &snip, None) {
                    log::debug!("snip FTS window update skipped for {id}: {err}");
                }
            }
            Ok(None) => {}
            Err(err) => {
                log::debug!("snip lookup after window metadata save failed for {id}: {err}");
            }
        }
        Ok(())
    })
}

/// Store a snip's OCR text — only while the snip still exists. OCR runs on a
/// detached thread and can finish after the user deleted the snip; the text of
/// a deleted snip must not be written back. Returns the rows written, 0 for a
/// snip that is gone.
fn insert_ocr(
    conn: &Connection,
    id: Uuid,
    engine: &str,
    language: Option<&str>,
    text: &str,
) -> rusqlite::Result<usize> {
    conn.execute(
        "
        INSERT INTO snip_ocr(snip_id, engine, language, text, updated_at)
        SELECT ?1, ?2, ?3, ?4, ?5
        WHERE EXISTS (SELECT 1 FROM snips WHERE id = ?1)
        ON CONFLICT(snip_id) DO UPDATE SET
            engine=excluded.engine,
            language=excluded.language,
            text=excluded.text,
            updated_at=excluded.updated_at
        ",
        params![
            id.to_string(),
            engine,
            language,
            text,
            Utc::now().to_rfc3339(),
        ],
    )
}

pub fn save_ocr_text(
    id: Uuid,
    engine: &str,
    language: Option<&str>,
    text: &str,
) -> anyhow::Result<()> {
    with_db(|conn| {
        insert_ocr(conn, id, engine, language, text)?;
        match load_snip_by_id(conn, id) {
            Ok(Some(snip)) => {
                if let Err(err) = upsert_fts_for_snip(conn, &snip, Some(text)) {
                    log::debug!("snip FTS OCR update skipped for {id}: {err}");
                }
            }
            Ok(None) => {}
            Err(err) => {
                log::debug!("snip lookup after OCR save failed for {id}: {err}");
            }
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn memory_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", true).unwrap();
        register_search_functions(&conn).unwrap();
        init_db(&conn).unwrap();
        conn
    }

    /// Point this test thread's `with_db` at a fresh in-memory database, so
    /// nothing here reaches the real gallery. An in-memory database belongs
    /// to its one connection, so the checks below go through `with_db` too.
    /// It also counts the rows written to `snips`.
    fn use_memory_db() {
        DB.with(|cell| *cell.borrow_mut() = Some(memory_db()));
        with_db(|conn| {
            conn.execute_batch(
                "CREATE TABLE writes(n INTEGER NOT NULL);
                 INSERT INTO writes VALUES (0);
                 CREATE TRIGGER count_inserts AFTER INSERT ON snips
                     BEGIN UPDATE writes SET n = n + 1; END;
                 CREATE TRIGGER count_updates AFTER UPDATE ON snips
                     BEGIN UPDATE writes SET n = n + 1; END;",
            )?;
            Ok(())
        })
        .unwrap();
    }

    /// Rows written to `snips` so far, inserts and updates.
    fn writes() -> i64 {
        with_db(|conn| Ok(conn.query_row("SELECT n FROM writes", [], |r| r.get(0))?)).unwrap()
    }

    fn fts_rows() -> i64 {
        with_db(|conn| Ok(conn.query_row("SELECT COUNT(*) FROM snip_fts", [], |r| r.get(0))?))
            .unwrap()
    }

    #[test]
    fn save_writes_only_what_changed() {
        use_memory_db();
        let mut gallery = GalleryState::default();
        for _ in 0..3 {
            gallery.push_new(make_snip(ScreenRect::new(0, 0, 10, 10)));
        }
        gallery.save().unwrap();
        assert_eq!(writes(), 3);
        assert_eq!(fts_rows(), 3);

        // Nothing changed: nothing written.
        gallery.save().unwrap();
        assert_eq!(writes(), 3);

        // One changed, however it was changed: one written.
        let id = gallery.snips[1].id;
        gallery.get_mut(id).unwrap().pinned = true;
        gallery.save().unwrap();
        assert_eq!(writes(), 4);
        assert_eq!(fts_rows(), 3);
    }

    #[test]
    fn search_finds_a_saved_snip_and_forgets_a_removed_one() {
        use_memory_db();
        let mut gallery = GalleryState::default();
        let mut snip = make_snip(ScreenRect::new(0, 0, 10, 10));
        snip.label = Some("Merhaba dünya".into());
        let id = snip.id;
        gallery.push_new(snip);
        gallery.push_new(make_snip(ScreenRect::new(0, 0, 20, 20)));
        gallery.save().unwrap();

        let found = gallery.search("dunya").unwrap();
        assert_eq!(found.iter().map(|s| s.id).collect::<Vec<_>>(), vec![id]);

        gallery.remove(id);
        assert!(gallery.search("dunya").unwrap().is_empty());
        assert_eq!(fts_rows(), 1);
    }

    /// An index from before the rowid layout is dropped and rebuilt.
    #[test]
    fn an_id_keyed_search_index_is_rebuilt() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE meta(key TEXT PRIMARY KEY, value TEXT NOT NULL);
             CREATE VIRTUAL TABLE snip_fts USING fts5(snip_id UNINDEXED, label, window_title);",
        )
        .unwrap();
        init_fts(&conn).unwrap();
        let columns = table_columns(&conn, "snip_fts").unwrap();
        assert!(!columns.iter().any(|c| c == "snip_id"), "{columns:?}");
        assert!(columns.iter().any(|c| c == "window_title"));
    }

    fn ocr_rows(conn: &Connection, id: Uuid) -> i64 {
        conn.query_row(
            "SELECT COUNT(*) FROM snip_ocr WHERE snip_id = ?1",
            [id.to_string()],
            |r| r.get(0),
        )
        .unwrap()
    }

    /// A late OCR thread for a snip the user already deleted writes nothing.
    #[test]
    fn ocr_for_a_deleted_snip_is_not_stored() {
        let conn = memory_db();
        let gone = Uuid::new_v4();
        assert_eq!(insert_ocr(&conn, gone, "test", None, "secret").unwrap(), 0);
        assert_eq!(ocr_rows(&conn, gone), 0);
    }

    /// With foreign keys enforced, deleting the snip takes its OCR text too.
    #[test]
    fn deleting_a_snip_cascades_to_its_ocr_text() {
        let conn = memory_db();
        let snip = make_snip(ScreenRect::new(0, 0, 10, 10));
        upsert_snips(&conn, std::slice::from_ref(&snip)).unwrap();
        assert_eq!(insert_ocr(&conn, snip.id, "test", None, "hello").unwrap(), 1);
        conn.execute("DELETE FROM snips WHERE id = ?1", [snip.id.to_string()]).unwrap();
        assert_eq!(ocr_rows(&conn, snip.id), 0);
    }

    #[test]
    fn search_fold_bridges_dotless_and_dotted_i() {
        // The whole point: "ışık" and "isik" must fold to the same thing
        // so OCR'd Turkish text is findable with a plain ASCII keyboard.
        assert_eq!(search_fold("ışık"), search_fold("isik"));
        assert_eq!(search_fold("AÇIK"), search_fold("açık"));
        assert_eq!(search_fold("İstanbul"), search_fold("istanbul"));
        assert_eq!(search_fold("GARANTİ"), search_fold("garanti"));
    }

    #[test]
    fn search_fold_strips_turkish_diacritics_to_ascii() {
        assert_eq!(search_fold("Büyükşehir"), "buyuksehir");
        assert_eq!(search_fold("ŞİŞE GÖZ AĞAÇ"), "sise goz agac");
        assert_eq!(search_fold("kırmızı"), "kirmizi");
    }

    #[test]
    fn search_fold_lowercases_plain_ascii_and_keeps_separators() {
        assert_eq!(search_fold("Hello World"), "hello world");
        assert_eq!(search_fold("1920x1080"), "1920x1080");
    }

    #[test]
    fn fts_match_query_folds_and_prefixes_tokens() {
        assert_eq!(fts_match_query("ışık saç").as_deref(), Some("isik* AND sac*"));
        // No alphanumerics → no FTS query (caller falls back to LIKE).
        assert_eq!(fts_match_query("   "), None);
    }
}

#[cfg(test)]
mod like_tests {
    use super::escape_like;

    /// `%` and `_` are LIKE's wildcards. Unescaped, searching "100%" matched
    /// every snip containing "100" and "a_b" matched "axb".
    #[test]
    fn wildcards_are_neutralised() {
        assert_eq!(escape_like("100%"), "100\\%");
        assert_eq!(escape_like("a_b"), "a\\_b");
    }

    /// Backslash has to be escaped first, or the escapes added for `%` and `_`
    /// get escaped in turn and the pattern means something else again.
    #[test]
    fn a_literal_backslash_survives() {
        assert_eq!(escape_like("a\\b"), "a\\\\b");
        assert_eq!(escape_like("\\%"), "\\\\\\%");
    }

    #[test]
    fn ordinary_text_is_untouched() {
        assert_eq!(escape_like("ekran görüntüsü"), "ekran görüntüsü");
    }
}
