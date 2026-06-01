//! Lightweight runtime i18n for the Rust side, sharing the SAME bundled gettext
//! `.po` catalogs the Slint UI uses (`@tr` + `with_bundled_translations`).
//!
//! English is the source language: msgids ARE the English strings, so [`t`]
//! returns its argument unchanged when the active language is English or when a
//! translation is missing (graceful fallback). Every other language ships one
//! `lang/<code>/LC_MESSAGES/snipdeck.po`, embedded here via `include_str!` and
//! parsed lazily into a `msgid -> msgstr` map the first time that language is
//! used. Adding a language = drop in its `.po` + one line in `po_source` +
//! `LANGUAGES`; adding a string = wrap it in `@tr(...)` / `t("...")` and add the
//! msgid to each `.po`.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Supported UI languages as `(code, endonym)`. English is first (the source).
/// `code` must match the folder under `lang/` and the value passed to
/// [`slint::select_bundled_translation`]. The endonym is shown in the toolbar
/// language menu.
pub const LANGUAGES: &[(&str, &str)] = &[
    ("en", "English"),
    ("es", "Español"),
    ("zh-CN", "简体中文"),
    ("hi", "हिन्दी"),
    ("ar", "العربية"),
    ("pt", "Português"),
    ("ru", "Русский"),
    ("ja", "日本語"),
    ("de", "Deutsch"),
    ("fr", "Français"),
    ("ko", "한국어"),
    ("it", "Italiano"),
    ("tr", "Türkçe"),
    ("id", "Bahasa Indonesia"),
    ("vi", "Tiếng Việt"),
    ("pl", "Polski"),
    ("uk", "Українська"),
    ("nl", "Nederlands"),
    ("fa", "فارسی"),
    ("th", "ไทย"),
    ("bn", "বাংলা"),
    ("ur", "اردو"),
    ("ro", "Română"),
];

/// The embedded `.po` source for a language. `None` for English (the source
/// needs no catalog) and anything unsupported.
fn po_source(code: &str) -> Option<&'static str> {
    Some(match code {
        "es" => include_str!("../lang/es/LC_MESSAGES/snipdeck.po"),
        "zh-CN" => include_str!("../lang/zh-CN/LC_MESSAGES/snipdeck.po"),
        "hi" => include_str!("../lang/hi/LC_MESSAGES/snipdeck.po"),
        "ar" => include_str!("../lang/ar/LC_MESSAGES/snipdeck.po"),
        "pt" => include_str!("../lang/pt/LC_MESSAGES/snipdeck.po"),
        "ru" => include_str!("../lang/ru/LC_MESSAGES/snipdeck.po"),
        "ja" => include_str!("../lang/ja/LC_MESSAGES/snipdeck.po"),
        "de" => include_str!("../lang/de/LC_MESSAGES/snipdeck.po"),
        "fr" => include_str!("../lang/fr/LC_MESSAGES/snipdeck.po"),
        "ko" => include_str!("../lang/ko/LC_MESSAGES/snipdeck.po"),
        "it" => include_str!("../lang/it/LC_MESSAGES/snipdeck.po"),
        "tr" => include_str!("../lang/tr/LC_MESSAGES/snipdeck.po"),
        "id" => include_str!("../lang/id/LC_MESSAGES/snipdeck.po"),
        "vi" => include_str!("../lang/vi/LC_MESSAGES/snipdeck.po"),
        "pl" => include_str!("../lang/pl/LC_MESSAGES/snipdeck.po"),
        "uk" => include_str!("../lang/uk/LC_MESSAGES/snipdeck.po"),
        "nl" => include_str!("../lang/nl/LC_MESSAGES/snipdeck.po"),
        "fa" => include_str!("../lang/fa/LC_MESSAGES/snipdeck.po"),
        "th" => include_str!("../lang/th/LC_MESSAGES/snipdeck.po"),
        "bn" => include_str!("../lang/bn/LC_MESSAGES/snipdeck.po"),
        "ur" => include_str!("../lang/ur/LC_MESSAGES/snipdeck.po"),
        "ro" => include_str!("../lang/ro/LC_MESSAGES/snipdeck.po"),
        _ => return None,
    })
}

static CURRENT: RwLock<&'static str> = RwLock::new("en");

fn catalogs() -> &'static RwLock<HashMap<&'static str, HashMap<String, String>>> {
    static C: OnceLock<RwLock<HashMap<&'static str, HashMap<String, String>>>> = OnceLock::new();
    C.get_or_init(|| RwLock::new(HashMap::new()))
}

/// The canonical `&'static` code for a (possibly differently-cased) language
/// code, or `"en"` if unsupported.
fn canonical(code: &str) -> &'static str {
    LANGUAGES
        .iter()
        .map(|(c, _)| *c)
        .find(|c| c.eq_ignore_ascii_case(code))
        .unwrap_or("en")
}

/// Is `code` one of the supported UI languages?
pub fn is_supported(code: &str) -> bool {
    LANGUAGES.iter().any(|(c, _)| c.eq_ignore_ascii_case(code))
}

/// Resolve a settings value (`"auto"` or a code) plus the OS locale into a
/// supported language code.
pub fn resolve_language(setting: &str) -> &'static str {
    if !setting.eq_ignore_ascii_case("auto") && is_supported(setting) {
        return canonical(setting);
    }
    match sys_locale::get_locale() {
        Some(loc) => match_locale(&loc),
        None => "en",
    }
}

/// Best-effort map of an OS locale tag (e.g. `"en-US"`, `"tr-TR"`,
/// `"zh-Hans-CN"`) to a supported code, falling back to English.
fn match_locale(loc: &str) -> &'static str {
    let lower = loc.to_ascii_lowercase();
    if lower.starts_with("zh") {
        return "zh-CN";
    }
    let primary = lower.split(['-', '_']).next().unwrap_or("");
    canonical(primary)
}

/// Set the active UI language (used by the Rust side). Pre-parses the catalog so
/// the first [`t`] is cheap. Pair this with [`slint::select_bundled_translation`]
/// for the Slint UI.
pub fn set_language(code: &str) {
    let canon = canonical(code);
    *CURRENT.write() = canon;
    if canon != "en" {
        ensure_loaded(canon);
    }
}

/// The active language code.
pub fn current() -> &'static str {
    *CURRENT.read()
}

fn ensure_loaded(code: &'static str) {
    if catalogs().read().contains_key(code) {
        return;
    }
    let map = po_source(code).map(parse_po).unwrap_or_default();
    catalogs().write().entry(code).or_insert(map);
}

/// Translate `msgid` into the active language. Returns `msgid` unchanged for
/// English or when no (non-empty) translation exists.
pub fn t(msgid: &str) -> String {
    let code = current();
    if code == "en" {
        return msgid.to_string();
    }
    ensure_loaded(code);
    let cats = catalogs().read();
    if let Some(s) = cats.get(code).and_then(|m| m.get(msgid)) {
        if !s.is_empty() {
            return s.clone();
        }
    }
    msgid.to_string()
}

/// [`t`] with a single literal placeholder (e.g. `"{e}"`) replaced by `val`.
pub fn t1(msgid: &str, key: &str, val: &str) -> String {
    t(msgid).replace(key, val)
}

/// Minimal gettext `.po` parser → `msgid -> msgstr` map. Handles standard
/// `msgid` / `msgstr` pairs with C-style escapes and multi-line continuation
/// (consecutive quoted lines). Ignores comments, the header (empty msgid),
/// plurals, and `msgctxt` (our build disables translation context).
fn parse_po(src: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut id = String::new();
    let mut s = String::new();
    let mut have_id = false;
    let mut in_str = false;
    for raw in src.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("msgid ") {
            if have_id && !id.is_empty() {
                map.insert(id.clone(), s.clone());
            }
            id = unquote(rest);
            s.clear();
            have_id = true;
            in_str = false;
        } else if let Some(rest) = line.strip_prefix("msgstr ") {
            s = unquote(rest);
            in_str = true;
        } else if line.starts_with('"') {
            if in_str {
                s.push_str(&unquote(line));
            } else if have_id {
                id.push_str(&unquote(line));
            }
        } else if line.starts_with("msgctxt ") {
            if have_id && !id.is_empty() {
                map.insert(id.clone(), s.clone());
            }
            id.clear();
            s.clear();
            have_id = false;
            in_str = false;
        }
    }
    if have_id && !id.is_empty() {
        map.insert(id, s);
    }
    map
}

/// Strip the surrounding quotes from a `.po` string literal and unescape the
/// C-style escapes gettext uses (`\n \r \t \" \\`).
fn unquote(s: &str) -> String {
    let s = s.trim();
    let inner = s
        .strip_prefix('"')
        .and_then(|x| x.strip_suffix('"'))
        .unwrap_or(s);
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some(other) => out.push(other),
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}
