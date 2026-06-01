use std::path::PathBuf;

const APP_DIR: &str = "Snipdeck";
const LEGACY_APP_DIR: &str = "Snipper";

fn data_base_dir() -> PathBuf {
    dirs::data_local_dir()
        .or_else(dirs::data_dir)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn config_base_dir() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from("."))
}

fn active_app_dir(base: PathBuf) -> PathBuf {
    let current = base.join(APP_DIR);
    let legacy = base.join(LEGACY_APP_DIR);
    if current.exists() || !legacy.exists() {
        current
    } else {
        legacy
    }
}

pub fn data_dir() -> PathBuf {
    active_app_dir(data_base_dir())
}

pub fn config_dir() -> PathBuf {
    active_app_dir(config_base_dir())
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
    migrate_legacy_dir(data_base_dir());
    migrate_legacy_dir(config_base_dir());
    std::fs::create_dir_all(data_dir())?;
    std::fs::create_dir_all(config_dir())?;
    std::fs::create_dir_all(cache_dir())?;
    std::fs::create_dir_all(images_dir())?;
    std::fs::create_dir_all(thumbs_dir())?;
    Ok(())
}

fn migrate_legacy_dir(base: PathBuf) {
    let current = base.join(APP_DIR);
    let legacy = base.join(LEGACY_APP_DIR);
    if current.exists() || !legacy.exists() {
        return;
    }
    if let Err(err) = std::fs::rename(&legacy, &current) {
        log::warn!(
            "legacy app directory migration failed: {} -> {}: {err}",
            legacy.display(),
            current.display()
        );
    }
}
