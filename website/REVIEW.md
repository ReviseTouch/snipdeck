# Snipdeck EN docs — consistency & completeness review

Reviewed: all 18 `.mdx` files in `website/content/snipdeck/en/`
Date: 2026-06-02

## Overall verdict

The set is in **good shape and ships-ready** after a few small fixes. All 18 intended
pages exist, every frontmatter block is complete and valid, all categories are in the
allowed set, and every sibling/inline link points to a real slug in the correct
`/en/docs/snipdeck/<slug>` form (0 broken links across ~150 link instances).

The issues below are **factual contradictions about the number of upload hosts** (the
one real correctness problem), plus minor terminology/title drift and a couple of
coverage gaps. None are structural.

**Issue count: 8** (1 high, 3 medium, 4 low) + 2 coverage gaps.

---

## Structural checks (all pass)

**(1) Pages, frontmatter, order, category.** All 18 intended pages present. Every file
has `title`, `description`, `order`, `category`. Orders and categories match the intended
table exactly:

- getting-started: introduction(1), installation(2), quick-start(3), capture-shortcuts(4), faq(5)
- features: floating-snips(1), annotation-editor(2), collage(3), ocr(4), ocr-translate(5), gallery-and-search(6), sharing-and-export(7)
- reference: keyboard-shortcuts(1), settings(2), languages(3), troubleshooting(4), architecture(5)
- legal: privacy-and-security(1)

**(2) Categories.** Only `getting-started`, `features`, `reference`, `legal` are used. No stray categories.

**(4) Links.** All internal links use `/en/docs/snipdeck/<slug>` and resolve to a real page.
Note the intended slug for the collage page is `collage` (not `collage-editor`), and every
link correctly uses `/en/docs/snipdeck/collage`. No `.mdx` extensions, no relative links,
no anchors pointing at missing pages (verified the in-page `#...` anchors in installation,
quick-start, and faq resolve to headings that exist).

---

## Issues grouped by file

### settings.mdx  +  privacy-and-security.mdx  vs  sharing-and-export.mdx — HIGH (factual contradiction)
The docs disagree on **how many upload hosts exist**:
- `sharing-and-export.mdx` (the canonical Upload page) says **"Two services are supported"**
  and lists only **catbox.moe** and **Imgur** (lines 105-111). It never mentions 0x0.
- `settings.mdx` says **"Snipdeck supports three hosts"** and documents **0x0** as a third
  option (`upload_service: "0x0"`), both in the keys table (line 52) and a dedicated row
  (line 76).
- `privacy-and-security.mdx` line 68 also lists **"catbox.moe by default, plus Imgur and 0x0"**.

So 0x0 appears as a supported host in two pages and is absent/contradicted ("Two services")
in the page that is supposed to be authoritative on uploads. Pick one: either 0x0 is
supported (then add it to sharing-and-export's table and change "Two services") or it is not
(then remove it from settings.mdx and privacy-and-security.mdx). This is the only hard
factual contradiction in the set and should be resolved before publishing.

> Context worth confirming against the code: project memory notes Imgur is "coded but
> shelved (addclient blocked)" and catbox is the shipped default. If Imgur and/or 0x0 are
> not actually wired up in the shipping build, the docs may be over-promising hosts on all
> three pages — verify against `src/upload.rs` and the `upload_service` parser.

### sharing-and-export.mdx — MEDIUM
- `default_format` is described as accepting `"png"`, `"jpg"`, or `"bmp"` in settings.mdx
  (line 56), but the **Save** format table here (lines 48-54) advertises PNG, JPEG, BMP,
  GIF, and TIFF as selectable output. These aren't strictly contradictory (the dialog filter
  vs. the default-extension setting), but a reader can reasonably conclude `default_format`
  should accept `gif`/`tif` too. Worth a one-line clarification that `default_format` only
  governs the *pre-filled* extension while the dialog accepts the wider set.

### keyboard-shortcuts.mdx — MEDIUM (terminology / completeness)
- This is the only page that documents the **five global hotkeys** (`Ctrl+Shift+A/C/X/H/G`)
  and the `hotkey_*` settings keys. `settings.mdx` line 64 mentions "the named hotkeys"
  only in passing under "Other keys" and does **not** list them in the main keys table, and
  `capture-shortcuts.mdx` never mentions global hotkeys at all. The hotkeys are real and
  documented, but their discoverability is uneven — capture-shortcuts (the conceptual home
  for "how do I trigger a capture") should at least cross-link to the global-hotkey section.

### settings.mdx — MEDIUM
- The "Other keys" paragraph (line 64) lumps `include_border_on_save`,
  `include_border_on_clipboard`, `save_dir`, `floating_in_taskbar`, and the `hotkey_*` keys
  into "not part of the supported, hand-edited surface." But **other pages tell readers to
  edit exactly these keys**: sharing-and-export references `include_border_on_clipboard`,
  `include_border_on_save`, and `save_dir` as things you set; floating-snips documents
  `floating_in_taskbar` with a JSON example; keyboard-shortcuts tells readers to edit the
  `hotkey_*` keys. So settings.mdx calls "not supported for hand-editing" several keys that
  sibling pages actively instruct the reader to hand-edit. Either promote these into the
  main keys table or soften the "not part of the supported surface" wording.

### introduction.mdx — LOW
- The headline table row "Light / dark themes" has the description "A system-tray home and
  registry-free launch-at-login" (line 40) — the description belongs to a different feature
  (tray/autostart), not to themes. Looks like a copy/paste slip in the table.

### Title vs. slug drift (cosmetic) — LOW
- Several `title` values don't match the short slug name, which is fine for display but can
  confuse nav: `collage.mdx` title is "Collage editor" (slug `collage`); `ocr.mdx` title
  "OCR (text recognition)"; `keyboard-shortcuts.mdx` title "Keyboard & mouse shortcuts";
  `gallery-and-search.mdx` title "Gallery & search"; `languages.mdx` "Languages &
  localization"; `architecture.mdx` "Architecture (for contributors)". Inline link text is
  inconsistent too — the collage page is referred to as "Collage", "Collage editor", and
  "the collage editor" across pages; same for "Gallery and search" vs "Gallery & search"
  (ampersand vs "and"). Harmonize the link labels if you want a tidy nav, but no link is broken.

### ocr-translate.mdx — LOW (terminology)
- Uses British spelling "favourites/colour" while most pages use American "color/behavior"
  (e.g. settings.mdx "behavior", introduction "color"). annotation-editor.mdx and
  floating-snips.mdx also use "colour"/"behaviour". Spelling is split roughly down the
  middle across the set. Pick one variant for consistency (the repo is US-English elsewhere,
  so American is the safer default).

---

## Terminology consistency (check 3) — mostly good

The core nouns are used consistently:
- **snip** — consistent everywhere (never "screenshot" as the product term except generically).
- **floating snip** — consistent; "on-screen snip" used as a synonym only in toolbar labels.
- **the gallery** — consistent; titled "Gallery & search" but referred to as "the gallery" in prose.
- **the annotation editor / the collage editor** — consistent in prose.
- **OCR** / **OCR + Clipboard** / **OCR + Translate** — consistent capitalization and spacing.

Minor drift only: the ampersand-vs-"and" titles and the British/American spelling split
noted above. No conceptual term is used to mean two different things.

---

## Factual cross-checks that PASS (no contradiction)

- Capture modifier matrix (`Win`/`Ctrl`/`Alt`/`Shift` + drag, freeze-first
  `Win+Shift+Space`) is identical across introduction, quick-start, capture-shortcuts, and
  keyboard-shortcuts.
- Data-folder paths (`%APPDATA%\Snipdeck\settings.json`, `%LOCALAPPDATA%\Snipdeck\` with
  `snips.db`/`state.json`/`cache\`, `Pictures\Snipdeck\`) agree across installation,
  quick-start, faq, settings, and privacy-and-security.
- "23 UI languages" is consistent (introduction, installation, faq, languages, architecture).
  The 23-row endonym table in languages.mdx matches the count.
- OCR engine (`Windows.Media.Ocr`, local, no download, `en-US` fallback) is consistent across
  ocr, ocr-translate, faq, privacy-and-security, troubleshooting, introduction.
- Translation endpoint (Google `translate_a`, only OCR text sent, never the image) is
  consistent across ocr-translate, faq, privacy-and-security, troubleshooting.
- Registry-free `shell:Startup` autostart story is consistent across installation,
  privacy-and-security, troubleshooting, architecture.
- OCR + Translate target-language list (29 entries) in ocr-translate is internally
  consistent with the `translate_favs` default `["en","es","fr","de","it","pt","ru","ar"]`
  repeated identically in ocr-translate and settings.

---

## Coverage gaps (check 6)

1. **Tray menu "Show / open" and bulk-clear behavior is under-documented as a workflow.**
   "Clear unpinned" / "Clear all" appear in floating-snips and the gallery toolbar list, but
   there is no single place that explains the pin → survive-clear lifecycle end to end. Minor.

2. **Reopening hidden floating snips / session restore.** floating-snips says Snipdeck
   "remembers which snips were floating and can reopen them together" and gallery mentions an
   `on screen` state, but no page documents *how* you trigger "reopen all floating snips"
   (which toolbar control, what happens on restart). The `state.json` file is listed in
   settings/architecture but its session-restore behavior for floating windows is never
   described from the user's side. Worth a short section in floating-snips.

Neither gap is critical; the documented feature surface is otherwise thorough (capture,
floating, annotation, collage, OCR, translate, gallery/search/filter/sort, sharing/export,
settings, i18n, architecture, privacy, troubleshooting are all covered in depth).

---

## Recommended fix order
1. Resolve the **0x0 / "Two services" vs "three hosts"** contradiction (HIGH) — and verify
   Imgur/0x0 are actually shipping, given the "Imgur shelved" note.
2. Reconcile settings.mdx "Other keys" with the keys other pages tell readers to edit (MEDIUM).
3. Cross-link global hotkeys from capture-shortcuts (MEDIUM); clarify `default_format` vs Save filter (MEDIUM).
4. Fix the "Light / dark themes" table description in introduction (LOW); harmonize
   spelling and link labels (LOW).
