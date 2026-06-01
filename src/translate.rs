//! Text translation via Google's unofficial `translate_a` endpoint.
//!
//! Keyless, free, high quality — the same endpoint browser extensions use.
//! It is *unofficial*: Google can change the response shape or rate-limit a
//! hammering IP, so every call returns a `Result` and the UI degrades to an
//! error message rather than crashing. For a personal snip tool the volume is
//! trivial (one short OCR string per click), so this is the pragmatic choice
//! over a self-hosted model or a metered/keyed cloud API.
//!
//! Runs synchronously on the same worker thread the OCR already uses (see
//! `translate_popup`), so no async runtime is pulled in — `ureq` blocks the
//! thread for the ~200 ms round-trip and that's fine off the UI thread.

use anyhow::{anyhow, Context, Result};

/// One translation result: the translated text plus the source language the
/// endpoint auto-detected (a BCP-47-ish code like "en", "de", "zh-CN").
pub struct Translation {
    pub text: String,
    pub source_lang: String,
}

/// Translate `text` into `target` (an endpoint language code, e.g. "tr").
/// Source language is auto-detected (`sl=auto`).
pub fn translate(text: &str, target: &str) -> Result<Translation> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("no text to translate"));
    }

    // A short timeout so a hung request can't strand the worker thread
    // forever; the UI just shows the error and the user can retry.
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(12))
        .build();

    let resp = agent
        .get("https://translate.googleapis.com/translate_a/single")
        // A browser-like UA reduces the chance of an automated-traffic block.
        .set(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Snipdeck",
        )
        .query("client", "gtx")
        .query("sl", "auto")
        .query("tl", target)
        .query("dt", "t")
        .query("q", trimmed)
        .call()
        .context("translation request failed (network?)")?;

    // Response shape (dt=t):
    //   [ [ ["translated segment","source segment",...], ... ], null, "en", ... ]
    // Translated text = concat of [0][i][0]; detected source = [2].
    let body: serde_json::Value = resp.into_json().context("failed to parse translation response")?;
    let segs = body
        .get(0)
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("unexpected translation response"))?;

    let mut out = String::new();
    for seg in segs {
        if let Some(s) = seg.get(0).and_then(|v| v.as_str()) {
            out.push_str(s);
        }
    }
    if out.trim().is_empty() {
        return Err(anyhow!("translation returned empty"));
    }

    let source_lang = body
        .get(2)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Ok(Translation {
        text: out,
        source_lang,
    })
}

/// Ordered, selectable target languages (proper-case codes as Google's `tl`
/// param expects, English labels). Drives the popup's "More" dropdown and the
/// favourite-pill labels. Distinct from `LANGS` below, which is the (lower-case,
/// alias-rich) table used only to label an auto-*detected* source language.
pub const TARGET_LANGS: &[(&str, &str)] = &[
    ("tr", "Turkish"),
    ("en", "English"),
    ("de", "German"),
    ("fr", "French"),
    ("es", "Spanish"),
    ("it", "Italian"),
    ("ru", "Russian"),
    ("ar", "Arabic"),
    ("pt", "Portuguese"),
    ("nl", "Dutch"),
    ("sv", "Swedish"),
    ("pl", "Polish"),
    ("ro", "Romanian"),
    ("el", "Greek"),
    ("uk", "Ukrainian"),
    ("az", "Azerbaijani"),
    ("fa", "Persian"),
    ("he", "Hebrew"),
    ("hi", "Hindi"),
    ("ja", "Japanese"),
    ("ko", "Korean"),
    ("zh-CN", "Chinese (Simpl.)"),
    ("zh-TW", "Chinese (Trad.)"),
    ("id", "Indonesian"),
    ("th", "Thai"),
    ("vi", "Vietnamese"),
    ("bg", "Bulgarian"),
    ("cs", "Czech"),
    ("hu", "Hungarian"),
];

/// English label for a selectable *target* code (e.g. "zh-CN" → "Chinese
/// (Simpl.)"). Falls back to `lang_label` for anything not in `TARGET_LANGS`.
pub fn target_label(code: &str) -> String {
    TARGET_LANGS
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(code))
        .map(|(_, v)| v.to_string())
        .unwrap_or_else(|| lang_label(code))
}

/// Friendly English label for a language code (for the "detected language"
/// line). Falls back to the raw code in upper-case when unknown, so an
/// unexpected code still reads sensibly instead of vanishing.
pub fn lang_label(code: &str) -> String {
    let c = code.to_ascii_lowercase();
    let hit = LANGS.iter().find(|(k, _)| *k == c).map(|(_, v)| *v);
    match hit {
        Some(label) => label.to_string(),
        None if code.is_empty() => "—".to_string(),
        None => code.to_uppercase(),
    }
}

/// Code/label pairs used for the detected-language label lookup. Keys are
/// lower-case (the lookup lower-cases its input). The popup's quick-pick strip
/// shows a curated subset; the "More" dropdown lists the full set — both live
/// in `ui/translate_popup.slint` and must stay in sync with these codes.
pub const LANGS: &[(&str, &str)] = &[
    ("tr", "Turkish"),
    ("en", "English"),
    ("de", "German"),
    ("fr", "French"),
    ("es", "Spanish"),
    ("it", "Italian"),
    ("ru", "Russian"),
    ("ar", "Arabic"),
    ("pt", "Portuguese"),
    ("nl", "Dutch"),
    ("sv", "Swedish"),
    ("pl", "Polish"),
    ("ro", "Romanian"),
    ("el", "Greek"),
    ("uk", "Ukrainian"),
    ("az", "Azerbaijani"),
    ("fa", "Persian"),
    ("he", "Hebrew"),
    ("iw", "Hebrew"),
    ("hi", "Hindi"),
    ("ja", "Japanese"),
    ("ko", "Korean"),
    ("zh-cn", "Chinese (Simpl.)"),
    ("zh-tw", "Chinese (Trad.)"),
    ("zh", "Chinese"),
    ("id", "Indonesian"),
    ("th", "Thai"),
    ("vi", "Vietnamese"),
    ("bg", "Bulgarian"),
    ("cs", "Czech"),
    ("hu", "Hungarian"),
];
