//! Snip image files: choosing the format, writing, exporting and deleting —
//! the one place that touches them on disk.
//!
//! Format
//! ------
//! Snips are stored as lossless WebP. Screenshots are text, flat colour and
//! sharp edges, which is exactly what WebP's lossless mode is good at: on a
//! real gallery of 126 screenshots the PNGs this app used to write came to
//! 10.7 MB and lossless WebP to 2.9 MB, every one decoded back pixel for pixel
//! identical. A lossy format would have gone smaller, but it smears the edges
//! of text — what OCR reads and what people snip for — and every rotate, crop
//! or annotation re-encodes the snip, so the damage would compound. The stored
//! copy is the only copy; it stays lossless.
//!
//! WebP cannot hold an image wider or taller than 16383 pixels. A capture
//! spanning several monitors, or a large collage, can exceed that; those are
//! written as PNG, and everything reading snips accepts either.
//!
//! Writing
//! -------
//! Every write is atomic (see `atomic_file`). A WebP is first written with the
//! fastest lossless setting, so a snip is on disk in milliseconds even when it
//! is large, then a background worker re-encodes it at the setting that makes
//! it small. That is a second writer racing the first, so every path carries a
//! generation number: each write and each delete bumps it, and a background
//! result is only renamed into place if the generation is still the one it
//! started from — checked and renamed under the same lock the writers bump it
//! under. Without that, a slow pass over an old version of a snip lands on top
//! of the crop or rotation made since, and a pass that outlives a delete puts
//! the deleted image back.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Mutex, OnceLock};

use anyhow::{anyhow, Context};
use image::{DynamicImage, ImageEncoder, ImageFormat, RgbaImage};

use crate::atomic_file;

/// Lossless WebP effort, as libwebp's `method`. Measured on the same 126
/// screenshots: 0 gives 59% of the old PNG size at 8 ms a snip, 2 and 4 give
/// 27% at 40–55 ms, 6 gives 25% at half a second. 0 for the write the user
/// waits on, 4 for the background pass.
const FAST_METHOD: i32 = 0;
const SMALL_METHOD: i32 = 4;

/// The largest width or height a WebP can hold.
const WEBP_MAX_DIM: u32 = 16383;

/// Generation per path; see the module comment.
static GENERATIONS: Mutex<Option<HashMap<PathBuf, u64>>> = Mutex::new(None);

/// The file name a snip image of this size is stored under: `<stem>.webp`, or
/// `<stem>.png` for one too large for WebP.
pub fn file_name(stem: &str, image: &RgbaImage) -> String {
    if fits_webp(image) {
        format!("{stem}.webp")
    } else {
        format!("{stem}.png")
    }
}

fn fits_webp(image: &RgbaImage) -> bool {
    image.width() <= WEBP_MAX_DIM && image.height() <= WEBP_MAX_DIM
}

/// Write a snip image in the format its extension names — `.webp` (then
/// shrunk in the background) or `.png`.
pub fn write(path: &Path, image: &RgbaImage) -> anyhow::Result<()> {
    if is_webp(path) {
        let bytes = encode_webp(image, FAST_METHOD)?;
        let generation = commit_bumping(path, &bytes)?;
        queue_shrink(Job {
            path: path.to_path_buf(),
            generation,
            image: image.clone(),
        });
        Ok(())
    } else {
        let temp = atomic_file::write_temp(path, |w| {
            image::codecs::png::PngEncoder::new(w).write_image(
                image.as_raw(),
                image.width(),
                image.height(),
                image::ColorType::Rgba8.into(),
            )?;
            Ok(())
        })?;
        let mut guard = lock();
        bump(&mut guard, path);
        temp.commit()
    }
}

/// Write `image` to `path` as small lossless WebP, but only if no one has
/// written or deleted `path` since `generation` was read. For work that runs
/// in the background over files other code may touch meanwhile — converting
/// old PNG snips. Returns whether it was written.
pub fn write_webp_if_unchanged(path: &Path, image: &RgbaImage, generation: u64) -> anyhow::Result<bool> {
    let bytes = encode_webp(image, SMALL_METHOD)?;
    let temp = atomic_file::write_temp(path, |w| Ok(std::io::Write::write_all(w, &bytes)?))?;
    let mut guard = lock();
    if current(&guard, path) != generation {
        return Ok(false);
    }
    bump(&mut guard, path);
    temp.commit()?;
    Ok(true)
}

/// The generation `path` is at now; see [`write_webp_if_unchanged`].
pub fn generation(path: &Path) -> u64 {
    current(&lock(), path)
}

/// Delete a snip image. Bumps the generation first, so background work still
/// in flight for it discards its result instead of recreating the file.
pub fn remove(path: &Path) {
    let mut guard = lock();
    bump(&mut guard, path);
    let _ = std::fs::remove_file(path);
}

/// Save a copy of `image` wherever the user chose, in the format its extension
/// names. Not a snip file: no generation, no background pass.
///
/// JPEG has no alpha channel, and the encoder refuses RGBA outright — it used
/// to fail only after the destination had already been truncated, turning
/// "Save as photo.jpg" over an existing photo into a 0-byte file. The image is
/// flattened to RGB for it, and the write is atomic like every other.
pub fn export(path: &Path, image: &RgbaImage) -> anyhow::Result<()> {
    let format = ImageFormat::from_path(path)
        .with_context(|| format!("unsupported image type: {}", path.display()))?;
    if format == ImageFormat::WebP {
        let bytes = encode_webp(image, SMALL_METHOD)?;
        return atomic_file::replace(path, |w| Ok(std::io::Write::write_all(w, &bytes)?));
    }
    let image = DynamicImage::ImageRgba8(image.clone());
    let image = match format {
        ImageFormat::Jpeg => DynamicImage::ImageRgb8(image.to_rgb8()),
        _ => image,
    };
    atomic_file::replace(path, |w| Ok(image.write_to(w, format)?))
}

fn is_webp(path: &Path) -> bool {
    path.extension().is_some_and(|e| e.eq_ignore_ascii_case("webp"))
}

/// Lossless WebP through libwebp. `exact` keeps the colour of fully
/// transparent pixels too, so a decode gives back exactly what went in.
fn encode_webp(image: &RgbaImage, method: i32) -> anyhow::Result<Vec<u8>> {
    if !fits_webp(image) {
        return Err(anyhow!(
            "{}x{} is larger than WebP can hold",
            image.width(),
            image.height()
        ));
    }
    let mut config = webp::WebPConfig::new().map_err(|_| anyhow!("libwebp: no config"))?;
    config.lossless = 1;
    config.quality = 100.0;
    config.method = method;
    config.exact = 1;
    let encoded = webp::Encoder::from_rgba(image.as_raw(), image.width(), image.height())
        .encode_advanced(&config)
        .map_err(|e| anyhow!("libwebp could not encode: {e:?}"))?;
    Ok(encoded.to_vec())
}

/// Write `bytes` to `path` atomically, bumping its generation under the lock.
fn commit_bumping(path: &Path, bytes: &[u8]) -> anyhow::Result<u64> {
    let temp = atomic_file::write_temp(path, |w| Ok(std::io::Write::write_all(w, bytes)?))?;
    let mut guard = lock();
    let generation = bump(&mut guard, path);
    temp.commit()?;
    Ok(generation)
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

/// A background re-encode: the pixels it was written from, and the generation
/// that write produced.
struct Job {
    path: PathBuf,
    generation: u64,
    image: RgbaImage,
}

/// One worker thread for every re-encode: a collage or a thumbnail rebuild
/// writes dozens of files at once, and they need not all run at the same time.
fn queue_shrink(job: Job) {
    static WORKER: OnceLock<Option<Mutex<Sender<Job>>>> = OnceLock::new();
    let worker = WORKER.get_or_init(|| {
        let (tx, rx) = channel::<Job>();
        std::thread::Builder::new()
            .name("webp-shrink".into())
            .spawn(move || {
                for job in rx {
                    if let Err(err) = shrink(&job) {
                        log::debug!("re-encoding {} skipped: {err:#}", job.path.display());
                    }
                }
            })
            .ok()?;
        Some(Mutex::new(tx))
    });
    if let Some(tx) = worker {
        let _ = tx.lock().unwrap_or_else(|e| e.into_inner()).send(job);
    }
}

fn shrink(job: &Job) -> anyhow::Result<()> {
    // A newer write is already queued behind this one, or the file is gone.
    if generation(&job.path) != job.generation {
        return Ok(());
    }
    let bytes = encode_webp(&job.image, SMALL_METHOD)?;
    let temp = atomic_file::write_temp(&job.path, |w| Ok(std::io::Write::write_all(w, &bytes)?))?;
    let guard = lock();
    let still_current = current(&guard, &job.path) == job.generation;
    let smaller = std::fs::metadata(&job.path).is_ok_and(|m| (bytes.len() as u64) < m.len());
    if still_current && smaller {
        temp.commit()?;
    }
    Ok(())
}

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
        RgbaImage::from_fn(40, 30, |x, y| {
            image::Rgba([(x * 6) as u8, (y * 8) as u8, 90, if x == 0 { 0 } else { 255 }])
        })
    }

    /// The whole reason for the format: what comes back is what went in,
    /// including the colour under a fully transparent pixel.
    #[test]
    fn webp_round_trips_pixel_for_pixel() {
        let dir = scratch_dir("roundtrip");
        let path = dir.join("snip.webp");
        let img = sample();
        write(&path, &img).unwrap();
        assert_eq!(image::open(&path).unwrap().to_rgba8(), img);
        let small = encode_webp(&img, SMALL_METHOD).unwrap();
        assert_eq!(image::load_from_memory(&small).unwrap().to_rgba8(), img);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn too_large_for_webp_is_named_png() {
        let wide = RgbaImage::new(WEBP_MAX_DIM + 1, 1);
        assert_eq!(file_name("x", &wide), "x.png");
        assert_eq!(file_name("x", &sample()), "x.webp");
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
        assert_eq!((back.width(), back.height()), (40, 30));
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
        let path = dir.join("a.webp");
        write(&path, &sample()).unwrap();
        let after_write = generation(&path);
        write(&path, &sample()).unwrap();
        assert_eq!(generation(&path), after_write + 1);
        remove(&path);
        assert_eq!(generation(&path), after_write + 2);
        assert!(!path.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Background conversion must lose to anything that touched the file
    /// after it started.
    #[test]
    fn a_stale_conversion_does_not_land() {
        let dir = scratch_dir("stale");
        let path = dir.join("b.webp");
        let before = generation(&path);
        write(&path, &sample()).unwrap(); // an edit lands meanwhile
        assert!(!write_webp_if_unchanged(&path, &RgbaImage::new(5, 5), before).unwrap());
        assert_eq!(image::open(&path).unwrap().to_rgba8(), sample());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
