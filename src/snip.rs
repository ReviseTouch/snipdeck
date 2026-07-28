use anyhow::Context;
use chrono::{DateTime, Utc};
use image::ImageEncoder;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::process::Command;
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
        write_png_best_effort(&path, image)
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
        write_png_best_effort(&path, &thumb)
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
        write_png_best_effort(&self.thumb_path(), &thumb)?;
        Ok(thumb)
    }

    pub fn delete_files(&self) {
        let _ = std::fs::remove_file(self.image_path());
        let _ = std::fs::remove_file(self.thumb_path());
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

    pub fn remove(&mut self, id: Uuid) -> Option<Snip> {
        let pos = self.snips.iter().position(|s| s.id == id)?;
        let snip = self.snips.remove(pos);
        snip.delete_files();
        if let Err(err) = delete_snip_from_db(id) {
            log::warn!("deleting snip from db failed: {err}");
        }
        Some(snip)
    }

    pub fn clear_all(&mut self) {
        for snip in &self.snips {
            snip.delete_files();
        }
        self.snips.clear();
        if let Err(err) = clear_db_snips() {
            log::warn!("clearing snip db failed: {err}");
        }
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

fn search_like(conn: &Connection, query: &str) -> anyhow::Result<Vec<Snip>> {
    // Fold the query the same way as the indexed columns (see `tr_fold`)
    // so the fallback is case-, diacritic- and dotless-`ı` insensitive too.
    let like = format!("%{}%", search_fold(query));
    let mut stmt = conn.prepare(
        "SELECT DISTINCT
                s.id, s.created_at, s.x, s.y, s.w, s.h, s.image_file, s.thumb_file,
                s.label, s.pinned, s.monitor_label, s.floating, s.last_x, s.last_y
         FROM snips s
         LEFT JOIN snip_ocr o ON o.snip_id = s.id
         LEFT JOIN snip_window win ON win.snip_id = s.id
         WHERE tr_fold(s.label) LIKE ?1
            OR tr_fold(s.monitor_label) LIKE ?1
            OR tr_fold(o.text) LIKE ?1
            OR tr_fold(win.title) LIKE ?1
            OR tr_fold(win.app_name) LIKE ?1
            OR tr_fold(win.process_name) LIKE ?1
            OR tr_fold(win.exe_path) LIKE ?1
            OR (CAST(s.w AS TEXT) || 'x' || CAST(s.h AS TEXT)) LIKE ?1
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

fn write_png_best_effort(path: &Path, image: &image::RgbaImage) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    let encoder = image::codecs::png::PngEncoder::new_with_quality(
        writer,
        image::codecs::png::CompressionType::Fast,
        image::codecs::png::FilterType::Adaptive,
    );
    encoder.write_image(
        image.as_raw(),
        image.width(),
        image.height(),
        image::ColorType::Rgba8.into(),
    )?;
    queue_pngquant(path.to_path_buf());
    Ok(())
}

fn queue_pngquant(path: PathBuf) {
    std::thread::spawn(move || {
        if let Err(err) = pngquant_best_effort(&path) {
            log::debug!("pngquant compression skipped for {}: {err}", path.display());
        }
    });
}

fn pngquant_best_effort(path: &Path) -> anyhow::Result<()> {
    let Some(pngquant) = pngquant_command() else {
        return Ok(());
    };
    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("snip.png");
    let tmp = path.with_file_name(format!("{file_name}.pngquant.tmp.png"));
    let mut cmd = Command::new(pngquant);
    hide_console_window(&mut cmd);
    let status = cmd
        .arg("--force")
        .arg("--skip-if-larger")
        .arg("--strip")
        .arg("--speed")
        .arg("1")
        .arg("--quality")
        .arg("0-100")
        .arg("--output")
        .arg(&tmp)
        .arg(path)
        .status()
        .with_context(|| "running pngquant")?;

    if !status.success() || !tmp.exists() {
        let _ = std::fs::remove_file(&tmp);
        return Ok(());
    }

    let original_len = std::fs::metadata(path)?.len();
    let quant_len = std::fs::metadata(&tmp)?.len();
    if quant_len < original_len {
        std::fs::copy(&tmp, path)?;
        let _ = std::fs::remove_file(&tmp);
    } else {
        let _ = std::fs::remove_file(&tmp);
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn hide_console_window(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(target_os = "windows"))]
fn hide_console_window(_cmd: &mut Command) {}

fn pngquant_command() -> Option<&'static PathBuf> {
    static PNGQUANT: OnceLock<Option<PathBuf>> = OnceLock::new();
    PNGQUANT
        .get_or_init(|| {
            let candidates = [
                PathBuf::from("pngquant"),
                PathBuf::from(r"C:\msys64\mingw64\bin\pngquant.exe"),
            ];
            candidates.into_iter().find(|candidate| {
                let mut cmd = Command::new(candidate);
                hide_console_window(&mut cmd);
                cmd.arg("--version")
                    .output()
                    .map(|out| out.status.success())
                    .unwrap_or(false)
            })
        })
        .as_ref()
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
                Err(err) => log::warn!("legacy JSON migration failed: {err}"),
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

pub fn save_ocr_text(
    id: Uuid,
    engine: &str,
    language: Option<&str>,
    text: &str,
) -> anyhow::Result<()> {
    let conn = open_db()?;
    conn.execute(
        "
        INSERT INTO snip_ocr(snip_id, engine, language, text, updated_at)
        VALUES(?1, ?2, ?3, ?4, ?5)
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
    )?;
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
