//! Upload a snip to an image host and copy the link.
//!
//! Three kinds of host: **catbox.moe** (the default — anonymous, no key),
//! **Imgur** (anonymous, needs a free Client-ID in `settings.imgur_client_id`)
//! and any host the user imports as a ShareX custom uploader (`crate::sxcu`).
//! The choice is `settings.upload_service`: `"catbox"`, `"imgur"`, or
//! `"custom:<name>"` naming one of `settings.upload_hosts`.
//!
//! Nothing here asks the user anything. The consent prompt and the result are
//! the upload window's (`crate::upload_window`), on the UI thread; this module
//! only knows where an image goes and how to send it. That split is the fix
//! for a prompt that never appeared: the native dialogs this used to raise
//! need `zenity` on Linux, and without it the consent "answer" was Cancel —
//! nothing uploaded, nothing said.

use anyhow::{anyhow, Context, Result};
use crossbeam_channel::Sender;
use std::time::Duration;

use crate::settings::Settings;
use crate::sxcu::Uploader;

/// Imgur anonymous image-upload endpoint.
const IMGUR_ENDPOINT: &str = "https://api.imgur.com/3/image";
/// catbox.moe anonymous upload endpoint (no API key, no login).
const CATBOX_ENDPOINT: &str = "https://catbox.moe/user/api.php";
/// Fixed multipart boundary. A binary PNG can't contain this ASCII token,
/// so a constant boundary is safe and avoids pulling in an RNG just to
/// randomise it.
const BOUNDARY: &str = "----snipdeckImgurBoundary7e3f1a9c4d";

/// Prefix of `upload_service` for an imported host.
pub const CUSTOM_PREFIX: &str = "custom:";

/// Where an upload goes.
#[derive(Clone, Debug)]
pub enum Host {
    Catbox,
    Imgur { client_id: String },
    Custom(Box<Uploader>),
}

/// How an upload ended, sent from the worker to the UI thread.
pub enum Outcome {
    Done { url: String, copied: bool },
    Failed(String),
}

impl Host {
    /// The host `settings` picks. An unknown name or a custom host that is no
    /// longer in the list falls back to catbox, as before.
    pub fn from_settings(settings: &Settings) -> Host {
        let service = settings.upload_service.trim();
        if service.eq_ignore_ascii_case("imgur") {
            return Host::Imgur { client_id: settings.imgur_client_id.trim().to_string() };
        }
        if let Some(name) = service.strip_prefix(CUSTOM_PREFIX) {
            if let Some(up) = settings.upload_hosts.iter().find(|u| u.display_name() == name) {
                return Host::Custom(Box::new(up.clone()));
            }
            log::warn!("upload host {name:?} is not in upload_hosts; using catbox");
        }
        Host::Catbox
    }

    pub fn name(&self) -> String {
        match self {
            Host::Catbox => "catbox.moe".into(),
            Host::Imgur { .. } => "Imgur".into(),
            Host::Custom(up) => up.display_name(),
        }
    }

    /// The address the image goes to, shown under the name.
    pub fn domain(&self) -> String {
        match self {
            Host::Catbox => "catbox.moe".into(),
            Host::Imgur { .. } => "api.imgur.com".into(),
            Host::Custom(up) => up.domain(),
        }
    }

    /// Known to keep the image out of any public listing. Unknown for an
    /// imported host, so the prompt does not promise it.
    pub fn is_unlisted(&self) -> bool {
        !matches!(self, Host::Custom(_))
    }

    /// An anonymous catbox upload can never be deleted by the uploader.
    pub fn is_permanent(&self) -> bool {
        matches!(self, Host::Catbox)
    }

    pub fn is_encrypted(&self) -> bool {
        match self {
            Host::Custom(up) => up.is_encrypted(),
            _ => true,
        }
    }

    /// Imgur without a Client-ID cannot upload at all; the window explains
    /// how to get one instead of asking for consent.
    pub fn needs_client_id(&self) -> bool {
        matches!(self, Host::Imgur { client_id } if client_id.is_empty())
    }
}

/// Upload on a worker thread. The link is copied to the clipboard there, and
/// the outcome comes back over `done`, with the pump woken to read it.
pub fn spawn(img: image::RgbaImage, host: Host, done: Sender<Outcome>) {
    let _ = std::thread::Builder::new()
        .name("upload".into())
        .spawn(move || {
            let result = encode_png(&img).and_then(|png| match &host {
                Host::Catbox => catbox_upload(&png),
                Host::Imgur { client_id } => imgur_upload(&png, client_id),
                Host::Custom(up) => crate::sxcu::send(&http_agent(), up, &png, &file_name()),
            });
            let outcome = match result {
                Ok(url) => {
                    let copied = crate::clipboard::copy_text(&url).is_ok();
                    log::info!("uploaded to {} (link copied: {copied})", host.name());
                    Outcome::Done { url, copied }
                }
                Err(e) => {
                    log::warn!("upload to {} failed: {e:#}", host.name());
                    Outcome::Failed(format!("{e}"))
                }
            };
            let _ = done.send(outcome);
            crate::pump::wake();
        });
}

/// `snip-20261005-142233.png` — what the host stores, and often shows.
fn file_name() -> String {
    format!("snip-{}.png", chrono::Local::now().format("%Y%m%d-%H%M%S"))
}

/// A ureq agent with connect/read/write timeouts so a stalled upload
/// (dropped Wi-Fi, captive portal, hung TLS handshake) can't pin a worker
/// thread — and a clone of the full image — forever.
fn http_agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(60))
        .timeout_write(Duration::from_secs(60))
        .build()
}

/// POST the PNG to Imgur's anonymous endpoint and return the image URL.
fn imgur_upload(png: &[u8], client_id: &str) -> Result<String> {
    // Hand-rolled multipart/form-data body (one `image` part). Cheaper than
    // base64 (no 33% bloat) and avoids adding a multipart crate.
    let mut body = Vec::with_capacity(png.len() + 256);
    body.extend_from_slice(
        format!(
            "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"image\"; \
             filename=\"snip.png\"\r\nContent-Type: image/png\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(png);
    body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());

    let resp = http_agent()
        .post(IMGUR_ENDPOINT)
        .set("Authorization", &format!("Client-ID {client_id}"))
        .set("Content-Type", &format!("multipart/form-data; boundary={BOUNDARY}"))
        .send_bytes(&body);

    let resp = match resp {
        Ok(r) => r,
        Err(ureq::Error::Status(403, _)) => {
            return Err(anyhow!(crate::i18n::t("Imgur refused the Client-ID (HTTP 403).")));
        }
        Err(ureq::Error::Status(429, _)) => {
            return Err(anyhow!(crate::i18n::t(
                "Imgur's upload limit is reached (HTTP 429). Try again later."
            )));
        }
        Err(ureq::Error::Status(code, r)) => {
            let detail = r.into_string().unwrap_or_default();
            return Err(anyhow!("HTTP {code}: {}", detail.trim()));
        }
        Err(e) => return Err(unreachable_host("api.imgur.com", &e)),
    };

    let text = resp.into_string().unwrap_or_default();
    serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .and_then(|v| v.get("data")?.get("link")?.as_str().map(str::to_string))
        .ok_or_else(|| no_link(&text))
}

/// The custom uploaders' wording, in the user's language: the built-in
/// hosts said these in English whatever the language was.
fn unreachable_host(host: &str, e: &ureq::Error) -> anyhow::Error {
    anyhow!(crate::i18n::t1("Could not reach {host}: {e}", "{host}", host)
        .replace("{e}", &crate::sxcu::net_reason(e)))
}

fn no_link(answer: &str) -> anyhow::Error {
    let answer: String = answer.trim().chars().take(200).collect();
    anyhow!(crate::i18n::t1("The host answered without a link: {answer}", "{answer}", &answer))
}

/// POST the PNG to catbox.moe and return the direct image URL. Anonymous
/// (no `userhash`); catbox replies with the bare URL as plain text.
fn catbox_upload(png: &[u8]) -> Result<String> {
    let mut body = Vec::with_capacity(png.len() + 512);
    // Part 1: reqtype=fileupload
    body.extend_from_slice(
        format!(
            "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"reqtype\"\r\n\r\n\
             fileupload\r\n"
        )
        .as_bytes(),
    );
    // Part 2: the file itself
    body.extend_from_slice(
        format!(
            "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"fileToUpload\"; \
             filename=\"snip.png\"\r\nContent-Type: image/png\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(png);
    body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());

    let resp = http_agent()
        .post(CATBOX_ENDPOINT)
        .set("Content-Type", &format!("multipart/form-data; boundary={BOUNDARY}"))
        .send_bytes(&body);

    let resp = match resp {
        Ok(r) => r,
        Err(ureq::Error::Status(code, r)) => {
            let detail = r.into_string().unwrap_or_default();
            return Err(anyhow!("HTTP {code}: {}", detail.trim()));
        }
        Err(e) => return Err(unreachable_host("catbox.moe", &e)),
    };

    // catbox returns the bare URL as plain text (and, on failure, a short
    // error string with HTTP 200 — so check it actually looks like a URL).
    let url = resp.into_string().unwrap_or_default();
    let url = url.trim().to_string();
    if url.starts_with("http://") || url.starts_with("https://") {
        Ok(url)
    } else {
        Err(no_link(&url))
    }
}

/// Encode the snip to PNG bytes in memory.
fn encode_png(img: &image::RgbaImage) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    let mut cur = std::io::Cursor::new(&mut buf);
    image::write_buffer_with_format(
        &mut cur,
        img.as_raw(),
        img.width(),
        img.height(),
        image::ExtendedColorType::Rgba8,
        image::ImageFormat::Png,
    )
    .context("Failed to encode snip as PNG")?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(service: &str) -> Settings {
        let mut s: Settings = serde_json::from_str("{}").unwrap();
        s.upload_service = service.into();
        s.upload_hosts = vec![Uploader::parse(
            r#"{"Name":"Mine","RequestURL":"http://10.0.0.2/up","Body":"Binary"}"#,
        )
        .unwrap()];
        s
    }

    #[test]
    fn the_setting_picks_the_host() {
        assert!(matches!(Host::from_settings(&settings("catbox")), Host::Catbox));
        assert!(Host::from_settings(&settings("imgur")).needs_client_id());
        let custom = Host::from_settings(&settings("custom:Mine"));
        assert_eq!(custom.name(), "Mine");
        assert!(!custom.is_encrypted() && !custom.is_unlisted());
        // Removed from the list since: back to the default, not a failure.
        assert!(matches!(Host::from_settings(&settings("custom:Gone")), Host::Catbox));
    }
}
