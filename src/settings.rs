use serde::{Deserialize, Serialize};

fn default_freeze() -> bool {
    false
}

fn default_live_overlay_migration() -> bool {
    false
}

fn default_floating_in_taskbar() -> bool {
    false
}

fn default_dark_theme() -> bool {
    true
}

fn default_translate_target() -> String {
    "en".into()
}

fn default_translate_favs() -> Vec<String> {
    ["en", "es", "fr", "de", "it", "pt", "ru", "ar"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

fn default_upload_service() -> String {
    "catbox".into()
}

fn default_imgur_client_id() -> String {
    String::new()
}

fn default_language() -> String {
    "auto".into()
}

fn default_thumb_size_index() -> i32 {
    1 // Medium
}
use std::path::PathBuf;

use crate::paths;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub save_dir: PathBuf,
    pub default_format: String, // "png" | "jpg" | "bmp"
    pub border_color_active: [u8; 3],
    pub border_color_inactive: [u8; 3],
    pub selection_tint: [u8; 4], // RGBA for the dim overlay outside selection
    pub include_border_on_save: bool,
    pub include_border_on_clipboard: bool,
    /// When true, the selection drag happens on a frozen snapshot of the
    /// screen rather than the live desktop. Visually identical, but
    /// eliminates compositor flicker and lets you capture moving content
    /// without motion artefacts.
    #[serde(default = "default_freeze")]
    pub freeze_screen_during_snip: bool,
    /// Internal one-shot migration: older builds defaulted to frozen
    /// full-screen snapshots, which can visibly blink on Win11.
    #[serde(default = "default_live_overlay_migration")]
    pub live_overlay_default_migrated: bool,
    pub hotkey_snip: String,
    pub hotkey_snip_clip: String,
    pub hotkey_clip_only: String,
    pub hotkey_toggle_visibility: String,
    pub hotkey_gallery: String,
    /// When false (default), floating snip windows get WS_EX_TOOLWINDOW
    /// applied so they don't clutter the taskbar / alt-tab list while
    /// still floating on top of everything. Toggle from the toolbar.
    #[serde(default = "default_floating_in_taskbar")]
    pub floating_in_taskbar: bool,
    /// Visual theme for the Slint shell.
    #[serde(default = "default_dark_theme")]
    pub dark_theme: bool,
    /// Last target language picked in the OCR-translate popup, remembered
    /// across sessions (an endpoint code like "tr"/"en"). There is no
    /// settings window — this is set straight from the popup's language strip.
    #[serde(default = "default_translate_target")]
    pub translate_target: String,
    /// Favourite target languages shown as quick-pick pills in the translate
    /// popup. Editable straight from the popup (star a row in "More"); the
    /// order is the pill order. Never empty (the popup keeps at least one).
    #[serde(default = "default_translate_favs")]
    pub translate_favs: Vec<String>,
    /// Which host the "Upload" action uploads to: "catbox" (default, no key /
    /// no login), "imgur" (needs `imgur_client_id`), or "0x0". Anything
    /// unknown falls back to catbox.
    #[serde(default = "default_upload_service")]
    pub upload_service: String,
    /// Imgur "Client-ID" for anonymous uploads when `upload_service` is "imgur".
    /// Empty by default; the user registers a free application at imgur.com
    /// (Settings → Applications → Register, "OAuth 2 without a callback URL")
    /// and pastes the Client-ID here. No user login or Client-Secret is needed.
    #[serde(default = "default_imgur_client_id")]
    pub imgur_client_id: String,
    /// UI language: "auto" (detect from the OS locale) or a supported code such
    /// as "de" / "zh-CN". Persisted when the user picks one from the toolbar.
    #[serde(default = "default_language")]
    pub language: String,
    /// Gallery card / thumbnail size step: 0 = Small, 1 = Medium (default),
    /// 2 = Large. Set from the filter-bar S/M/L toggle.
    #[serde(default = "default_thumb_size_index")]
    pub thumb_size_index: i32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            save_dir: paths::default_save_dir(),
            default_format: "png".into(),
            border_color_active: [255, 138, 61], // Snipdeck orange
            border_color_inactive: [136, 136, 136],
            selection_tint: [0, 0, 0, 55],
            include_border_on_save: false,
            include_border_on_clipboard: false,
            freeze_screen_during_snip: default_freeze(),
            live_overlay_default_migrated: default_live_overlay_migration(),
            hotkey_snip: "CtrlShift+A".into(),
            hotkey_snip_clip: "CtrlShift+C".into(),
            hotkey_clip_only: "CtrlShift+X".into(),
            hotkey_toggle_visibility: "WinAlt+H".into(),
            hotkey_gallery: "CtrlShift+G".into(),
            floating_in_taskbar: default_floating_in_taskbar(),
            dark_theme: default_dark_theme(),
            translate_target: default_translate_target(),
            translate_favs: default_translate_favs(),
            upload_service: default_upload_service(),
            imgur_client_id: default_imgur_client_id(),
            language: default_language(),
            thumb_size_index: default_thumb_size_index(),
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        let path = paths::settings_file();
        let mut s: Self = match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
            Err(_) => Self::default(),
        };
        // Migrate older builds that wrote a very dark selection tint.
        if s.selection_tint[3] > 100 {
            s.selection_tint[3] = 55;
        }
        if s.border_color_active == [0, 200, 0] {
            s.border_color_active = [255, 138, 61];
        }
        if s.border_color_inactive == [30, 100, 220] {
            s.border_color_inactive = [136, 136, 136];
        }
        if !s.live_overlay_default_migrated {
            s.freeze_screen_during_snip = false;
            s.live_overlay_default_migrated = true;
        }
        // The translate popup always needs at least one quick-pick pill.
        if s.translate_favs.is_empty() {
            s.translate_favs = default_translate_favs();
        }
        // The "show / hide all snips" hotkey moved from Ctrl+Shift+H to
        // Win+Alt+H. Migrate anyone still on the old default so they pick up the
        // new combo (a user who chose a different value of their own is left be).
        if s.hotkey_toggle_visibility == "CtrlShift+H" {
            s.hotkey_toggle_visibility = "WinAlt+H".into();
        }
        s
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = paths::settings_file();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(self)?;
        std::fs::write(&path, bytes)?;
        Ok(())
    }
}
