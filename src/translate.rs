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

/// The longest piece sent in one request, in characters. The endpoint refuses
/// very large bodies; screenshots of a full page of text exceed this easily.
const CHUNK_CHARS: usize = 4000;

/// Translate `text` into `target` (an endpoint language code, e.g. "tr").
/// Source language is auto-detected (`sl=auto`).
///
/// The text travels in a POST body, not the URL. In the query string it was
/// logged by any TLS-inspecting proxy along the way, and a page of non-ASCII
/// text — 3–6 bytes per character once percent-encoded — overran the URL limit
/// and failed as "network?". Long text goes in pieces cut at line breaks.
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

    let mut out = String::new();
    let mut source_lang = String::new();
    for chunk in chunks(trimmed, CHUNK_CHARS) {
        let piece = translate_chunk(&agent, chunk, target)?;
        if source_lang.is_empty() {
            source_lang = piece.source_lang;
        }
        out.push_str(&piece.text);
    }
    if out.trim().is_empty() {
        return Err(anyhow!("translation returned empty"));
    }
    Ok(Translation {
        text: out,
        source_lang,
    })
}

fn translate_chunk(agent: &ureq::Agent, text: &str, target: &str) -> Result<Translation> {
    let resp = agent
        .post("https://translate.googleapis.com/translate_a/single")
        // A browser-like UA reduces the chance of an automated-traffic block.
        .set(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Snipdeck",
        )
        .query("client", "gtx")
        .query("sl", "auto")
        .query("tl", target)
        .query("dt", "t")
        .send_form(&[("q", text)])
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

/// Split `text` into pieces of at most `max` characters, cutting after a line
/// break where one falls inside the limit, so sentences and lines stay whole.
/// A single line longer than `max` is cut at a character boundary.
fn chunks(text: &str, max: usize) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = text;
    while rest.chars().count() > max {
        let limit = rest.char_indices().nth(max).map_or(rest.len(), |(i, _)| i);
        let cut = rest[..limit].rfind('\n').map_or(limit, |i| i + 1);
        out.push(&rest[..cut]);
        rest = &rest[cut..];
    }
    if !rest.is_empty() {
        out.push(rest);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::chunks;

    #[test]
    fn short_text_is_one_piece() {
        assert_eq!(chunks("merhaba dünya", 4000), vec!["merhaba dünya"]);
    }

    #[test]
    fn long_text_is_cut_at_line_breaks_and_loses_nothing() {
        let text = "satır bir\nikinci satır\nüç\n".repeat(400);
        let pieces = chunks(&text, 100);
        assert!(pieces.iter().all(|p| p.chars().count() <= 100));
        assert!(pieces.iter().all(|p| p.ends_with('\n')), "cut after a line break");
        assert_eq!(pieces.concat(), text);
    }

    #[test]
    fn one_overlong_line_is_cut_on_a_char_boundary() {
        let text = "ğ".repeat(250);
        let pieces = chunks(&text, 100);
        assert_eq!(pieces.len(), 3);
        assert_eq!(pieces.concat(), text);
    }
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
