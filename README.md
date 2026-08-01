<div align="center">

**English** · [Türkçe](README.tr.md) · [Español](README.es.md) · [简体中文](README.zh-CN.md) · [العربية](README.ar.md) · [Русский](README.ru.md) · [Deutsch](README.de.md) · [日本語](README.ja.md) · [Français](README.fr.md) · [Português](README.pt.md)

</div>

<div align="center">

<img src="assets/preview_256.png" alt="Snipdeck" width="120" />

# Snipdeck

**A fast, keyboard-driven screen-snipping _deck_ for Windows.**

<sub>A free & open-source app by <a href="https://revisetouch.com"><b>ReviseTouch</b></a>.</sub>

Capture a region, keep snips floating on top of everything, mark them up,
OCR-index them, and search hundreds of screenshots from one dense gallery —
all from a native Rust app with a software-rendered [Slint](https://slint.dev) UI.

![platform](https://img.shields.io/badge/Windows-10%20%2F%2011-0078D6?logo=windows&logoColor=white)
![linux](https://img.shields.io/badge/Linux-X11%20full%20%2F%20Wayland%20partial-yellow) ![soon](https://img.shields.io/badge/macOS-on%20the%20way-lightgrey)
![built with](https://img.shields.io/badge/built%20with-Rust%20%2B%20Slint-orange?logo=rust&logoColor=white)
![languages](https://img.shields.io/badge/languages-23-success)
![license](https://img.shields.io/badge/license-MIT-green)

</div>

---

## 📸 Screenshots

<!--
  Drop your screenshots into docs/screenshots/ with these exact names and they
  will show up here automatically:
    gallery.png · floating.png · annotate.png · translate.png
-->

<div align="center">

| The gallery workspace | Floating snips on the desktop |
| :---: | :---: |
| <img src="docs/screenshots/gallery.png" alt="Gallery" width="420" /> | <img src="docs/screenshots/floating.png" alt="Floating snips" width="420" /> |
| **The annotation editor** | **OCR + translate** |
| <img src="docs/screenshots/annotate.png" alt="Annotation editor" width="420" /> | <img src="docs/screenshots/translate.png" alt="OCR translate" width="420" /> |

</div>

---

## ✨ Highlights

- 🖱️ **Capture on a modifier + drag.** Hold `Win` and drag a region; modifiers decide what
  happens to the result — floating window, clipboard, gallery-only ([matrix below](#️-capture-shortcuts)).
- 🧊 **Freeze-first capture** (`Win`+`Shift`+`Space`). Dims and freezes the screen *before* you
  select, so hover tooltips and other click-sensitive UI stay captured.
- 📌 **Floating snips.** Captures stay pinned on top as lightweight, borderless overlays you can
  drag, resize, pin, crop, rotate/flip, and dismiss.
- ✏️ **Built-in annotation editor.** Pen, highlighter, shapes, arrows, and text with a full HSV
  colour wheel, adjustable size + opacity, zoom, and undo/redo ([details](#️-annotation-editor)).
- 🔍 **Built-in OCR.** Every snip is OCR-indexed in the background with the OS engine
  (`Windows.Media.Ocr`) — no model download, no network, fully local.
- 🌐 **OCR + translate.** Read a snip's text in another language inline (30+ targets, with
  favourites you can star).
- 🗂️ **Powerful gallery.** Full-text search over OCR text, window metadata, monitor and path;
  filter by status / content / size / date; a virtualized grid that stays smooth at thousands of snips.
- 📤 **Share & upload.** Windows share sheet, a new email (Simple MAPI), the system image editor,
  or an image host — uploads are **opt-in with an explicit consent dialog**.
- 🌍 **23 languages**, auto-detected from your OS and switchable from the toolbar.
- 🌗 **Light / dark themes**, a system-tray home, and **registry-free** launch-at-login.

---

## ⌨️ Capture shortcuts

Hold `Win`, then drag a region with the **left** mouse button. Add modifiers to change what
happens to the snip:

| Shortcut | Result |
| --- | --- |
| `Win` + drag | Floating snip on screen |
| `Win`+`Ctrl` + drag | Floating snip **and** copy to clipboard |
| `Win`+`Alt` + drag | Clipboard only (not added to the gallery) |
| `Win`+`Shift` + drag | Gallery only (no floating window) |
| `Win`+`Ctrl`+`Shift` + drag | Gallery **and** clipboard, no floating window |
| `Win`+`Shift`+`Space`, then drag | **Freeze-first** — freeze the screen, then select on the still image |

You can also right-click the tray icon → **New Snip** to arm a one-shot drag capture without
holding `Win`.

### Global hotkeys

Besides `Win` + drag, Snipdeck registers five configurable global hotkeys:

| Hotkey | Action |
| --- | --- |
| `Win`+`Alt`+`H` | Show / hide all on-screen snips |
| `Ctrl`+`Shift`+`A` | New snip |
| `Ctrl`+`Shift`+`C` | New snip **and** copy to clipboard |
| `Ctrl`+`Shift`+`X` | Clipboard only |
| `Ctrl`+`Shift`+`G` | Open the gallery |

Change them with the `hotkey_*` keys in `settings.json`.

---

## ✏️ Annotation editor

Right-click any snip → **Annotate** to open a focused, Telegram-style editor that sits on top
of your snips.

- **Tools** — freehand pen, highlighter, rectangle, ellipse, line, arrow, and text.
- **Full HSV colour wheel** with a brightness slider — pick *any* colour, not a fixed palette.
- **Size & opacity sliders** with a blinking on-image size preview, remembered per tool.
- **Zoom in / out / fit** for precise work; the canvas pans when zoomed in.
- **Undo / redo / clear**, plus `Esc`, `Ctrl`+`Z`, `Ctrl`+`Y`.
- On **Save**, the strokes are flattened onto the image at full resolution (crisp, native-size).

The live preview and the saved result come from the same code, so **what you see is what you get**.

---

## 🖱️ Right-click actions

Right-click a floating snip **or** a gallery thumbnail:

- **Copy** the image to the clipboard
- **OCR + Clipboard** — run OCR and copy the recognized text
- **OCR + Translate** — open the translate popup
- **Annotate** — open the editor above
- **Share** — Windows share sheet, **Mail** (MAPI attachment), open in **editor** (Paint),
  or **Upload** to an image host
- **Transform** — rotate (90° / 180°), flip horizontal / vertical
- **Crop** (drag the handles inward) and **Resize**
- **Hide from screen** / **Delete**

---

## 📦 Install

### Download

Grab the latest `snipdeck.exe` from the [**Releases**](https://github.com/hasantr/snipdeck/releases)
page. It is a single portable executable — no installer.

### Build from source

Requirements: [Rust](https://rustup.rs/) (stable) and Windows 10 or 11.

```powershell
git clone https://github.com/hasantr/snipdeck.git
cd snipdeck
cargo build --release
```

The binary lands at `target\release\snipdeck.exe`.

---

## 🌍 Languages

Snipdeck is fully localized. On first launch it picks your language from the OS locale;
override it any time from the **globe menu** in the toolbar.

> English · Spanish · Simplified Chinese · Hindi · Arabic · Portuguese · Russian · Japanese ·
> German · French · Korean · Italian · Turkish · Indonesian · Vietnamese · Polish · Ukrainian ·
> Dutch · Persian · Thai · Bengali · Urdu · Romanian

The whole app — both the Slint UI (`@tr`) and the Rust-side messages (`i18n::t`) — reads from
**one gettext `.po` per language** under `lang/<code>/LC_MESSAGES/snipdeck.po`, bundled into the
binary at build time (no runtime gettext dependency).

<details>
<summary><b>Adding or updating a translation</b></summary>

1. New strings are wrapped in `@tr("…")` (in `.slint`) or `i18n::t("…")` (in Rust).
2. `lang/snipdeck.pot` is the template that lists every English string.
3. Copy a new msgid into each `lang/<code>/LC_MESSAGES/snipdeck.po` and fill its `msgstr` —
   keep placeholders (`{}`, `{e}`, `{url}`, `{path}`) and `\r\n` escapes intact.
4. To add a new language, create its `.po` and add the code to `LANGUAGES` and `po_source()`
   in `src/i18n.rs`.

</details>

---

## 🔒 Privacy & security

- **Capture and OCR are fully local.** OCR uses the built-in `Windows.Media.Ocr` engine; nothing
  is sent anywhere for capture or indexing.
- **Launch-at-login is registry-free.** Snipdeck drops a normal shortcut in your `shell:Startup`
  folder instead of writing an `HKCU\…\Run` value — the ordinary, user-visible mechanism that
  avoids the autostart registry key antivirus / EDR tools most often flag.
- **Uploads are opt-in.** Nothing leaves your machine without an explicit consent dialog that
  spells out the destination. Don't upload sensitive content to a public-by-link host.
- **OCR + Translate uses the network.** Only the OCR'd *text* is sent to Google's public
  `translate_a` endpoint to be translated; it is not used anywhere else.

---

## 🗺️ Roadmap

- [x] Windows 10 / 11
- [x] **Linux / X11** — full behaviour
- [~] **Linux / Wayland** — capture, global shortcuts, OCR and dragging work; pinned
      and always-on-top snips do not (see below)
- [ ] **macOS** — on the way
- [ ] In-app settings window
- [ ] More annotation tools (blur / pixelate, numbered step badges, move/edit shapes)

### Running on Linux

Install it and **launch it from the applications menu**, not from a terminal.
Global shortcuts go through the desktop portal, which refuses a session from a
client with no application id, and an unsandboxed app only gets one when the
desktop launches it from a `.desktop` entry — see `packaging/linux/snipdeck.desktop`.

Runtime dependencies: `libayatana-appindicator` (tray), `xdotool` (sharing),
and `tesseract` plus the language data you want (OCR — optional; without it OCR
is skipped rather than failing).

**What Wayland does not allow.** A Wayland client cannot position its own
windows, ask where they are, or request that they stay above others; the
compositor owns all three. So on Wayland a snip opens wherever the compositor
puts it rather than over the region it was captured from, it cannot float above
other applications, and it appears in the window list. Everything else — region
capture, the freeze overlay, global shortcuts, OCR, dragging, the context menu —
works.

KDE and the wlroots compositors (sway, Hyprland, river, COSMIC) expose
`wlr-layer-shell`, which grants exactly those three; GNOME/Mutter has declined
to implement it. `window_env::HostStrategy` names the three ways snips can be
hosted and picks between them.

---

## ⚙️ Configuration

Settings are stored as JSON and created on first run. There is no settings window yet — edit the
file directly, then restart Snipdeck.

| Path | Contents |
| --- | --- |
| `%APPDATA%\Snipdeck\settings.json` | User settings (below) |
| `%LOCALAPPDATA%\Snipdeck\` | `snips.db`, `state.json`, and the `cache\` of images + thumbnails |
| `Pictures\Snipdeck\` | Default folder for *Save* |

<details>
<summary><b>Selected <code>settings.json</code> keys</b></summary>

| Key | Default | Meaning |
| --- | --- | --- |
| `language` | `"auto"` | UI language code, or `"auto"` to follow the OS |
| `dark_theme` | `true` | Light or dark shell |
| `upload_service` | `"catbox"` | Image host for *Upload*: `"catbox"` (no key) or `"imgur"` |
| `imgur_client_id` | `""` | Imgur Client-ID, required only when `upload_service` is `"imgur"` |
| `translate_target` | `"en"` | Default OCR-translate target language code |
| `translate_favs` | `["en","es","fr","de","it","pt","ru","ar"]` | Quick-pick languages in the translate popup |
| `default_format` | `"png"` | Default extension for *Save* |

**Upload services** — *catbox.moe* (default) needs no account or key, but anonymous uploads
**cannot be deleted by you** afterwards. *Imgur* needs a free Client-ID (register once at
imgur.com → Settings → Applications). In both cases the image is unlisted but anyone with the
link can open it, and it is not encrypted.

</details>

---

## 🧩 Architecture

A single-threaded Slint event loop drives the UI; OS hooks and blocking work run on their own
threads and hand results back over channels, drained once per tick.

<details>
<summary><b>Module map</b></summary>

| Module | Responsibility |
| --- | --- |
| `src/app/` | App state, the per-tick loop, UI actions, gallery model |
| `src/capture.rs`, `src/dxgi_capture.rs` | Monitor capture (DXGI Desktop Duplication, GDI fallback) |
| `src/mouse_trigger.rs`, `src/native_mouse_trigger.rs` | Low-level mouse/keyboard hooks that arm and drive snips |
| `src/slint_overlay.rs`, `ui/overlay.slint` | Full-screen selection overlay (live or frozen) |
| `src/floating.rs`, `ui/floating.slint` | Floating snip windows (drag, resize, crop, border) |
| `src/annotate.rs`, `ui/annotate.slint` | Annotation editor (tiny-skia flatten, HSV wheel, zoom) |
| `src/ocr.rs` | `Windows.Media.Ocr` wrapper (on a worker thread) |
| `src/translate.rs`, `src/translate_popup.rs` | OCR-translate popup |
| `src/share.rs`, `src/upload.rs` | Share sheet / MAPI mail / editor / image-host upload |
| `src/tray.rs`, `src/autostart.rs`, `src/single_instance.rs` | Tray icon, launch-at-login, single-instance guard |
| `src/snip.rs`, `src/settings.rs`, `src/paths.rs` | Gallery persistence (SQLite + FTS), settings, paths |
| `src/i18n.rs`, `lang/` | Runtime i18n + bundled gettext `.po` catalogs (23 languages) |

</details>

---

## ✨ Made by ReviseTouch

Snipdeck is a free, open-source app from **[ReviseTouch](https://revisetouch.com)** —
we build developer tools and AI products. See what else we're building at
**[revisetouch.com](https://revisetouch.com)**.

---

## License

[MIT](LICENSE) © 2026 Hasan Salihoğlu

<div align="center">
<sub>Built with 🦀 Rust and <a href="https://slint.dev">Slint</a>.</sub>
</div>
