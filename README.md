<div align="center">

**English** · [Türkçe](README.tr.md) · [Español](README.es.md) · [简体中文](README.zh-CN.md) · [العربية](README.ar.md) · [Русский](README.ru.md) · [Deutsch](README.de.md) · [日本語](README.ja.md) · [Français](README.fr.md) · [Português](README.pt.md)

</div>

<div align="center">

<img src="assets/preview_256.png" alt="Snipdeck" width="120" />

# Snipdeck

**A fast, keyboard-driven screen-snipping _deck_ for Windows and Linux.**

<sub>A free & open-source app by <a href="https://revisetouch.com"><b>ReviseTouch</b></a>.</sub>

Capture a region, keep snips floating on top of everything, mark them up,
OCR-index them, and search hundreds of screenshots from one dense gallery —
all from a native Rust app with a [Slint](https://slint.dev) UI.

![platform](https://img.shields.io/badge/Windows-10%20%2F%2011-0078D6?logo=windows&logoColor=white) ![release](https://img.shields.io/github/v/release/ReviseTouch/snipdeck?label=release)
![linux](https://img.shields.io/badge/Linux-GNOME%20%C2%B7%20KDE%20%C2%B7%20X11%20%28beta%29-FCC624?logo=linux&logoColor=black) ![soon](https://img.shields.io/badge/macOS-on%20the%20way-lightgrey)
![built with](https://img.shields.io/badge/built%20with-Rust%20%2B%20Slint-orange?logo=rust&logoColor=white)
![languages](https://img.shields.io/badge/languages-23-success)
![license](https://img.shields.io/badge/license-MIT-green)

<br/>
<picture>
  <source media="(prefers-color-scheme: light)" srcset="docs/screenshots/hero-en-light.png">
  <img src="docs/screenshots/hero-en-dark.png" alt="The Snipdeck gallery, with two snips floating on the desktop" width="100%">
</picture>

</div>

---

## 📸 Screenshots

<!--
  Drop your screenshots into docs/screenshots/ with these exact names and they
  will show up here automatically:
    gallery.png · floating.png · annotate.png · collage.png
-->

<div align="center">

| The gallery workspace | Floating snips on the desktop |
| :---: | :---: |
| <img src="docs/screenshots/gallery.png" alt="Gallery" width="420" /> | <img src="docs/screenshots/floating.png" alt="Floating snips" width="420" /> |
| **The annotation editor** | **The collage editor** |
| <img src="docs/screenshots/annotate.png" alt="Annotation editor" width="420" /> | <img src="docs/screenshots/collage.png" alt="Collage editor" width="420" /> |

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
- 🧩 **Collage.** Lay several snips out as one image — grid, justified rows or masonry, with gap,
  rounded corners and background — saved back to the gallery as a new snip.
- 🔍 **Built-in OCR.** Every snip is OCR-indexed in the background with the OS engine
  (`Windows.Media.Ocr`) — no model download, no network, fully local.
- 🌐 **OCR + translate.** Read a snip's text in another language inline (29 targets, with
  favourites you can star).
- 🗂️ **Powerful gallery.** Full-text search over OCR text, window metadata, monitor and path;
  filter by status / content / size / date; a virtualized grid that stays smooth at thousands of snips
  ([tour](#️-the-gallery)).
- 📤 **Share & upload.** Windows share sheet, a new email (Simple MAPI), the system image editor,
  or an image host — catbox.moe, Imgur, or **your own server** from a ShareX `.sxcu` file
  ([details](#-your-own-image-host)). Every upload **asks first**.
- 🌍 **23 languages**, auto-detected from your OS and switchable from the toolbar's `⋯` menu.
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
| `Ctrl`+`Alt`+`Shift`+`A` | New snip |
| `Ctrl`+`Alt`+`Shift`+`C` | New snip **and** copy to clipboard |
| `Ctrl`+`Alt`+`Shift`+`X` | Clipboard only |
| `Ctrl`+`Alt`+`Shift`+`G` | Open the gallery |

Change them with the `hotkey_*` keys in `settings.json`. On Wayland the
desktop owns global shortcuts: these are only proposed on first launch, and you
change them in your desktop's keyboard settings afterwards.

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
  or **Upload** to an image host ([which one](#-your-own-image-host))
- **Transform** — rotate (90° / 180°), flip horizontal / vertical
- **Crop** (drag the handles inward) and **Resize**
- **Hide from screen** / **Delete**

---

## 🗂️ The gallery

One toolbar row; everything else is a click away.

- **New** takes a snip. Its arrow holds **Snip + Clip** and **Clip only**, each with its hotkey.
- **Collage** and **Pick color** sit beside it, and the search box takes the rest of the row —
  `Ctrl`+`F` to focus it, `Esc` to clear it.
- **Filter** opens status, content, size and date in one panel. Active filters show as chips
  under the toolbar, each removable on its own or all at once with **Clear filters**.
- **`⋯`** holds what changes rarely: floating snips in the taskbar, dark theme, launch at
  login, language, clearing the gallery, and About.
- The status bar counts what is shown and holds **Sort** and the **S / M / L** card size.

Hover a card for its quick actions; right-click it for [everything else](#️-right-click-actions).

---

## 📤 Your own image host

**`⋯` → Upload to** picks where **Upload** sends a snip: catbox.moe (the default, no account),
Imgur, or a host of your own. To add one, choose **Add from a .sxcu file…** and pick a ShareX
custom-uploader file — the format Zipline, XBackBone, Chevereto and most self-hosted image
servers already offer, so there is nothing to write by hand.

Snipdeck sends the image as multipart form data or as a binary body, with the file's headers,
arguments and query parameters, and reads the link with `{json:…}`, `{regex:…}`, `{header:…}`
or `{response}` — ShareX 12's `$json:…$` form too. A file that needs more (a prompt, a random
value, an XML answer, a destination that is not an image host) is refused when you import it,
with the reason, rather than failing on the first upload.

Before anything is sent, Snipdeck's own window names the host and its address and says what
the upload means; when it is done, it shows the link and has already copied it.

---

## 📦 Install

### Download

**Windows** — grab `snipdeck-windows-x64.exe` from the
[**Releases**](https://github.com/ReviseTouch/snipdeck/releases) page. A single
portable executable, no installer.

**Linux (x64)** — from the same page, the package for your distribution:

```bash
sudo apt install ./snipdeck_*_amd64.deb      # Debian, Ubuntu, Mint
sudo dnf install ./snipdeck-*.x86_64.rpm     # Fedora, RHEL, Rocky
sudo zypper install ./snipdeck-*.x86_64.rpm  # openSUSE
```

or, on any distribution and without root, the tarball:

```bash
tar xzf snipdeck-linux-x64.tar.gz
cd snipdeck-linux-x64
./install.sh          # into ~/.local; --uninstall reverses it
```

The packages run on any distribution with glibc 2.30 or later — Debian 11,
Ubuntu 20.04, RHEL 9 and everything newer — and were installed and started on
Debian 12 and 13, Ubuntu 22.04 and 24.04, Fedora, Rocky 9, openSUSE Leap 15.6
and Tumbleweed (`packaging/linux/test-install.sh`). On GNOME and KDE Plasma
they also install Snipdeck's desktop helper (below), which Snipdeck turns on at
its first start; GNOME loads a newly installed extension at the next login.

### Build from source

Requirements: [Rust](https://rustup.rs/) (stable).

```bash
git clone https://github.com/ReviseTouch/snipdeck.git
cd snipdeck
cargo build --release
```

**Windows** (10 or 11): the binary lands at `target\release\snipdeck.exe`.

**Linux**: the build needs the GTK, X11/Wayland, tray, D-Bus and GL headers.
On Debian/Ubuntu:

```bash
sudo apt install libgtk-3-dev libxcb1-dev libxcb-render0-dev libxcb-shape0-dev \
  libxcb-xfixes0-dev libxcb-randr0-dev libxrandr-dev libxkbcommon-dev \
  libwayland-dev libayatana-appindicator3-dev libfontconfig-dev libdbus-1-dev \
  libgl1-mesa-dev libegl1-mesa-dev
```

PipeWire needs nothing at build time: it is loaded at run time for Wayland
capture. To install your build the way a release installs, stage it beside the
installer:

```bash
packaging/linux/install-from-source.sh   # builds, stages afresh, installs
```

---

## 🌍 Languages

Snipdeck is fully localized. On first launch it picks your language from the OS locale;
override it any time from **`⋯` → Language** in the toolbar.

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
5. Run `tools/i18n-check.py`: it checks that every string is translated, placeholders intact; CI runs it too.

</details>

---

## 🔒 Privacy & security

- **Capture and OCR are local.** OCR uses `Windows.Media.Ocr` on Windows and
  Tesseract on Linux (fed through a pipe, no temporary file); nothing is sent
  anywhere for capture or indexing. On Wayland the screen is read through the
  ScreenCast portal: you grant it once, and the sharing indicator shows only
  for the moment a snip is taken.
- **OCR + Translate asks first.** The first time, it explains that the OCR'd
  *text* — never the image — goes to Google Translate, and waits for your yes.
  The text is sent in the request body, not the URL.
- **Uploads ask first, every time.** Nothing leaves your machine until you
  press **Upload** in a window that names the host and its address and says
  what that means: anyone with the link can open the image, a catbox upload
  cannot be deleted, a plain `http://` server sends it unencrypted. Don't
  upload sensitive content to a public-by-link host. API keys from an imported
  `.sxcu` file are kept in `settings.json`, as ShareX keeps them.
- **Hand-over copies stay private.** Images passed to a mail client or editor go
  to a folder only you can read (`$XDG_RUNTIME_DIR` on Linux,
  `%TEMP%\Snipdeck` on Windows) and are swept afterwards.
- **Logs keep no traces.** Pointer coordinates and window titles are logged
  only at debug level.
- **Launch-at-login is file-based.** A normal shortcut in `shell:Startup` on
  Windows (not the `HKCU\…\Run` key security tools most often flag), an XDG
  autostart entry on Linux.

---

## 🗺️ Roadmap

- [x] Windows 10 / 11
- [x] **Linux / GNOME** (Wayland) — with the bundled shell extension: capture
      with `Super`+`Alt`+drag or the hotkeys, snips placed where captured and kept
      on top, OCR, the colour picker
- [x] **Linux / KDE Plasma** (Wayland) — with the bundled KWin script: snips
      placed where captured and kept on top; capture from the hotkeys and the
      toolbar, OCR, the colour picker
- [x] **Linux / X11** (Xfce, MATE, Cinnamon, GNOME on Xorg…) — everything,
      snips kept on top by the window manager
- [~] **Linux / wlroots** (sway, Hyprland, river) — capture from the hotkeys
      and the toolbar, OCR, the colour picker; snips are neither placed nor
      kept on top (see below)

- [ ] **macOS** — on the way
- [ ] In-app settings window
- [ ] More annotation tools (blur / pixelate, numbered step badges, move/edit shapes)

There is no modifier+drag gesture outside GNOME: it needs a global pointer
hook, which Linux grants only by reading `/dev/input` — a system-wide keylogger
Snipdeck will not ask you to enable.

### Running on Linux

Global shortcuts on Wayland go through the desktop portal, which refuses a
session from a client with no application id. Launched from the applications
menu, Snipdeck has one (`com.revisetouch.Snipdeck`, the name of its desktop
entry); started from a terminal it registers the same id itself, which needs
xdg-desktop-portal 1.19 or later — on an older portal, use the menu.

Runtime dependencies: `libayatana-appindicator` (tray), `xdg-utils` (sharing),
and `tesseract` plus the language data you want (OCR — optional; without it OCR
is skipped rather than failing).

**What Wayland does not allow.** A Wayland client cannot position its own
windows, ask where they are, or request that they stay above others; the
compositor owns all three. So on a bare Wayland session a snip opens wherever the
compositor puts it rather than over the region it was captured from, it cannot
float above other applications, and it appears in the window list. Everything
else — region capture, the freeze overlay, global shortcuts, OCR, dragging, the
context menu — works.

**On KDE Plasma, a KWin script does the same** (`packaging/kwin-script/`):
it keeps snips above other windows, places them where they were captured, puts
them on every desktop and keeps them out of the taskbar. It loads without a
new login.

On the wlroots compositors (sway, Hyprland, river) nothing does this yet: snips
open where the compositor puts them and do not stay on top. A window rule in
the compositor's own configuration can float them — they are titled
`SnipdeckFloating …`.

**On GNOME, a shell extension closes the gap.** What a client may not do, code
running inside GNOME Shell may. `install.sh` therefore also installs
`snipdeck@revisetouch.com` (from `packaging/gnome-extension/`) when it detects a
GNOME session — a single file that watches for Snipdeck's snip windows and
places them where they were captured, keeps them above other windows, sticks
them to every workspace, and takes them out of the window switcher. Placement
travels in the window title, which is never visible on these borderless windows.

The extension also brings back the capture gesture. On GNOME `Super`+drag
already moves windows, so the gesture is **`Super`+`Alt`+drag**, active while
Snipdeck is running:

| Gesture (GNOME) | Result |
|---|---|
| `Super`+`Alt` + drag | Floating snip on screen |
| `Super`+`Alt`+`Ctrl` + drag | Floating snip **and** copy to clipboard |
| `Super`+`Alt`+`Shift` + drag | Gallery only (no floating window) |
| `Super`+`Alt`+`Ctrl`+`Shift` + drag | Gallery **and** clipboard, no floating window |

"Clipboard only" — `Win`+`Alt` on Windows — has no gesture here; use its
hotkey. `Esc` cancels a drag, and keeping `Super`+`Alt` held lets you take
several snips in a row.

Log out and back in after installing; GNOME Shell cannot reload extensions in
place on Wayland, because it *is* the compositor holding your session. The
extension is entirely optional — without it Snipdeck behaves like any other
Wayland client — and beyond Snipdeck's own windows it touches only
`Super`+`Alt`+drag, and only while Snipdeck is running.

Still missing on GNOME Wayland: a snip you drag is moved by Mutter, and the app
is never told where it landed, so its position is not remembered across
restarts.

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
| `upload_service` | `"catbox"` | Image host for *Upload*: `"catbox"` (no key), `"imgur"` or `"custom:<name>"`; set from `⋯` → Upload to |
| `upload_hosts` | `[]` | Hosts imported from ShareX `.sxcu` files, in ShareX's own field names |
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
| `src/sxcu.rs`, `src/upload_window.rs`, `ui/upload.slint` | ShareX custom uploaders; the upload consent / progress / result window |
| `src/tray.rs`, `src/autostart.rs`, `src/single_instance.rs` | Tray icon, launch-at-login, single-instance guard |
| `src/snip.rs`, `src/settings.rs`, `src/paths.rs` | Gallery persistence (SQLite + FTS), settings, paths |
| `src/image_files.rs`, `src/atomic_file.rs` | Snip images on disk: lossless WebP, atomic writes |
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
