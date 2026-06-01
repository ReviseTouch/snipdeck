use anyhow::Context;
use arboard::{Clipboard, ImageData};
use image::RgbaImage;

pub fn copy_image(img: &RgbaImage) -> anyhow::Result<()> {
    let mut cb = Clipboard::new().context("opening clipboard")?;
    let data = ImageData {
        width: img.width() as usize,
        height: img.height() as usize,
        bytes: img.as_raw().clone().into(),
    };
    cb.set_image(data).context("setting clipboard image")?;
    Ok(())
}

pub fn copy_text(text: &str) -> anyhow::Result<()> {
    let mut cb = Clipboard::new().context("opening clipboard")?;
    cb.set_text(text.to_string())
        .context("setting clipboard text")?;
    Ok(())
}
