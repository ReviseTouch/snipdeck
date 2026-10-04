use std::path::PathBuf;
use std::sync::OnceLock;

const APP_DIR: &str = "Snipdeck";
fn data_base_dir() -> PathBuf {
    dirs::data_local_dir()
        .or_else(dirs::data_dir)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn config_base_dir() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from("."))
}

/// Resolved once per process: `image_path()` runs once per snip per gallery
/// refresh, and the answer never changes.
static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();
static CONFIG_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn data_dir() -> PathBuf {
    DATA_DIR.get_or_init(|| data_base_dir().join(APP_DIR)).clone()
}

pub fn config_dir() -> PathBuf {
    CONFIG_DIR.get_or_init(|| config_base_dir().join(APP_DIR)).clone()
}

pub fn cache_dir() -> PathBuf {
    data_dir().join("cache")
}

pub fn images_dir() -> PathBuf {
    cache_dir().join("images")
}

pub fn thumbs_dir() -> PathBuf {
    cache_dir().join("thumbs")
}

pub fn default_save_dir() -> PathBuf {
    dirs::picture_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")))
        .join(APP_DIR)
}

pub fn db_file() -> PathBuf {
    data_dir().join("snips.db")
}

pub fn settings_file() -> PathBuf {
    config_dir().join("settings.json")
}

/// A directory for short-lived copies of snips handed to other programs —
/// a share, an e-mail attachment — that only this user can read.
///
/// Not the shared temp directory: on Linux /tmp is world-writable, files land
/// there 0644 by default, and another local user can create a predictable name
/// first. `$XDG_RUNTIME_DIR` is the user's own tmpfs, mode 0700 and cleared at
/// logout; the directory inside it is created 0700 too. Falls back to the
/// system temp dir only where no runtime dir exists.
#[cfg(unix)]
pub fn private_temp_dir() -> std::io::Result<PathBuf> {
    use std::os::unix::fs::DirBuilderExt;
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .unwrap_or_else(std::env::temp_dir);
    // SAFETY: getuid has no preconditions and cannot fail.
    let dir = base.join(format!("snipdeck-{}", unsafe { libc::getuid() }));
    match std::fs::DirBuilder::new().mode(0o700).create(&dir) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e),
    }
    // In the fallback case someone else may have created the name first; use
    // it only if it is ours and private.
    use std::os::unix::fs::MetadataExt;
    let meta = std::fs::symlink_metadata(&dir)?;
    // SAFETY: as above.
    if !meta.is_dir() || meta.uid() != unsafe { libc::getuid() } || meta.mode() & 0o077 != 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!("{} is not a private directory", dir.display()),
        ));
    }
    Ok(dir)
}

pub fn ensure_dirs() -> std::io::Result<()> {
    std::fs::create_dir_all(data_dir())?;
    std::fs::create_dir_all(config_dir())?;
    std::fs::create_dir_all(cache_dir())?;
    std::fs::create_dir_all(images_dir())?;
    std::fs::create_dir_all(thumbs_dir())?;
    Ok(())
}
