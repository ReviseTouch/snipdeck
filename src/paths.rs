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
    create_private_dir(&data_dir())?;
    create_private_dir(&config_dir())?;
    std::fs::create_dir_all(cache_dir())?;
    std::fs::create_dir_all(images_dir())?;
    std::fs::create_dir_all(thumbs_dir())?;
    // Once per process: this runs for every database connection, and an
    // earlier install's directories need tightening only the first time.
    static TIGHTENED: OnceLock<()> = OnceLock::new();
    TIGHTENED.get_or_init(|| {
        for dir in [data_dir(), config_dir()] {
            if let Err(e) = make_private(&dir) {
                log::warn!("could not make {} private: {e}", dir.display());
            }
        }
    });
    Ok(())
}

/// Snipdeck's own data and config directories, readable by this user only.
///
/// What is inside is private by nature: every screenshot, the text OCR read
/// from them, the titles of the windows they came from, and settings.json
/// with the tokens of any imported uploader. Created with the umask's 0755,
/// all of it was open to every local account wherever the home directory
/// itself is not closed — still the default on several distributions. 0700
/// on the two top directories covers everything beneath them, the SQLite
/// database and its journal files included, so the files keep their usual
/// modes and nothing the user saves elsewhere is affected.
///
/// Windows: the per-user AppData directories are already closed by their ACL.
#[cfg(unix)]
fn create_private_dir(dir: &std::path::Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    std::fs::DirBuilder::new().recursive(true).mode(0o700).create(dir)
}

#[cfg(not(unix))]
fn create_private_dir(dir: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)
}

/// Close a directory an earlier version created open. Leaves one that is
/// already private alone; an error here (a filesystem without Unix modes)
/// is reported, not fatal.
#[cfg(unix)]
fn make_private(dir: &std::path::Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mode = std::fs::metadata(dir)?.permissions().mode();
    if mode & 0o077 != 0 {
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(mode & 0o7700))?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn make_private(_dir: &std::path::Path) -> std::io::Result<()> {
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn mode(dir: &std::path::Path) -> u32 {
        std::fs::metadata(dir).unwrap().permissions().mode() & 0o7777
    }

    #[test]
    fn app_directories_are_created_and_tightened_private() {
        let base = std::env::temp_dir().join(format!("snipdeck-paths-{}", uuid::Uuid::new_v4()));

        let fresh = base.join("new").join("Snipdeck");
        create_private_dir(&fresh).unwrap();
        assert_eq!(mode(&fresh), 0o700);

        // An earlier install's directory, left open by the umask.
        let old = base.join("old");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::set_permissions(&old, std::fs::Permissions::from_mode(0o755)).unwrap();
        make_private(&old).unwrap();
        assert_eq!(mode(&old), 0o700);
        // Already private: untouched, and calling again is harmless.
        make_private(&old).unwrap();
        assert_eq!(mode(&old), 0o700);

        std::fs::remove_dir_all(&base).unwrap();
    }
}
