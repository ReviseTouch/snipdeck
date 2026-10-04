//! Share a snip image to the OS / mail, or open it in the image editor.
//!
//! Entry points, all taking the decoded snip:
//!   * `share_system` — Windows' native share sheet (the Win11 flyout). The
//!     image is handed over directly as a bitmap, so Mail, WhatsApp, Telegram,
//!     Teams … every registered share target receives the actual picture. This
//!     is also the only reliable way to land an attachment in the *new* Outlook
//!     (which doesn't speak Simple MAPI — see `share_mail`).
//!   * `share_mail` — open a new message with the snip attached via Simple
//!     MAPI. Works with desktop MAPI clients (classic Outlook, Thunderbird);
//!     the new Outlook / web mail can't attach this way, so the user should use
//!     the share sheet for those.
//!   * `open_in_editor` — open the snip in the system image editor (Paint) for
//!     a quick edit.

#[cfg(target_os = "windows")]
pub fn share_system(img: &image::RgbaImage) -> anyhow::Result<()> {
    win::share_system(img)
}

#[cfg(target_os = "windows")]
pub fn share_mail(img: &image::RgbaImage) -> anyhow::Result<()> {
    win::share_mail(img)
}

#[cfg(target_os = "windows")]
pub fn open_in_editor(img: &image::RgbaImage) -> anyhow::Result<()> {
    win::open_in_editor(img)
}

/// The Windows share sheet has no portable counterpart — xdg-desktop-portal's
/// share interface needs a D-Bus round trip we don't ship. Mail and "open in
/// editor" below cover the cases that matter in practice.
#[cfg(not(target_os = "windows"))]
pub fn share_system(_img: &image::RgbaImage) -> anyhow::Result<()> {
    anyhow::bail!("the system share sheet is Windows-only — use Mail or Save instead")
}

#[cfg(not(target_os = "windows"))]
pub fn share_mail(img: &image::RgbaImage) -> anyhow::Result<()> {
    unix::spawn_with("xdg-email", &["--attach"], img)
}

#[cfg(not(target_os = "windows"))]
pub fn open_in_editor(img: &image::RgbaImage) -> anyhow::Result<()> {
    unix::spawn_with("xdg-open", &[], img)
}

/// Off Windows both paths reduce to "write a PNG, hand the path to an
/// xdg-utils helper". The helper resolves the user's actual mail client /
/// image viewer, so we don't have to guess at desktop environments.
#[cfg(not(target_os = "windows"))]
pub use unix::spawn_detached;

#[cfg(not(target_os = "windows"))]
mod unix {
    use anyhow::{Context, Result};
    use std::path::PathBuf;
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Duration;

    /// How long a shared copy is kept for the program it was handed to.
    const KEEP: Duration = Duration::from_secs(60 * 60);

    /// The spawned helper reads the file asynchronously — often after we
    /// return — so it cannot be deleted here. It goes in the user's private
    /// temp dir (`paths::private_temp_dir`), not world-readable /tmp, under a
    /// name unique per share — the old name, process id plus snip size, made
    /// two same-sized shares overwrite each other — and copies older than an
    /// hour are swept on each share; the runtime dir itself is cleared at logout.
    fn temp_png(img: &image::RgbaImage) -> Result<PathBuf> {
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let dir = crate::paths::private_temp_dir().context("no private temp directory")?;
        sweep(&dir);
        let path = dir.join(format!(
            "snipdeck-share-{}-{}.png",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        img.save_with_format(&path, image::ImageFormat::Png)
            .with_context(|| format!("writing {} to share", path.display()))?;
        Ok(path)
    }

    fn sweep(dir: &std::path::Path) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let stale = entry
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.elapsed().ok())
                .is_some_and(|age| age > KEEP);
            if stale && entry.file_name().to_string_lossy().starts_with("snipdeck-share-") {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }

    pub fn spawn_with(program: &str, args: &[&str], img: &image::RgbaImage) -> Result<()> {
        let path = temp_png(img)?;
        let mut command = Command::new(program);
        command.args(args).arg(&path);
        spawn_detached(command)
            .with_context(|| format!("could not run {program} — install xdg-utils"))
    }

    /// Start a helper and leave it to run, reaping it when it exits.
    ///
    /// xdg-open and xdg-email hand the file on and exit at once, and a child
    /// nobody waits for stays in the process table as a zombie until Snipdeck
    /// itself exits — one per share, open and link click. A small thread
    /// waits on each instead.
    pub fn spawn_detached(mut command: Command) -> std::io::Result<()> {
        let mut child = command.spawn()?;
        let _ = std::thread::Builder::new()
            .name("reap-child".into())
            .spawn(move || {
                let _ = child.wait();
            });
        Ok(())
    }
}

#[cfg(target_os = "windows")]
mod win {
    use anyhow::{anyhow, Context, Result};
    use std::os::windows::ffi::OsStrExt;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Duration;
    use windows::core::{factory, s, w, HSTRING, PCWSTR, PWSTR};
    use windows::ApplicationModel::DataTransfer::{
        DataRequestedEventArgs, DataTransferManager,
    };
    use windows::Foundation::TypedEventHandler;
    use windows::Storage::Streams::{
        DataWriter, InMemoryRandomAccessStream, RandomAccessStreamReference,
    };
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
    use windows::Win32::System::Mapi::{MapiFileDescW, MapiMessageW, MAPI_LOGON_UI};
    use windows::Win32::UI::Shell::{IDataTransferManagerInterop, ShellExecuteW};
    use windows::Win32::UI::WindowsAndMessaging::{
        MessageBoxW, SetForegroundWindow, MB_ICONINFORMATION, MB_OK,
        MB_SETFOREGROUND, MB_TOPMOST, SW_SHOWNORMAL,
    };

    /// Mail share: open a NEW message with the snip actually attached.
    ///
    /// `mailto:` can't carry an attachment, so we go through Simple MAPI
    /// (`MAPISendMailW` — the "Send to → Mail recipient" path). It opens the
    /// default *desktop* mail client's compose window with the PNG attached.
    /// The call is modal (it blocks until the user sends/closes the compose),
    /// so it runs on a worker thread — never the UI thread.
    ///
    /// If no MAPI client is registered (e.g. the *new* Outlook or a web/UWP
    /// mail app, which can't accept a programmatic attachment), we fall back to:
    /// copy the snip to the clipboard, open an empty compose with `mailto:`, and
    /// pop a notice telling the user to paste it with Ctrl+V.
    pub fn share_mail(img: &image::RgbaImage) -> Result<()> {
        let path = temp_png(img, "mail")?;
        let fallback_img = img.clone();
        std::thread::spawn(move || match mapi_send(&path) {
            Ok(()) => log::info!("share: MAPI mail opened with attachment"),
            Err(e) => {
                log::warn!("share: MAPI failed ({e}); falling back to mailto + clipboard");
                let copied = crate::clipboard::copy_image(&fallback_img).is_ok();
                let _ = shell_open("mailto:?subject=Snipdeck");
                notify_paste(copied);
            }
        });
        Ok(())
    }

    /// Tell the user the snip is on the clipboard and to paste it into the mail
    /// compose with Ctrl+V (shown when the mail client can't auto-attach). A
    /// topmost message box so it's seen even when the gallery window is hidden.
    fn notify_paste(copied: bool) {
        let body = if copied {
            crate::i18n::t("The image has been copied to the clipboard.\r\nPaste it into the open email with Ctrl+V.")
        } else {
            crate::i18n::t("The email opened, but the image could not be copied to the clipboard.\r\nCopy the snip again and paste it with Ctrl+V.")
        };
        let text = HSTRING::from(body.as_str());
        let caption = HSTRING::from(crate::i18n::t("Snipdeck — Attach to email").as_str());
        unsafe {
            MessageBoxW(
                None,
                PCWSTR(text.as_ptr()),
                PCWSTR(caption.as_ptr()),
                MB_OK | MB_ICONINFORMATION | MB_SETFOREGROUND | MB_TOPMOST,
            );
        }
    }

    /// Open the default MAPI mail client's compose window with `path` attached.
    /// Blocking + modal — call from a worker thread.
    fn mapi_send(path: &Path) -> Result<()> {
        // ULONG MAPISendMailW(LHANDLE, ULONG_PTR uiparam, lpMapiMessageW, FLAGS, ULONG)
        type SendMailW =
            unsafe extern "system" fn(usize, usize, *const MapiMessageW, u32, u32) -> u32;
        const MAPI_DIALOG: u32 = 0x8;

        unsafe {
            let lib = LoadLibraryW(w!("mapi32.dll")).context("failed to load mapi32.dll")?;
            let Some(addr) = GetProcAddress(lib, s!("MAPISendMailW")) else {
                return Err(anyhow!("MAPISendMailW not found (no MAPI client)"));
            };
            let send: SendMailW = std::mem::transmute(addr);

            let mut path_w: Vec<u16> = path
                .as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            let mut file_w: Vec<u16> = "snip.png".encode_utf16().chain(std::iter::once(0)).collect();
            let mut subj_w: Vec<u16> = "Snipdeck".encode_utf16().chain(std::iter::once(0)).collect();
            let mut note_w: Vec<u16> = "Snipdeck screenshot\r\n"
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();

            let mut file = MapiFileDescW {
                nPosition: u32::MAX,
                lpszPathName: PWSTR(path_w.as_mut_ptr()),
                lpszFileName: PWSTR(file_w.as_mut_ptr()),
                ..Default::default()
            };
            let msg = MapiMessageW {
                lpszSubject: PWSTR(subj_w.as_mut_ptr()),
                lpszNoteText: PWSTR(note_w.as_mut_ptr()),
                nFileCount: 1,
                lpFiles: &mut file,
                ..Default::default()
            };

            // 0 = SUCCESS, 1 = USER_ABORT (user opened compose then cancelled —
            // MAPI worked, so don't fall back). Anything else is a real failure.
            let rc = send(0, 0, &msg, MAPI_LOGON_UI | MAPI_DIALOG, 0);
            if rc == 0 || rc == 1 {
                Ok(())
            } else {
                Err(anyhow!("MAPISendMailW rc={rc}"))
            }
        }
    }

    /// Open the snip in the system image editor (Paint) for a quick edit.
    /// Writes a temp PNG, then asks the shell for the registered image *editor*
    /// (the "edit" verb — usually Paint). Some setups don't register an "edit"
    /// verb for PNG, so we fall back to launching `mspaint.exe` with the file.
    pub fn open_in_editor(img: &image::RgbaImage) -> Result<()> {
        let path = temp_png(img, "edit")?;
        let path_w: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        unsafe {
            let inst = ShellExecuteW(
                None,
                w!("edit"),
                PCWSTR(path_w.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            );
            if (inst.0 as isize) > 32 {
                return Ok(());
            }
            // No "edit" verb → open Paint explicitly, passing the (quoted, in
            // case of spaces) file path as its argument.
            let quoted = format!("\"{}\"", path.display());
            let param_w: Vec<u16> = quoted.encode_utf16().chain(std::iter::once(0)).collect();
            let inst2 = ShellExecuteW(
                None,
                w!("open"),
                w!("mspaint.exe"),
                PCWSTR(param_w.as_ptr()),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            );
            if (inst2.0 as isize) <= 32 {
                return Err(anyhow!(
                    "failed to open image editor (code {})",
                    inst2.0 as isize
                ));
            }
        }
        Ok(())
    }

    /// Open a URL/protocol with the default handler (mailto: …).
    pub fn shell_open(url: &str) -> Result<()> {
        let url = HSTRING::from(url);
        unsafe {
            let inst = ShellExecuteW(
                None,
                w!("open"),
                PCWSTR(url.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            );
            // ShellExecuteW returns an HINSTANCE whose value is > 32 on success.
            if (inst.0 as isize) <= 32 {
                return Err(anyhow!("failed to launch app (code {})", inst.0 as isize));
            }
        }
        Ok(())
    }

    /// How long a handed-over copy is kept for the program it went to.
    const KEEP: Duration = Duration::from_secs(24 * 60 * 60);

    /// A copy of the snip for another program to open: the mail client's
    /// attachment, the editor's file. That program reads it after we return,
    /// so it cannot be deleted here.
    ///
    /// Unique per hand-over: under the old fixed names a second "edit in
    /// Paint" overwrote the file the first Paint window had open, and a second
    /// mail share replaced an attachment the first compose had not read yet.
    /// Kept in a folder of Snipdeck's own in %TEMP%, where copies older than a
    /// day are swept at the next hand-over rather than left in Temp for good.
    fn temp_png(img: &image::RgbaImage, kind: &str) -> Result<PathBuf> {
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join("Snipdeck");
        std::fs::create_dir_all(&dir).context("creating Snipdeck's temp folder")?;
        sweep(&dir);
        let path = dir.join(format!(
            "snipdeck-{kind}-{}-{}.png",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        img.save_with_format(&path, image::ImageFormat::Png)
            .with_context(|| format!("writing {}", path.display()))?;
        Ok(path)
    }

    fn sweep(dir: &Path) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let stale = entry
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.elapsed().ok())
                .is_some_and(|age| age > KEEP);
            if stale && entry.file_name().to_string_lossy().starts_with("snipdeck-") {
                // Fails harmlessly on a file a program still holds open.
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }

    thread_local! {
        // The share before this one's DataRequested registration, to remove.
        static PREVIOUS_SHARE: std::cell::RefCell<
            Option<(DataTransferManager, windows::Foundation::EventRegistrationToken)>,
        > = const { std::cell::RefCell::new(None) };
    }

    /// A window to anchor the share flyout on. Prefer the MAIN window: it's a
    /// normal, focusable, non-topmost window, so the Win11 share flyout hosts on
    /// it reliably and isn't covered/auto-dismissed the way it is when anchored
    /// on a borderless, always-on-top tool window (the floating snip). The
    /// caller makes the main window visible first. Falls back to a floating snip.
    fn anchor_hwnd() -> HWND {
        if let Some(h) = crate::window_tamer::find_own_window("Snipdeck") {
            log::info!("share: anchoring on main window");
            return h;
        }
        log::info!("share: no main window, falling back to floating snip");
        crate::window_tamer::find_own_window("SnipdeckFloating").unwrap_or_default()
    }

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
        .context("failed to encode snip as PNG")?;
        Ok(buf)
    }

    pub fn share_system(img: &image::RgbaImage) -> Result<()> {
        let png = encode_png(img)?;
        unsafe {
            let hwnd = anchor_hwnd();
            log::info!("share: anchor hwnd = {:?}", hwnd.0);
            if hwnd.0.is_null() {
                return Err(anyhow!("no window found for sharing"));
            }
            // The share flyout hangs off the foreground window — make our
            // anchor foreground first, or Windows may swallow the call.
            let _ = SetForegroundWindow(hwnd);

            // PNG bytes → in-memory WinRT stream → stream reference. The data
            // writer's StoreAsync completes on a thread-pool thread, so the
            // blocking `.get()` here doesn't deadlock the UI (STA) thread.
            let stream = InMemoryRandomAccessStream::new().context("WinRT stream")?;
            let writer = DataWriter::CreateDataWriter(&stream).context("DataWriter")?;
            writer.WriteBytes(&png).context("WriteBytes")?;
            writer.StoreAsync().context("StoreAsync")?.get().context("StoreAsync.get")?;
            writer.FlushAsync().context("FlushAsync")?.get().context("FlushAsync.get")?;
            let _ = writer.DetachStream();
            stream.Seek(0).context("seek")?;
            let stream_ref =
                RandomAccessStreamReference::CreateFromStream(&stream).context("stream ref")?;

            // Desktop apps must obtain the DataTransferManager via the interop
            // interface bound to a specific HWND, then show the flyout for it.
            let interop: IDataTransferManagerInterop =
                factory::<DataTransferManager, IDataTransferManagerInterop>()
                    .context("DataTransferManager interop")?;
            let dtm: DataTransferManager =
                interop.GetForWindow(hwnd).context("GetForWindow")?;

            // Capture BOTH the stream reference AND the underlying stream so the
            // image stays alive until the chosen target actually reads it (which
            // happens after this function returns, when the user picks an app).
            let handler = TypedEventHandler::<DataTransferManager, DataRequestedEventArgs>::new(
                move |_sender, args| {
                    log::info!("share: DataRequested fired");
                    let _keep_alive = &stream;
                    if let Some(args) = args.as_ref() {
                        let data = args.Request()?.Data()?;
                        data.Properties()?.SetTitle(&HSTRING::from("Snipdeck"))?;
                        match data.SetBitmap(&stream_ref) {
                            Ok(()) => log::info!("share: bitmap set"),
                            Err(e) => log::warn!("share: SetBitmap failed: {e}"),
                        }
                    }
                    Ok(())
                },
            );
            // One handler per share, and the last one taken back before the
            // next goes in. Registered and never removed, every earlier share's
            // handler fired again on each later one — each setting its own,
            // older bitmap on the same request, the stale image racing the new
            // one into the target app — and each kept its PNG alive for the
            // rest of the session.
            PREVIOUS_SHARE.with(|previous| {
                if let Some((old_dtm, token)) = previous.borrow_mut().take() {
                    let _ = old_dtm.RemoveDataRequested(token);
                }
            });
            let token = dtm.DataRequested(&handler).context("DataRequested")?;
            PREVIOUS_SHARE.with(|previous| *previous.borrow_mut() = Some((dtm.clone(), token)));
            interop
                .ShowShareUIForWindow(hwnd)
                .context("ShowShareUIForWindow")?;
            log::info!("share: ShowShareUIForWindow returned Ok");
        }
        Ok(())
    }
}
