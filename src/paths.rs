use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const APP_DIR: &str = "Snipdeck";
const LEGACY_APP_DIR: &str = "Snipper";

/// What makes a directory the user's, rather than an empty shell.
///
/// Only consulted when both the current and the legacy directory exist, to
/// tell "scaffolding something created early" from "a second real dataset".
/// A directory entry counts only when it has something in it, because
/// `ensure_dirs` creates the whole `cache` tree empty.
const DATA_MARKERS: &[&str] = &["snips.db", "state.json", "cache/images"];
const CONFIG_MARKERS: &[&str] = &["settings.json"];

fn data_base_dir() -> PathBuf {
    dirs::data_local_dir()
        .or_else(dirs::data_dir)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn config_base_dir() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from("."))
}

/// Resolved once per process, migration included.
///
/// This used to be recomputed on every path lookup — two `exists()` calls
/// each, and `image_path()` runs once per snip per gallery refresh. The
/// reason to cache it is not speed, though: it is that the migration below
/// now happens inside the resolution, so it runs exactly once, from whichever
/// call comes first. No caller has to remember to reach `ensure_dirs()`
/// before asking for a path, which is the ordering the old code depended on
/// and did not get.
static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();
static CONFIG_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn data_dir() -> PathBuf {
    DATA_DIR
        .get_or_init(|| resolve_app_dir(data_base_dir(), DATA_MARKERS))
        .clone()
}

pub fn config_dir() -> PathBuf {
    CONFIG_DIR
        .get_or_init(|| resolve_app_dir(config_base_dir(), CONFIG_MARKERS))
        .clone()
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

pub fn state_file() -> PathBuf {
    data_dir().join("state.json")
}

pub fn db_file() -> PathBuf {
    data_dir().join("snips.db")
}

pub fn settings_file() -> PathBuf {
    config_dir().join("settings.json")
}

pub fn ensure_dirs() -> std::io::Result<()> {
    // `data_dir()` and `config_dir()` carry out the legacy migration on their
    // first call, so by the time this returns a path there is nothing left to
    // migrate — only directories to create.
    std::fs::create_dir_all(data_dir())?;
    std::fs::create_dir_all(config_dir())?;
    std::fs::create_dir_all(cache_dir())?;
    std::fs::create_dir_all(images_dir())?;
    std::fs::create_dir_all(thumbs_dir())?;
    Ok(())
}

/// Pick the app directory under `base`, moving the legacy `Snipper` name onto
/// the current `Snipdeck` one on the way.
///
/// The case that lost data was "both directories exist". That used to be read
/// as "the migration already happened", and the current directory was
/// returned — but an *empty* `Snipdeck` appears the moment anything writes
/// under `data_dir()` before the migration runs, and something does: the panic
/// handler creates it to append `crash.log`. One panic during startup and the
/// migration was skipped from then on, permanently. The user's snips stayed
/// behind in `Snipper`, the gallery came up empty, and nothing anywhere said
/// why. So "both exist" is now settled by what is inside them, not by the fact
/// that they are there.
fn resolve_app_dir(base: PathBuf, markers: &[&str]) -> PathBuf {
    let current = base.join(APP_DIR);
    let legacy = base.join(LEGACY_APP_DIR);

    if !legacy.exists() {
        return current;
    }

    if !current.exists() {
        return match std::fs::rename(&legacy, &current) {
            Ok(()) => {
                log::info!("migrated {} to {}", legacy.display(), current.display());
                current
            }
            // Keep using the old directory rather than starting empty beside
            // it. A rename we could not perform is no reason to hide the data
            // it was going to move; the next launch tries again.
            Err(err) => {
                log::warn!(
                    "could not rename {} to {} ({err}); continuing to use the old one",
                    legacy.display(),
                    current.display()
                );
                legacy
            }
        };
    }

    if holds_user_data(&current, markers) {
        log::warn!(
            "both {} and {} hold data; using the first and leaving the second alone",
            current.display(),
            legacy.display()
        );
        return current;
    }

    // An empty shell. `rename` will not put a directory onto one that already
    // exists, so the contents move across instead.
    match merge_into(&legacy, &current) {
        Ok(()) => log::info!(
            "migrated the contents of {} into {}",
            legacy.display(),
            current.display()
        ),
        Err(err) => log::warn!(
            "could not fully merge {} into {} ({err}); what is left is still there",
            legacy.display(),
            current.display()
        ),
    }
    current
}

/// Does this directory hold anything of the user's own?
///
/// Deliberately narrow: `crash.log` and an empty `cache` tree are what a
/// stray early write and `ensure_dirs` leave behind, and neither should be
/// read as "there is a second dataset here, do not touch it".
fn holds_user_data(dir: &Path, markers: &[&str]) -> bool {
    markers.iter().any(|marker| {
        let path = dir.join(marker);
        match std::fs::metadata(&path) {
            Ok(meta) if meta.is_dir() => std::fs::read_dir(&path)
                .map(|mut entries| entries.next().is_some())
                .unwrap_or(false),
            Ok(_) => true,
            Err(_) => false,
        }
    })
}

/// Move everything in `from` into `to`, recursing where both sides have a
/// directory of the same name.
///
/// Nothing is ever overwritten and nothing non-empty is ever removed: a name
/// present on both sides is left where it is, and the `remove_dir` calls
/// refuse to run on a directory that still has anything in it. A merge that
/// only half finished therefore shows up as data still sitting in `from`,
/// which is recoverable, rather than as data that was deleted after being
/// copied, which is not.
fn merge_into(from: &Path, to: &Path) -> std::io::Result<()> {
    move_contents(from, to)?;
    // Succeeds only once every entry has moved out. A directory that is still
    // standing afterwards is the report that something stayed behind.
    std::fs::remove_dir(from)
}

fn move_contents(from: &Path, to: &Path) -> std::io::Result<()> {
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let src = entry.path();
        let dst = to.join(entry.file_name());
        if src.is_dir() && dst.is_dir() {
            move_contents(&src, &dst)?;
            // Best-effort, and harmless when a collision left something in
            // there: an emptied directory is tidied up, a used one stays.
            // Failing here must not abandon the entries still to come.
            let _ = std::fs::remove_dir(&src);
        } else if !dst.exists() {
            std::fs::rename(&src, &dst)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "snipdeck-paths-{}-{tag}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn touch(path: PathBuf) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"x").unwrap();
    }

    #[test]
    fn a_lone_legacy_directory_is_renamed() {
        let base = scratch("lone-legacy");
        touch(base.join(LEGACY_APP_DIR).join("snips.db"));

        let resolved = resolve_app_dir(base.clone(), DATA_MARKERS);

        assert_eq!(resolved, base.join(APP_DIR));
        assert!(base.join(APP_DIR).join("snips.db").exists());
        assert!(!base.join(LEGACY_APP_DIR).exists());
        let _ = std::fs::remove_dir_all(&base);
    }

    /// The regression this whole function exists for.
    ///
    /// A panic during startup makes the crash handler create the new directory
    /// to append `crash.log` to, and `ensure_dirs` then fills it with the empty
    /// `cache` tree. The old code saw that directory, concluded the migration
    /// was done, and left every snip behind in `Snipper` — silently, and from
    /// then on permanently.
    #[test]
    fn an_empty_shell_does_not_strand_the_legacy_data() {
        let base = scratch("empty-shell");
        touch(base.join(APP_DIR).join("crash.log"));
        std::fs::create_dir_all(base.join(APP_DIR).join("cache/images")).unwrap();
        std::fs::create_dir_all(base.join(APP_DIR).join("cache/thumbs")).unwrap();
        touch(base.join(LEGACY_APP_DIR).join("snips.db"));
        touch(base.join(LEGACY_APP_DIR).join("cache/images/a.png"));

        let resolved = resolve_app_dir(base.clone(), DATA_MARKERS);

        assert_eq!(resolved, base.join(APP_DIR));
        assert!(base.join(APP_DIR).join("snips.db").exists(), "the database");
        assert!(
            base.join(APP_DIR).join("cache/images/a.png").exists(),
            "images move even though the destination directory already existed"
        );
        assert!(base.join(APP_DIR).join("crash.log").exists(), "kept, not clobbered");
        assert!(!base.join(LEGACY_APP_DIR).exists(), "emptied, so tidied away");
        let _ = std::fs::remove_dir_all(&base);
    }

    /// Two real datasets are not something to reconcile automatically.
    #[test]
    fn two_populated_directories_are_both_left_intact() {
        let base = scratch("two-datasets");
        touch(base.join(APP_DIR).join("snips.db"));
        touch(base.join(LEGACY_APP_DIR).join("snips.db"));

        let resolved = resolve_app_dir(base.clone(), DATA_MARKERS);

        assert_eq!(resolved, base.join(APP_DIR));
        assert!(
            base.join(LEGACY_APP_DIR).join("snips.db").exists(),
            "the one we are not using must survive for the user to recover"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_fresh_install_just_gets_the_current_name() {
        let base = scratch("fresh");

        let resolved = resolve_app_dir(base.clone(), DATA_MARKERS);

        assert_eq!(resolved, base.join(APP_DIR));
        let _ = std::fs::remove_dir_all(&base);
    }
}
