//! Snip image files: writing, exporting, deleting, and the optional pngquant
//! pass — the one place that touches them on disk.
//!
//! Every write is atomic (see `atomic_file`): a snip's PNG is its only copy,
//! and rotate, crop and annotate all rewrite it in place.
//!
//! pngquant
//! --------
//! Where pngquant is installed, each PNG written here is handed to it
//! afterwards to shrink the file. That is a second writer racing the first, so
//! every path carries a generation number: each write and each delete bumps
//! it, and a pngquant result is only renamed into place if the generation is
//! still the one it started from — checked and renamed under the same lock the
//! writers bump it under. Without that, a slow pngquant run on an old version
//! of a snip landed on top of the crop or rotation made since, and a run that
//! outlived a delete put the deleted image back.
//!
//! One worker thread, not a thread per write: `--speed 1` is slow on a large
//! image, and a collage or a thumbnail rebuild writes dozens of files at once.
//!
//! pngquant is looked up on `PATH` only, and never run from an elevated or
//! root process: an executable found by a fixed path or in a user-writable
//! directory is not something to run with someone else's rights.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Mutex, OnceLock};

use anyhow::Context;
use image::{DynamicImage, ImageEncoder, ImageFormat, RgbaImage};

use crate::atomic_file;

/// Generation per path; see the module comment.
static GENERATIONS: Mutex<Option<HashMap<PathBuf, u64>>> = Mutex::new(None);

/// Write `image` to `path` as PNG, then queue the pngquant pass.
pub fn write_png(path: &Path, image: &RgbaImage) -> anyhow::Result<()> {
    let temp = atomic_file::write_temp(path, |w| {
        image::codecs::png::PngEncoder::new_with_quality(
            w,
            image::codecs::png::CompressionType::Fast,
            image::codecs::png::FilterType::Adaptive,
        )
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            image::ColorType::Rgba8.into(),
        )?;
        Ok(())
    })?;
    let generation = {
        let mut guard = lock();
        let generation = bump(&mut guard, path);
        temp.commit()?;
        generation
    };
    queue_pngquant(path.to_path_buf(), generation);
    Ok(())
}

/// Delete a snip image. Bumps the generation first, so a pngquant run still
/// in flight for it discards its result instead of recreating the file.
pub fn remove(path: &Path) {
    let mut guard = lock();
    bump(&mut guard, path);
    let _ = std::fs::remove_file(path);
}

/// Save a copy of `image` wherever the user chose, in the format its extension
/// names. Not a snip file: no generation, no pngquant.
///
/// JPEG has no alpha channel, and the encoder refuses RGBA outright — it used
/// to fail only after the destination had already been truncated, turning
/// "Save as photo.jpg" over an existing photo into a 0-byte file. The image is
/// flattened to RGB for it, and the write is atomic like every other.
pub fn export(path: &Path, image: &RgbaImage) -> anyhow::Result<()> {
    let format = ImageFormat::from_path(path)
        .with_context(|| format!("unsupported image type: {}", path.display()))?;
    let image = DynamicImage::ImageRgba8(image.clone());
    let image = match format {
        ImageFormat::Jpeg => DynamicImage::ImageRgb8(image.to_rgb8()),
        _ => image,
    };
    atomic_file::replace(path, |w| Ok(image.write_to(w, format)?))
}

fn lock() -> std::sync::MutexGuard<'static, Option<HashMap<PathBuf, u64>>> {
    // A panic while holding this lock cannot leave the map inconsistent — every
    // critical section is a single insert — so a poisoned lock is still usable.
    GENERATIONS.lock().unwrap_or_else(|e| e.into_inner())
}

fn bump(map: &mut Option<HashMap<PathBuf, u64>>, path: &Path) -> u64 {
    let generation = map.get_or_insert_with(HashMap::new).entry(path.to_path_buf()).or_insert(0);
    *generation += 1;
    *generation
}

fn current(map: &Option<HashMap<PathBuf, u64>>, path: &Path) -> u64 {
    map.as_ref().and_then(|m| m.get(path)).copied().unwrap_or(0)
}

struct Job {
    path: PathBuf,
    generation: u64,
}

fn queue_pngquant(path: PathBuf, generation: u64) {
    static WORKER: OnceLock<Option<Mutex<Sender<Job>>>> = OnceLock::new();
    let worker = WORKER.get_or_init(|| {
        let pngquant = pngquant_command()?;
        let (tx, rx) = channel::<Job>();
        std::thread::Builder::new()
            .name("pngquant".into())
            .spawn(move || {
                for job in rx {
                    if let Err(err) = run_job(&pngquant, &job) {
                        log::debug!("pngquant skipped for {}: {err:#}", job.path.display());
                    }
                }
            })
            .ok()?;
        Some(Mutex::new(tx))
    });
    if let Some(tx) = worker {
        let _ = tx
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .send(Job { path, generation });
    }
}

fn run_job(pngquant: &Path, job: &Job) -> anyhow::Result<()> {
    // A newer write is already queued behind this one, or the file is gone.
    if current(&lock(), &job.path) != job.generation {
        return Ok(());
    }
    let name = job
        .path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "snip.png".into());
    // Named for the generation, so two runs can never share an output file.
    let out = job
        .path
        .with_file_name(format!(".{name}.pngquant-{}.snipdeck-tmp", job.generation));
    let mut cmd = Command::new(pngquant);
    hide_console_window(&mut cmd);
    let status = cmd
        .args(["--force", "--skip-if-larger", "--strip", "--speed", "1", "--quality", "0-100"])
        .arg("--output")
        .arg(&out)
        .arg(&job.path)
        .status()
        .context("running pngquant")?;

    let guard = lock();
    let still_current = current(&guard, &job.path) == job.generation;
    let smaller = status.success()
        && matches!(
            (std::fs::metadata(&out), std::fs::metadata(&job.path)),
            (Ok(q), Ok(o)) if q.len() < o.len()
        );
    if still_current && smaller {
        std::fs::rename(&out, &job.path)?;
    } else {
        let _ = std::fs::remove_file(&out);
    }
    Ok(())
}

/// pngquant on `PATH`, if there is one and it is safe to run.
fn pngquant_command() -> Option<PathBuf> {
    if running_with_raised_rights() {
        log::info!("pngquant disabled: not running external tools with raised rights");
        return None;
    }
    let candidate = PathBuf::from(if cfg!(windows) { "pngquant.exe" } else { "pngquant" });
    let mut cmd = Command::new(&candidate);
    hide_console_window(&mut cmd);
    let found = cmd
        .arg("--version")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false);
    found.then_some(candidate)
}

fn running_with_raised_rights() -> bool {
    #[cfg(unix)]
    {
        // SAFETY: geteuid has no preconditions and cannot fail.
        if unsafe { libc::geteuid() } == 0 {
            return true;
        }
    }
    crate::elevate::is_elevated()
}

#[cfg(target_os = "windows")]
fn hide_console_window(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(target_os = "windows"))]
fn hide_console_window(_cmd: &mut Command) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("snipdeck-images-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn sample() -> RgbaImage {
        RgbaImage::from_fn(8, 6, |x, y| image::Rgba([x as u8 * 30, y as u8 * 40, 90, 255]))
    }

    /// The bug: "Save as" with a .jpg name produced a 0-byte file and destroyed
    /// whatever it was saving over.
    #[test]
    fn export_writes_a_real_jpeg_over_an_existing_file() {
        let dir = scratch_dir("jpeg");
        let path = dir.join("photo.jpg");
        std::fs::write(&path, b"precious").unwrap();
        export(&path, &sample()).unwrap();
        let back = image::open(&path).unwrap();
        assert_eq!((back.width(), back.height()), (8, 6));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn export_refuses_an_unknown_type_without_touching_the_file() {
        let dir = scratch_dir("unknown");
        let path = dir.join("notes.xyz");
        std::fs::write(&path, b"keep me").unwrap();
        assert!(export(&path, &sample()).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"keep me");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn every_write_and_delete_moves_the_generation_on() {
        let dir = scratch_dir("gen");
        let path = dir.join("a.png");
        write_png(&path, &sample()).unwrap();
        let after_write = current(&lock(), &path);
        write_png(&path, &sample()).unwrap();
        assert_eq!(current(&lock(), &path), after_write + 1);
        remove(&path);
        assert_eq!(current(&lock(), &path), after_write + 2);
        assert!(!path.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
