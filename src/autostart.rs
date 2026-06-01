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

/// Is launch-at-login currently enabled? (Cheap filesystem check.)
pub fn is_enabled() -> bool {
    imp::is_enabled()
}

/// Enable or disable launch-at-login.
pub fn set_enabled(enabled: bool) -> anyhow::Result<()> {
    imp::set_enabled(enabled)
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
    <array><string>{}</string></array>
    <key>RunAtLoad</key><true/>
</dict>
</plist>
"#,
            exe.display()
        );
        std::fs::write(&path, plist)?;
        Ok(())
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use anyhow::{anyhow, Result};
    use std::path::PathBuf;

    fn desktop_path() -> Option<PathBuf> {
        let dir = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
        Some(dir.join("autostart").join("snipdeck.desktop"))
    }

    pub fn is_enabled() -> bool {
        desktop_path().map(|p| p.exists()).unwrap_or(false)
    }

    pub fn set_enabled(enabled: bool) -> Result<()> {
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
        let exe = std::env::current_exe()?;
        let entry = format!(
            "[Desktop Entry]\nType=Application\nName=Snipdeck\nExec={}\nX-GNOME-Autostart-enabled=true\nTerminal=false\n",
            exe.display()
        );
        std::fs::write(&path, entry)?;
        Ok(())
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
mod imp {
    pub fn is_enabled() -> bool {
        false
    }
    pub fn set_enabled(_enabled: bool) -> anyhow::Result<()> {
        Ok(())
    }
}
