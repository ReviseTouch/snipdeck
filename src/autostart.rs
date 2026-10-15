//! "Launch at login" support — deliberately registry-free.
//!
//! On Windows we drop a normal `.lnk` shortcut into the per-user
//! `shell:Startup` folder rather than writing a `Run` value under
//! `HKCU\...\CurrentVersion\Run`. Both autostart the app, but the registry
//! Run key is the single most heuristically-flagged autostart vector for
//! antivirus / EDR tools, whereas a Startup-folder shortcut is the ordinary,
//! user-visible mechanism the Settings UI itself manages. macOS uses a
//! LaunchAgent plist and Linux an XDG autostart `.desktop` entry — the
//! native, file-based equivalents on each platform.

/// Passed by every launch-at-login entry: a login start comes up in the
/// tray, not as a window in the user's face. Launched by hand — no flag — the
/// gallery opens as it always has.
pub const ARG: &str = "--autostart";

/// Is launch-at-login currently enabled? (Cheap filesystem check.)
pub fn is_enabled() -> bool {
    imp::is_enabled()
}

/// Enable or disable launch-at-login.
pub fn set_enabled(enabled: bool) -> anyhow::Result<()> {
    imp::set_enabled(enabled)
}

/// At start-up: if launch-at-login is on, write the entry again for this
/// executable. The entry records the path it was enabled from; moved or
/// reinstalled elsewhere, the binary would otherwise stop starting at login
/// without a word. It also carries an entry written by an older build up to
/// the current form — one without `ARG` opened the window at every login.
/// macOS: a no-op for now.
pub fn refresh() {
    if let Err(e) = imp::refresh() {
        log::warn!("could not refresh the launch-at-login entry: {e}");
    }
}

/// Flip the current state; returns the new state.
pub fn toggle() -> anyhow::Result<bool> {
    let next = !is_enabled();
    set_enabled(next)?;
    Ok(next)
}

#[cfg(target_os = "windows")]
mod imp {
    use anyhow::{anyhow, Result};
    use std::path::PathBuf;
    use windows::core::{Interface, PCWSTR};
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, IPersistFile, CLSCTX_INPROC_SERVER,
        COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};

    fn link_path() -> Option<PathBuf> {
        let appdata = std::env::var_os("APPDATA")?;
        Some(
            PathBuf::from(appdata)
                .join(r"Microsoft\Windows\Start Menu\Programs\Startup")
                .join("Snipdeck.lnk"),
        )
    }

    pub fn refresh() -> Result<()> {
        if is_enabled() {
            set_enabled(true)?;
        }
        Ok(())
    }

    pub fn is_enabled() -> bool {
        link_path().map(|p| p.exists()).unwrap_or(false)
    }

    pub fn set_enabled(enabled: bool) -> Result<()> {
        let link = link_path().ok_or_else(|| anyhow!("APPDATA not found"))?;
        if !enabled {
            if link.exists() {
                std::fs::remove_file(&link)?;
            }
            return Ok(());
        }
        if let Some(parent) = link.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let exe = std::env::current_exe()?;
        let workdir = exe.parent().map(|p| p.to_path_buf());
        unsafe { create_shortcut(&exe, workdir.as_deref(), &link) }
    }

    unsafe fn create_shortcut(
        target: &std::path::Path,
        workdir: Option<&std::path::Path>,
        link: &std::path::Path,
    ) -> Result<()> {
        let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let inited = hr.is_ok();
        let result = (|| -> Result<()> {
            let shell_link: IShellLinkW =
                CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
            let target_w = to_wide(target.as_os_str());
            shell_link.SetPath(PCWSTR(target_w.as_ptr()))?;
            let args_w = to_wide(std::ffi::OsStr::new(super::ARG));
            shell_link.SetArguments(PCWSTR(args_w.as_ptr()))?;
            if let Some(dir) = workdir {
                let dir_w = to_wide(dir.as_os_str());
                shell_link.SetWorkingDirectory(PCWSTR(dir_w.as_ptr()))?;
            }
            let persist: IPersistFile = shell_link.cast()?;
            let link_w = to_wide(link.as_os_str());
            // fremember = TRUE: remember this as the file's path.
            persist.Save(PCWSTR(link_w.as_ptr()), windows::Win32::Foundation::TRUE)?;
            Ok(())
        })();
        if inited {
            CoUninitialize();
        }
        result
    }

    fn to_wide(s: &std::ffi::OsStr) -> Vec<u16> {
        use std::os::windows::ffi::OsStrExt;
        s.encode_wide().chain(std::iter::once(0)).collect()
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use anyhow::{anyhow, Result};
    use std::path::PathBuf;

    fn plist_path() -> Option<PathBuf> {
        let home = std::env::var_os("HOME")?;
        Some(
            PathBuf::from(home)
                .join("Library/LaunchAgents")
                .join("com.snipdeck.snipdeck.plist"),
        )
    }

    pub fn refresh() -> Result<()> {
        Ok(())
    }

    pub fn is_enabled() -> bool {
        plist_path().map(|p| p.exists()).unwrap_or(false)
    }

    pub fn set_enabled(enabled: bool) -> Result<()> {
        let path = plist_path().ok_or_else(|| anyhow!("HOME not found"))?;
        if !enabled {
            if path.exists() {
                std::fs::remove_file(&path)?;
            }
            return Ok(());
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let exe = std::env::current_exe()?;
        let plist = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key><string>com.snipdeck.snipdeck</string>
    <key>ProgramArguments</key>
    <array><string>{}</string><string>{}</string></array>
    <key>RunAtLoad</key><true/>
</dict>
</plist>
"#,
            exe.display(),
            super::ARG
        );
        std::fs::write(&path, plist)?;
        Ok(())
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use anyhow::{anyhow, Result};
    use std::path::PathBuf;

    fn autostart_dir() -> Option<PathBuf> {
        let dir = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
        Some(dir.join("autostart"))
    }

    /// Named after the application id: GNOME and KDE start an autostart entry
    /// in a scope named after the file, and that name is the id the portal
    /// gives the app.
    fn desktop_path() -> Option<PathBuf> {
        Some(autostart_dir()?.join(format!("{}.desktop", crate::APP_ID)))
    }

    /// The entry's name before the application id was reverse-DNS.
    fn old_desktop_path() -> Option<PathBuf> {
        Some(autostart_dir()?.join("snipdeck.desktop"))
    }

    /// An entry under the old name becomes one under the new: "Launch at
    /// login" stays on, and a login does not start Snipdeck twice.
    fn migrate() {
        let (Some(old), Some(new)) = (old_desktop_path(), desktop_path()) else { return };
        if old.exists() {
            let moved = if new.exists() {
                std::fs::remove_file(&old)
            } else {
                std::fs::rename(&old, &new)
            };
            if let Err(e) = moved {
                log::warn!("could not move the old launch-at-login entry: {e}");
            }
        }
    }

    pub fn is_enabled() -> bool {
        migrate();
        desktop_path().map(|p| p.exists()).unwrap_or(false)
    }

    pub fn set_enabled(enabled: bool) -> Result<()> {
        migrate();
        let path = desktop_path().ok_or_else(|| anyhow!("config directory not found"))?;
        if !enabled {
            if path.exists() {
                std::fs::remove_file(&path)?;
            }
            return Ok(());
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, entry(&std::env::current_exe()?))?;
        Ok(())
    }

    pub fn refresh() -> Result<()> {
        migrate();
        let Some(path) = desktop_path() else { return Ok(()) };
        let Ok(current) = std::fs::read_to_string(&path) else { return Ok(()) };
        let wanted = entry(&std::env::current_exe()?);
        if current != wanted {
            std::fs::write(&path, wanted)?;
            log::info!("launch-at-login entry pointed at this executable again");
        }
        Ok(())
    }

    fn entry(exe: &std::path::Path) -> String {
        format!(
            "[Desktop Entry]\nType=Application\nName=Snipdeck\nIcon=snipdeck\nExec={} {}\nX-GNOME-Autostart-enabled=true\nTerminal=false\n",
            exec_arg(&exe.to_string_lossy()),
            super::ARG
        )
    }

    /// One argument for an `Exec` key, as the Desktop Entry spec wants it.
    ///
    /// Written bare, a path with a space in it was two arguments, and the
    /// login autostart ran nothing. Anything beyond plain path characters is
    /// quoted: `"`, `` ` ``, `$` and `\` get a backslash (the quoting rule),
    /// then every backslash is doubled (the string rule, which readers undo
    /// first) — and `%` is doubled throughout, or it reads as a field code.
    pub(super) fn exec_arg(arg: &str) -> String {
        let arg = arg.replace('%', "%%");
        let plain = !arg.is_empty()
            && arg
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "/._-+,:@%".contains(c));
        if plain {
            return arg;
        }
        let mut quoted = String::from("\"");
        for c in arg.chars() {
            if matches!(c, '"' | '`' | '$' | '\\') {
                quoted.push('\\');
            }
            quoted.push(c);
        }
        quoted.push('"');
        quoted.replace('\\', "\\\\")
    }

    #[cfg(test)]
    mod tests {
        use super::{entry, exec_arg};

        #[test]
        fn a_login_start_is_marked() {
            let e = entry(std::path::Path::new("/home/u/.local/bin/snipdeck"));
            assert!(e.contains("\nExec=/home/u/.local/bin/snipdeck --autostart\n"), "{e}");
        }

        #[test]
        fn a_plain_path_is_left_bare() {
            assert_eq!(exec_arg("/home/u/.local/bin/snipdeck"), "/home/u/.local/bin/snipdeck");
        }

        #[test]
        fn spaces_and_specials_are_quoted_and_escaped() {
            assert_eq!(exec_arg("/home/a b/snipdeck"), "\"/home/a b/snipdeck\"");
            assert_eq!(exec_arg("/opt/$x/snip"), "\"/opt/\\\\$x/snip\"");
            assert_eq!(exec_arg("/a\\b c"), "\"/a\\\\\\\\b c\"");
            assert_eq!(exec_arg("/100% sure/snip"), "\"/100%% sure/snip\"");
        }
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
mod imp {
    pub fn refresh() -> anyhow::Result<()> {
        Ok(())
    }
    pub fn is_enabled() -> bool {
        false
    }
    pub fn set_enabled(_enabled: bool) -> anyhow::Result<()> {
        Ok(())
    }
}
