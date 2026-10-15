//! Turning on the desktop's half of Snipdeck: the GNOME Shell extension, or
//! the KWin script on KDE Plasma.
//!
//! Both do what a Wayland client may not — keep snips above other windows,
//! place them where they were captured, keep them out of the taskbar — and
//! both are installed by the packages system-wide, where nothing turns them on:
//! a desktop enables an extension or a script per user. `install.sh` enables
//! what it installs; this is the same for a package, at the app's start.
//!
//! Only once, in effect: each desktop records a helper the user turned off
//! (GNOME's `disabled-extensions`, KWin's `snipdeckEnabled=false`), and a
//! helper the user turned off is left off.

use std::path::{Path, PathBuf};
use std::process::Command;

const GNOME_UUID: &str = "snipdeck@revisetouch.com";
const KWIN_SCRIPT: &str = "snipdeck";

/// Check and enable, on a thread of its own: it runs a few small commands.
pub fn spawn() {
    let compositor = crate::window_env::caps().compositor;
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    let _ = std::thread::Builder::new()
        .name("desktop-helper".into())
        .spawn(move || {
            if compositor == crate::window_env::Compositor::Gnome || has(&desktop, "GNOME") {
                gnome();
            } else if has(&desktop, "KDE") {
                kwin();
            }
        });
}

fn has(desktops: &str, name: &str) -> bool {
    desktops.split(':').any(|d| d.eq_ignore_ascii_case(name))
}

/// `$XDG_DATA_HOME` first, then `$XDG_DATA_DIRS`, each joined with `rel`.
fn data_dirs(rel: &str) -> Vec<PathBuf> {
    let home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")));
    let system = std::env::var("XDG_DATA_DIRS")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    home.into_iter()
        .chain(system.split(':').map(PathBuf::from))
        .map(|d| d.join(rel))
        .collect()
}

fn installed(rel: &str) -> bool {
    data_dirs(rel).iter().any(|d| d.is_dir())
}

fn output(cmd: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(cmd).args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn gnome() {
    if !installed(&format!("gnome-shell/extensions/{GNOME_UUID}")) {
        return;
    }
    let listed = |key: &str| {
        output("gsettings", &["get", "org.gnome.shell", key])
            .is_some_and(|v| v.contains(&format!("'{GNOME_UUID}'")))
    };
    if listed("enabled-extensions") || listed("disabled-extensions") {
        return;
    }
    // Through the shell when it already knows the extension, so it loads now.
    if output("gnome-extensions", &["enable", GNOME_UUID]).is_some() {
        log::info!("GNOME extension {GNOME_UUID} enabled");
        return;
    }
    // A shell started before the package was installed has not seen it, and
    // refuses; in the list, it loads at the next login.
    let current = output("gsettings", &["get", "org.gnome.shell", "enabled-extensions"])
        .unwrap_or_default();
    let next = with_item(&current, GNOME_UUID);
    if output("gsettings", &["set", "org.gnome.shell", "enabled-extensions", &next]).is_some() {
        log::info!("GNOME extension {GNOME_UUID} enabled from the next login on");
    }
}

/// A GVariant string list with `item` added: `@as []` and `['a']` both.
fn with_item(list: &str, item: &str) -> String {
    let inner = list
        .trim()
        .trim_start_matches("@as")
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim();
    if inner.is_empty() {
        format!("['{item}']")
    } else {
        format!("[{inner}, '{item}']")
    }
}

fn kwin() {
    if !installed(&format!("kwin/scripts/{KWIN_SCRIPT}")) {
        return;
    }
    let key = format!("{KWIN_SCRIPT}Enabled");
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")));
    let Some(kwinrc) = config.map(|c| c.join("kwinrc")) else { return };
    if plugin_setting(&kwinrc, &key).is_some() {
        return; // on already, or turned off by the user
    }
    let wrote = ["kwriteconfig6", "kwriteconfig5"].iter().any(|tool| {
        output(tool, &["--file", "kwinrc", "--group", "Plugins", "--key", &key, "true"]).is_some()
    });
    if !wrote {
        log::warn!("could not enable the KWin script: no kwriteconfig");
        return;
    }
    // KWin loads scripts on reconfigure: no need to log out.
    let reconfigured = output(
        "dbus-send",
        &["--session", "--type=method_call", "--dest=org.kde.KWin", "/KWin", "org.kde.KWin.reconfigure"],
    )
    .is_some();
    log::info!("KWin script {KWIN_SCRIPT} enabled (reloaded now: {reconfigured})");
}

/// `key` in the `[Plugins]` group of a KDE config file.
fn plugin_setting(path: &Path, key: &str) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut in_plugins = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_plugins = line == "[Plugins]";
        } else if in_plugins {
            if let Some((k, v)) = line.split_once('=') {
                if k.trim() == key {
                    return Some(v.trim().to_string());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_item_joins_an_empty_or_a_full_list() {
        assert_eq!(with_item("@as []", "x@y"), "['x@y']");
        assert_eq!(with_item("['a@b']", "x@y"), "['a@b', 'x@y']");
    }

    #[test]
    fn a_plugin_setting_is_read_from_its_group_only() {
        let dir = std::env::temp_dir().join(format!("sd-kwinrc-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let rc = dir.join("kwinrc");
        std::fs::write(&rc, "[Other]\nsnipdeckEnabled=x\n\n[Plugins]\nfooEnabled=true\nsnipdeckEnabled=false\n").unwrap();
        assert_eq!(plugin_setting(&rc, "snipdeckEnabled").as_deref(), Some("false"));
        assert_eq!(plugin_setting(&rc, "barEnabled"), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn desktops_are_matched_whole() {
        assert!(has("ubuntu:GNOME", "GNOME"));
        assert!(!has("GNOME-Flashback", "GNOME"));
        assert!(has("KDE", "KDE"));
    }
}
