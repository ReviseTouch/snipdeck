use anyhow::Context;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use uuid::Uuid;

use crate::paths;
use crate::window_metadata::WindowInfo;

const THUMB_MAX_DIM: u32 = 256;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

    pub fn save_image_png(&mut self, image: &image::RgbaImage) -> anyhow::Result<()> {
        if self.image_file.is_empty() {
            self.image_file = format!("{}.png", self.id);
        }
        if self.thumb_file.is_none() {
            self.thumb_file = Some(default_thumb_file(self.id));
        }

        let path = self.image_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        crate::image_files::write_png(&path, image)
            .with_context(|| format!("saving snip image to {}", path.display()))?;

        self.save_thumbnail_png(image)
            .with_context(|| format!("saving snip thumbnail for {}", self.id))?;
        Ok(())
    }

    pub fn save_thumbnail_png(&self, image: &image::RgbaImage) -> anyhow::Result<()> {
        let thumb = thumbnail_from(image, THUMB_MAX_DIM);
        let path = self.thumb_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        crate::image_files::write_png(&path, &thumb)
            .with_context(|| format!("saving snip thumbnail to {}", path.display()))?;
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
        crate::image_files::write_png(&self.thumb_path(), &thumb)?;
        Ok(thumb)
    }

    pub fn delete_files(&self) {
        crate::image_files::remove(&self.image_path());
        crate::image_files::remove(&self.thumb_path());
    }
}

pub fn make_snip(area: ScreenRect) -> Snip {
    let id = Uuid::new_v4();
    Snip {
        id,
        created_at: Utc::now(),
        area,
        image_file: format!("{id}.png"),
        thumb_file: Some(default_thumb_file(id)),
        label: None,
        pinned: false,
        monitor_label: None,
        floating: false,
        last_position: None,
    }
}

/// Top-level persisted state for the gallery.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GalleryState {
    /// Newest first.
    pub snips: Vec<Snip>,
}

impl GalleryState {
    pub fn load() -> Self {
        match Self::load_from_db() {
            Ok(state) => state,
            Err(err) => {
                log::warn!("SQLite gallery load failed, falling back to legacy JSON: {err}");
                Self::load_legacy_json(&paths::state_file()).unwrap_or_default()
            }
        }
    }

    fn load_legacy_json(path: &Path) -> anyhow::Result<Self> {
        let bytes = std::fs::read(path)?;
        let state: GalleryState = serde_json::from_slice(&bytes)?;
        Ok(state)
    }

    fn load_from_db() -> anyhow::Result<Self> {
        let conn = open_db()?;
        migrate_legacy_json_if_needed(&conn)?;
        purge_orphans(&conn);
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
        Ok(Self { snips })
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let mut conn = open_db()?;
        let tx = conn.transaction()?;
        upsert_snips(&tx, &self.snips)?;
        tx.commit()?;
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
            Ok(()) => snip.delete_files(),
            Err(err) => log::warn!("deleting snip from db failed, keeping its files: {err}"),
        }
        Some(snip)
    }

    /// Same ordering argument as [`GalleryState::remove`].
    pub fn clear_all(&mut self) {
        match clear_db_snips() {
            Ok(()) => {
                for snip in &self.snips {
                    snip.delete_files();
                }
            }
            Err(err) => log::warn!("clearing snip db failed, keeping the files: {err}"),
        }
        self.snips.clear();
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
        let conn = open_db()?;
        if let Some(match_query) = fts_match_query(trimmed) {
            match search_fts(&conn, &match_query) {
                Ok(snips) => return Ok(snips),
                Err(err) => {
                    log::debug!("SQLite FTS search failed, falling back to LIKE: {err}");
                }
            }
        }
        search_like(&conn, trimmed)
    }
}

fn search_fts(conn: &Connection, match_query: &str) -> anyhow::Result<Vec<Snip>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT
                s.id, s.created_at, s.x, s.y, s.w, s.h, s.image_file, s.thumb_file,
                s.label, s.pinned, s.monitor_label, s.floating, s.last_x, s.last_y
         FROM snip_fts
         JOIN snips s ON s.id = snip_fts.snip_id
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
    format!("{id}.thumb.png")
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
    register_search_functions(&conn)?;
    init_db(&conn)?;
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
        PRAGMA synchronous = NORMAL;
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

fn init_fts(conn: &Connection) -> anyhow::Result<()> {
    if fts_needs_recreate(conn)? {
        conn.execute("DROP TABLE IF EXISTS snip_fts", [])?;
        conn.execute(
            "DELETE FROM meta WHERE key IN ('fts5_index_built_v1', 'fts5_index_built_v2', 'fts5_index_built_v3')",
            [],
        )?;
    }
    conn.execute_batch(
        "
        CREATE VIRTUAL TABLE IF NOT EXISTS snip_fts USING fts5(
            snip_id UNINDEXED,
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

fn fts_needs_recreate(conn: &Connection) -> anyhow::Result<bool> {
    let columns = table_columns(conn, "snip_fts")?;
    Ok(!columns.is_empty() && !columns.iter().any(|col| col == "window_title"))
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
    // v3 re-indexes existing rows through `search_fold` (Turkish-aware
    // folding). Bumping the key forces one rebuild on upgrade.
    let built: Option<String> = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'fts5_index_built_v3'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    if built.as_deref() == Some("1") {
        return Ok(());
    }

    rebuild_fts(conn)?;
    conn.execute(
        "INSERT OR REPLACE INTO meta(key, value) VALUES('fts5_index_built_v3', '1')",
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

fn migrate_legacy_json_if_needed(conn: &Connection) -> anyhow::Result<()> {
    let migrated: Option<String> = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'legacy_json_migrated'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    if migrated.as_deref() == Some("1") {
        return Ok(());
    }

    let count: i64 = conn.query_row("SELECT COUNT(*) FROM snips", [], |row| row.get(0))?;
    if count == 0 {
        let path = paths::state_file();
        if path.exists() {
            match GalleryState::load_legacy_json(&path) {
                Ok(state) => {
                    upsert_snips(conn, &state.snips)?;
                    log::info!(
                        "migrated {} legacy JSON snips into SQLite",
                        state.snips.len()
                    );
                }
                // Deliberately does NOT fall through to the marker below.
                // `state.json` is still there and still the only copy of those
                // snips; a read that failed once — half-written, locked, a
                // transient IO error — is a reason to try again on the next
                // launch, not to conclude the user has no legacy gallery.
                // Marking it migrated here made a single bad read permanent:
                // the snips stayed in the JSON and nothing ever looked at it
                // again.
                Err(err) => {
                    log::warn!("legacy JSON migration failed, retrying next launch: {err}");
                    return Ok(());
                }
            }
        }
    }

    conn.execute(
        "INSERT OR REPLACE INTO meta(key, value) VALUES('legacy_json_migrated', '1')",
        [],
    )?;
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
        "DELETE FROM snip_fts WHERE snip_id NOT IN (SELECT id FROM snips)",
    ] {
        match conn.execute(sql, []) {
            Ok(0) => {}
            Ok(n) => log::info!("removed {n} rows left behind by deleted snips"),
            Err(err) => log::debug!("orphan cleanup skipped: {err}"),
        }
    }
}

fn delete_snip_from_db(id: Uuid) -> anyhow::Result<()> {
    let conn = open_db()?;
    let id_text = id.to_string();
    let _ = conn.execute(
        "DELETE FROM snip_fts WHERE snip_id = ?1",
        [id_text.as_str()],
    );
    conn.execute(
        "DELETE FROM snip_window WHERE snip_id = ?1",
        [id_text.as_str()],
    )?;
    conn.execute(
        "DELETE FROM snip_ocr WHERE snip_id = ?1",
        [id_text.as_str()],
    )?;
    conn.execute("DELETE FROM snips WHERE id = ?1", [id_text.as_str()])?;
    Ok(())
}

fn clear_db_snips() -> anyhow::Result<()> {
    let conn = open_db()?;
    let _ = conn.execute("DELETE FROM snip_fts", []);
    conn.execute("DELETE FROM snip_window", [])?;
    conn.execute("DELETE FROM snip_ocr", [])?;
    conn.execute("DELETE FROM snips", [])?;
    Ok(())
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
    let dims = format!("{}x{}", snip.area.w, snip.area.h);
    let id_text = snip.id.to_string();
    // Store a search-folded copy of every matchable column. `snip_fts` is
    // a pure index (its `snip_id` is UNINDEXED and we JOIN back to `snips`
    // for the displayed rows), so folding here never touches what the user
    // sees — it just lets the query fold the same way. See `search_fold`.
    let label = snip.label.as_deref().map(search_fold);
    let monitor_label = snip.monitor_label.as_deref().map(search_fold);
    let ocr_text = search_fold(ocr_text);
    let window_title = window.title.as_deref().map(search_fold);
    let window_app = window.app_name.as_deref().map(search_fold);
    let window_process = window.process_name.as_deref().map(search_fold);
    let window_path = window.exe_path.as_deref().map(search_fold);
    conn.execute(
        "DELETE FROM snip_fts WHERE snip_id = ?1",
        [id_text.as_str()],
    )?;
    conn.execute(
        "
        INSERT INTO snip_fts(
            snip_id, label, monitor_label, dims, ocr_text,
            window_title, window_app, window_process, window_path
        )
        VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        ",
        params![
            id_text,
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
    let conn = open_db()?;
    cached_ocr_text_with_conn(&conn, id)
}

#[derive(Debug, Default)]
pub struct SnipMetadataIndex {
    pub ocr_ids: HashSet<Uuid>,
    pub window_ids: HashSet<Uuid>,
}

pub fn metadata_index() -> anyhow::Result<SnipMetadataIndex> {
    let conn = open_db()?;
    Ok(SnipMetadataIndex {
        ocr_ids: uuid_set(&conn, "SELECT snip_id FROM snip_ocr")?,
        window_ids: uuid_set(&conn, "SELECT snip_id FROM snip_window")?,
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
    let conn = open_db()?;
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
    match load_snip_by_id(&conn, id) {
        Ok(Some(snip)) => {
            if let Err(err) = upsert_fts_for_snip(&conn, &snip, None) {
                log::debug!("snip FTS window update skipped for {id}: {err}");
            }
        }
        Ok(None) => {}
        Err(err) => {
            log::debug!("snip lookup after window metadata save failed for {id}: {err}");
        }
    }
    Ok(())
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
    let conn = open_db()?;
    insert_ocr(&conn, id, engine, language, text)?;
    match load_snip_by_id(&conn, id) {
        Ok(Some(snip)) => {
            if let Err(err) = upsert_fts_for_snip(&conn, &snip, Some(text)) {
                log::debug!("snip FTS OCR update skipped for {id}: {err}");
            }
        }
        Ok(None) => {}
        Err(err) => {
            log::debug!("snip lookup after OCR save failed for {id}: {err}");
        }
    }
    Ok(())
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
