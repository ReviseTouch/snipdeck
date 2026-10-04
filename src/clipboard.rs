//! The system clipboard.
//!
//! One `Clipboard` for the whole process, not one per copy. On Linux arboard
//! serves the copied data from a hidden X11 window for as long as a
//! `Clipboard` exists, and dropping the last one destroys that window after
//! asking a clipboard manager to take the data — a request nothing is obliged
//! to honour. Without a clipboard manager (sway, Hyprland, i3) a copy was gone
//! the moment the function that made it returned; on GNOME it survived only
//! because Mutter keeps a copy of its own. Keeping the instance alive keeps the
//! data served until the next copy replaces it. On Windows and macOS arboard
//! opens and closes the system clipboard per call, so one long-lived instance
//! changes nothing there.

use std::sync::Mutex;

use anyhow::Context;
use arboard::{Clipboard, ImageData};
use image::RgbaImage;

static CLIPBOARD: Mutex<Option<Clipboard>> = Mutex::new(None);

/// Run `f` on the process-wide clipboard, opening it on first use. A failed
/// call drops the instance, so the next one starts from a fresh connection
/// rather than a broken one.
fn with_clipboard(f: impl FnOnce(&mut Clipboard) -> Result<(), arboard::Error>) -> anyhow::Result<()> {
    let mut guard = CLIPBOARD.lock().unwrap_or_else(|e| e.into_inner());
    if guard.is_none() {
        *guard = Some(Clipboard::new().context("opening clipboard")?);
    }
    let result = f(guard.as_mut().expect("clipboard opened above"));
    if result.is_err() {
        *guard = None;
    }
    Ok(result?)
}

pub fn copy_image(img: &RgbaImage) -> anyhow::Result<()> {
    let data = ImageData {
        width: img.width() as usize,
        height: img.height() as usize,
        bytes: img.as_raw().clone().into(),
    };
    with_clipboard(|cb| cb.set_image(data)).context("setting clipboard image")
}

pub fn copy_text(text: &str) -> anyhow::Result<()> {
    with_clipboard(|cb| cb.set_text(text.to_string())).context("setting clipboard text")
}
