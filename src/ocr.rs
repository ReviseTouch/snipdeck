//! Windows.Media.Ocr wrapper.
//!
//! Uses the OS's built-in OCR engine (introduced in Windows 8.1, still
//! present and improved on Win11). No external dependency, no model
//! download. Language picked from the user's profile; falls back to
//! en-US if the profile has no OCR-supported language installed.
//!
//! The work is dispatched to a fresh thread initialised as MTA so we
//! don't risk dead-locking on the Slint event loop (which runs the
//! main thread in STA). WinRT async ops also poll the calling thread's
//! message queue from `.get()` — without a clean apartment that can
//! produce hard-to-diagnose access violations, which is what the
//! initial naive implementation hit.
//!
//! Reference implementation studied: the Descolada/AHK OCR class
//! shipped with the legacy Snipper-AHK project. It pre-scales tiny
//! images (<40 px on either axis) before handing them to the engine
//! — we do the same.

#[cfg(target_os = "windows")]
pub fn extract_text(image: &image::RgbaImage) -> anyhow::Result<String> {
    use std::sync::mpsc;

    // WinRT OCR fails on very small images (~< 40 px) — Microsoft
    // doesn't document the exact threshold, but the AHK reference
    // hits errors at <40 and resolves it by up-scaling. We do the
    // same with a Lanczos filter so the result looks reasonable.
    let working = upscale_if_tiny(image);
    let img = working.as_ref().unwrap_or(image);

    // Encode as PNG into memory. PNG decode → SoftwareBitmap path is
    // robust to row-pitch / colour-order differences across GPUs.
    let mut png_bytes: Vec<u8> = Vec::new();
    {
        use anyhow::Context;
        let mut cursor = std::io::Cursor::new(&mut png_bytes);
        image::write_buffer_with_format(
            &mut cursor,
            img.as_raw(),
            img.width(),
            img.height(),
            image::ExtendedColorType::Rgba8,
            image::ImageFormat::Png,
        )
        .context("encoding snip as PNG for OCR")?;
    }

    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        // Wrap in catch_unwind so a panic inside windows-rs surfaces
        // as a normal error rather than aborting the whole process.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            ocr_winrt_blocking(&png_bytes)
        }));
        let send = match result {
            Ok(r) => r,
            Err(_) => Err(anyhow::anyhow!("internal error during OCR")),
        };
        let _ = tx.send(send);
    });
    rx.recv()
        .map_err(|_| anyhow::anyhow!("OCR thread did not respond"))?
}

#[cfg(target_os = "windows")]
pub fn engine_name() -> &'static str {
    "windows-media-ocr"
}

#[cfg(target_os = "macos")]
pub fn engine_name() -> &'static str {
    "macos-vision"
}

#[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
pub fn engine_name() -> &'static str {
    "tesseract"
}

fn upscale_if_tiny(image: &image::RgbaImage) -> Option<image::RgbaImage> {
    let (w, h) = (image.width(), image.height());
    let min = w.min(h);
    if min >= 40 {
        return None;
    }
    let scale = 40.0_f32 / min as f32;
    let nw = ((w as f32) * scale).ceil() as u32;
    let nh = ((h as f32) * scale).ceil() as u32;
    Some(image::imageops::resize(
        image,
        nw,
        nh,
        image::imageops::FilterType::Lanczos3,
    ))
}

#[cfg(target_os = "windows")]
fn ocr_winrt_blocking(png_bytes: &[u8]) -> anyhow::Result<String> {
    use anyhow::{anyhow, Context};
    use windows::core::HSTRING;
    use windows::Globalization::Language;
    use windows::Graphics::Imaging::BitmapDecoder;
    use windows::Media::Ocr::OcrEngine;
    use windows::Storage::Streams::{DataWriter, InMemoryRandomAccessStream};
    use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};

    // Initialise this worker thread's apartment as MTA. RoInitialize
    // would also work but CoInitializeEx is interchangeable for our
    // purposes and the windows-rs feature set already includes Com.
    // Failure (e.g. already-initialised, or a different mode) is fine
    // — we only need *some* apartment, not specifically MTA.
    let com_initialised = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok();

    let result = (|| -> anyhow::Result<String> {
        let stream = InMemoryRandomAccessStream::new().context("creating WinRT stream")?;
        let writer = DataWriter::CreateDataWriter(&stream).context("creating DataWriter")?;
        writer.WriteBytes(png_bytes).context("writing PNG bytes")?;
        writer
            .StoreAsync()
            .context("StoreAsync")?
            .get()
            .context("StoreAsync.get")?;
        writer
            .FlushAsync()
            .context("FlushAsync")?
            .get()
            .context("FlushAsync.get")?;
        let _ = writer.DetachStream();
        stream.Seek(0).context("seek stream")?;

        let decoder = BitmapDecoder::CreateAsync(&stream)
            .context("BitmapDecoder::CreateAsync")?
            .get()
            .context("BitmapDecoder::CreateAsync.get")?;
        let software_bitmap = decoder
            .GetSoftwareBitmapAsync()
            .context("GetSoftwareBitmapAsync")?
            .get()
            .context("GetSoftwareBitmapAsync.get")?;

        // Try the user's profile languages first (so installed TR/EN
        // packs are picked automatically); fall back to en-US.
        let engine = match OcrEngine::TryCreateFromUserProfileLanguages() {
            Ok(e) => e,
            Err(_) => {
                let lang = Language::CreateLanguage(&HSTRING::from("en-US"))
                    .context("creating fallback Language")?;
                OcrEngine::TryCreateFromLanguage(&lang).context("TryCreateFromLanguage(en-US)")?
            }
        };

        let result = engine
            .RecognizeAsync(&software_bitmap)
            .context("RecognizeAsync")?
            .get()
            .context("RecognizeAsync.get")?;
        let text = result.Text().context("OcrResult::Text")?;
        let text = text.to_string_lossy();
        if text.trim().is_empty() {
            return Err(anyhow!("OCR returned empty — no text found in the image"));
        }
        Ok(text)
    })();

    if com_initialised {
        unsafe { CoUninitialize() };
    }
    result
}

#[cfg(target_os = "macos")]
pub fn extract_text(_image: &image::RgbaImage) -> anyhow::Result<String> {
    anyhow::bail!("macOS Vision OCR backend is not wired up yet")
}

/// Linux backend: shell out to the user's own `tesseract` if it is
/// installed (runtime detection — no bundled binaries, no downloads; the
/// distro package is the trusted install path). Absent tesseract, OCR
/// degrades gracefully: callers already treat an Err as "skip indexing".
#[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
pub fn extract_text(image: &image::RgbaImage) -> anyhow::Result<String> {
    use anyhow::Context;
    use std::process::Command;

    // Which language packs are installed? Prefer Turkish+English when
    // present; otherwise let tesseract use its default (eng).
    let langs = Command::new("tesseract")
        .arg("--list-langs")
        .output()
        .context(
            "tesseract not found — install it for OCR (e.g. `sudo apt install tesseract-ocr tesseract-ocr-tur`)",
        )?;
    let installed = String::from_utf8_lossy(&langs.stdout);
    let mut wanted: Vec<&str> = Vec::new();
    for lang in ["tur", "eng"] {
        if installed.lines().any(|l| l.trim() == lang) {
            wanted.push(lang);
        }
    }

    // A snip of one line of interface text is often 20 px tall, and tesseract
    // reads text that small poorly; the Windows path above scales such images
    // up before OCR, and this one now does too.
    let working = upscale_if_tiny(image);
    let image = working.as_ref().unwrap_or(image);

    // The image goes in on stdin, never through a file. A temp PNG in /tmp was
    // readable by every local user (0644) for as long as tesseract ran — the
    // full snip, on the "clipboard only" path too. `stdin` as the input name
    // has been tesseract's way to read a pipe since 4.0.
    let mut png = Vec::new();
    image
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .context("encoding the snip for tesseract")?;
    let mut cmd = Command::new("tesseract");
    cmd.arg("stdin").arg("stdout");
    if !wanted.is_empty() {
        cmd.arg("-l").arg(wanted.join("+"));
    }
    let mut child = cmd
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("running tesseract")?;
    // Feed it from a thread: tesseract may start writing before it has read
    // everything, and a full stdout pipe against a full stdin pipe deadlocks.
    let mut stdin = child.stdin.take().context("tesseract stdin")?;
    let feeder = std::thread::spawn(move || std::io::Write::write_all(&mut stdin, &png));
    let out = child.wait_with_output().context("running tesseract")?;
    let _ = feeder.join();

    if !out.status.success() {
        anyhow::bail!(
            "tesseract failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if text.is_empty() {
        anyhow::bail!("OCR returned empty — no text found in the image");
    }
    Ok(text)
}
