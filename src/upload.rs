//! Upload a snip image to an image host and copy the resulting URL.
//!
//! The default host is **catbox.moe** — no API key, no login: the PNG is POSTed
//! anonymously and catbox replies with a bare URL. **Imgur** is also supported
//! via its *anonymous* upload endpoint, which needs a free "Client-ID" (no user
//! login, no Client-Secret); register an application once at imgur.com
//! (Settings → Applications → Register, "OAuth 2 without a callback URL") and
//! paste the Client-ID into `settings.imgur_client_id`. The host is chosen by
//! `settings.upload_service` ("catbox" or "imgur").
//!
//! The HTTP upload is blocking, so it runs on a worker thread; on success it
//! copies the image URL to the clipboard and pops a notification — mirroring
//! the `share_mail` clipboard-fallback pattern. The backend is kept behind a
//! single `upload` entry point so other hosts (0x0, a custom / self-hosted
//! endpoint, cloud storage) can slot in later without touching the call site.

pub fn upload(img: &image::RgbaImage, service: &str, client_id: &str) -> anyhow::Result<()> {
    imp::upload(img, service, client_id)
}

mod imp {
    use anyhow::{anyhow, Context, Result};
    use std::time::Duration;

    /// Imgur anonymous image-upload endpoint.
    const IMGUR_ENDPOINT: &str = "https://api.imgur.com/3/image";
    /// catbox.moe anonymous upload endpoint (no API key, no login).
    const CATBOX_ENDPOINT: &str = "https://catbox.moe/user/api.php";
    /// Fixed multipart boundary. A binary PNG can't contain this ASCII token,
    /// so a constant boundary is safe and avoids pulling in an RNG just to
    /// randomise it.
    const BOUNDARY: &str = "----snipdeckImgurBoundary7e3f1a9c4d";

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

    /// Spawn the (blocking) upload on a worker thread. On success the link is
    /// copied to the clipboard and a notification is shown; on failure a notice
    /// explains why. Returns immediately — never blocks the UI thread.
    ///
    /// `service` picks the host: "imgur" (needs a Client-ID) or anything else →
    /// catbox.moe (the default, no key/login). A missing Imgur Client-ID is
    /// surfaced with its own topmost notice (the action is usually fired from a
    /// floating snip's menu while the gallery — and its error banner — is
    /// hidden, so a banner alone would go unseen).
    pub fn upload(img: &image::RgbaImage, service: &str, client_id: &str) -> Result<()> {
        let use_imgur = service.eq_ignore_ascii_case("imgur");
        // Imgur needs a Client-ID up front; fail fast with a visible notice
        // rather than spawning a thread that can only error out.
        if use_imgur && client_id.trim().is_empty() {
            notify_no_client_id();
            return Ok(());
        }
        // Informed consent: spell out the destination + privacy implications and
        // require an explicit OK before any bytes leave the machine. Cancel aborts.
        if !confirm_upload(use_imgur) {
            return Ok(());
        }
        let img = img.clone();
        let client_id = client_id.to_string();
        std::thread::spawn(move || {
            let result = if use_imgur {
                imgur_upload(&img, &client_id)
            } else {
                catbox_upload(&img)
            };
            match result {
                Ok(url) => {
                    let copied = crate::clipboard::copy_text(&url).is_ok();
                    notify_ok(&url, copied);
                }
                Err(e) => {
                    log::warn!("upload failed: {e:#}");
                    notify_err(&format!("{e}"));
                }
            }
        });
        Ok(())
    }

    /// POST the PNG to Imgur's anonymous endpoint and return the image URL.
    fn imgur_upload(img: &image::RgbaImage, client_id: &str) -> Result<String> {
        let png = encode_png(img)?;

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
        body.extend_from_slice(&png);
        body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());

        let resp = http_agent()
            .post(IMGUR_ENDPOINT)
            .set("Authorization", &format!("Client-ID {client_id}"))
            .set(
                "Content-Type",
                &format!("multipart/form-data; boundary={BOUNDARY}"),
            )
            .send_bytes(&body);

        let resp = match resp {
            Ok(r) => r,
            Err(ureq::Error::Status(403, _)) => {
                return Err(anyhow!(
                    "Imgur rejected the request (HTTP 403). The Client-ID may be invalid."
                ));
            }
            Err(ureq::Error::Status(429, _)) => {
                return Err(anyhow!(
                    "Imgur quota exceeded (HTTP 429). Try again later."
                ));
            }
            Err(ureq::Error::Status(code, r)) => {
                let detail = r.into_string().unwrap_or_default();
                return Err(anyhow!("Imgur returned an error (HTTP {code}): {detail}"));
            }
            Err(e) => return Err(anyhow!("Failed to connect to Imgur: {e}")),
        };

        let v: serde_json::Value = resp.into_json().context("Failed to parse Imgur response")?;
        v.get("data")
            .and_then(|d| d.get("link"))
            .and_then(|l| l.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("No image link found in Imgur response"))
    }

    /// POST the PNG to catbox.moe and return the direct image URL. Anonymous
    /// (no `userhash`); catbox replies with the bare URL as plain text.
    fn catbox_upload(img: &image::RgbaImage) -> Result<String> {
        let png = encode_png(img)?;

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
        body.extend_from_slice(&png);
        body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());

        let resp = http_agent()
            .post(CATBOX_ENDPOINT)
            .set(
                "Content-Type",
                &format!("multipart/form-data; boundary={BOUNDARY}"),
            )
            .send_bytes(&body);

        let resp = match resp {
            Ok(r) => r,
            Err(ureq::Error::Status(code, r)) => {
                let detail = r.into_string().unwrap_or_default();
                return Err(anyhow!("catbox returned an error (HTTP {code}): {detail}"));
            }
            Err(e) => return Err(anyhow!("Failed to connect to catbox: {e}")),
        };

        // catbox returns the bare URL as plain text (and, on failure, a short
        // error string with HTTP 200 — so check it actually looks like a URL).
        let url = resp.into_string().context("Failed to read catbox response")?;
        let url = url.trim().to_string();
        if url.starts_with("http://") || url.starts_with("https://") {
            Ok(url)
        } else {
            Err(anyhow!("catbox returned an unexpected response: {url}"))
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

    fn notify_ok(url: &str, copied: bool) {
        let body = if copied {
            crate::i18n::t1(
                "Image uploaded.\r\nLink copied to clipboard:\r\n{url}",
                "{url}",
                url,
            )
        } else {
            crate::i18n::t1(
                "Image uploaded:\r\n{url}\r\n\r\n(Could not copy the link to the clipboard.)",
                "{url}",
                url,
            )
        };
        message_box(&body, &crate::i18n::t("Snipdeck — Upload"), Level::Info);
    }

    fn notify_err(err: &str) {
        message_box(
            &crate::i18n::t1("Upload failed:\r\n{err}", "{err}", err),
            &crate::i18n::t("Snipdeck — Upload"),
            Level::Error,
        );
    }

    /// Shown when no Imgur Client-ID is configured: explain how to get one (free,
    /// no login) and exactly where to paste it. Includes the live settings path.
    fn notify_no_client_id() {
        let path = crate::paths::settings_file();
        let body = crate::i18n::t1(
            "No Imgur Client-ID configured — cannot upload.\r\n\r\n\
             1) Get a free Client-ID:\r\n   \
             imgur.com → Settings → Applications → Register\r\n   \
             (\"OAuth 2 authorization without a callback URL\")\r\n\r\n\
             2) Add it to this file:\r\n   {path}\r\n   \
             \"imgur_client_id\": \"YOUR_CLIENT_ID_HERE\"\r\n\r\n\
             3) Restart Snipdeck.",
            "{path}",
            &path.display().to_string(),
        );
        message_box(
            &body,
            &crate::i18n::t("Snipdeck — Imgur Client-ID required"),
            Level::Info,
        );
    }

    /// Pre-upload informed-consent prompt. Spells out the destination and the
    /// privacy implications (public-by-link, not encrypted, no self-delete on
    /// catbox) and requires an explicit OK. Returns true only if the user
    /// confirmed; cancelling aborts the upload before anything is sent.
    fn confirm_upload(use_imgur: bool) -> bool {
        let body = if use_imgur {
            crate::i18n::t(
                "This image will be uploaded to the INTERNET — \"Imgur\" (anonymous).\r\n\r\n\
                 • It will NOT appear on the Imgur home page / gallery (hidden, unlisted).\r\n\
                 • But anyone who has the resulting link can open the image.\r\n\
                 • The image is not encrypted.\r\n\
                 • Do NOT upload SENSITIVE content such as IDs, passwords, or private messages.\r\n\r\n\
                 On success the link is copied to your clipboard.\r\n\r\nUpload?",
            )
        } else {
            crate::i18n::t(
                "This image will be uploaded to the INTERNET — \"catbox.moe\".\r\n\r\n\
                 • It will NOT appear in any public gallery / search (unlisted).\r\n\
                 • But anyone who has the resulting link can open the image.\r\n\
                 • Because it is uploaded anonymously, YOU CANNOT DELETE IT yourself later.\r\n\
                 • The image is not encrypted.\r\n\
                 • Do NOT upload SENSITIVE content such as IDs, passwords, or private messages.\r\n\r\n\
                 On success the link is copied to your clipboard.\r\n\r\nUpload?",
            )
        };
        confirm_box(&body, &crate::i18n::t("Snipdeck — Upload to the internet"))
    }

    /// Severity of a notice, mapped to each platform's own icon/level.
    #[derive(Clone, Copy)]
    enum Level {
        Info,
        Error,
    }

    /// Topmost OK/Cancel confirmation. Returns true on OK.
    #[cfg(target_os = "windows")]
    fn confirm_box(body: &str, caption: &str) -> bool {
        use windows::core::{HSTRING, PCWSTR};
        use windows::Win32::UI::WindowsAndMessaging::{
            MessageBoxW, IDOK, MB_ICONWARNING, MB_OKCANCEL, MB_SETFOREGROUND, MB_TOPMOST,
        };
        let text = HSTRING::from(body);
        let caption = HSTRING::from(caption);
        let ret = unsafe {
            MessageBoxW(
                None,
                PCWSTR(text.as_ptr()),
                PCWSTR(caption.as_ptr()),
                MB_OKCANCEL | MB_ICONWARNING | MB_SETFOREGROUND | MB_TOPMOST,
            )
        };
        ret == IDOK
    }

    /// Consent prompt off Windows. Anything other than an explicit OK — Cancel,
    /// a closed window, or a desktop with no dialog backend at all — must read
    /// as "do not upload", so this compares against Ok rather than negating.
    #[cfg(not(target_os = "windows"))]
    fn confirm_box(body: &str, caption: &str) -> bool {
        matches!(
            rfd::MessageDialog::new()
                .set_level(rfd::MessageLevel::Warning)
                .set_title(caption)
                .set_description(body)
                .set_buttons(rfd::MessageButtons::OkCancel)
                .show(),
            rfd::MessageDialogResult::Ok
        )
    }

    /// A topmost message box so the result is seen even when the gallery window
    /// is hidden (the upload finishes on a worker thread, possibly long after
    /// the user moved on).
    #[cfg(target_os = "windows")]
    fn message_box(body: &str, caption: &str, level: Level) {
        use windows::core::{HSTRING, PCWSTR};
        use windows::Win32::UI::WindowsAndMessaging::{
            MessageBoxW, MB_ICONERROR, MB_ICONINFORMATION, MB_OK, MB_SETFOREGROUND, MB_TOPMOST,
        };
        let icon = match level {
            Level::Info => MB_ICONINFORMATION,
            Level::Error => MB_ICONERROR,
        };
        let text = HSTRING::from(body);
        let caption = HSTRING::from(caption);
        unsafe {
            MessageBoxW(
                None,
                PCWSTR(text.as_ptr()),
                PCWSTR(caption.as_ptr()),
                MB_OK | icon | MB_SETFOREGROUND | MB_TOPMOST,
            );
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn message_box(body: &str, caption: &str, level: Level) {
        rfd::MessageDialog::new()
            .set_level(match level {
                Level::Info => rfd::MessageLevel::Info,
                Level::Error => rfd::MessageLevel::Error,
            })
            .set_title(caption)
            .set_description(body)
            .set_buttons(rfd::MessageButtons::Ok)
            .show();
    }
}
