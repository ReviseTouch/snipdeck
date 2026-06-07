---
title: "Snipdeck — A fast, keyboard-driven screen-snipping deck for Windows"
description: "Landing page copy and section spec for Snipdeck, ReviseTouch's free, open-source Rust + Slint screen-snipping app for Windows."
order: 1
category: "Product"
---

# Snipdeck landing page — copy + section spec

This document is the **copy + section spec** for the `/snipdeck` product page on revisetouch.com. It mirrors the section rhythm and tone of the existing Evreka/Çolpan product page (hero badge → "What is it" → feature cards → tech/spec tables → audience/trust → final CTA). It is not a finished `.astro` file; each section below gives the section name, eyebrow/heading, body copy, any cards, button labels with their link targets, and a one-line layout note.

Every claim here is grounded in `README.md`. Nothing is invented. Where the README hedges (e.g. "on the way"), the copy hedges too.

**Global link targets used throughout:**

| Token | Destination |
| --- | --- |
| `DOWNLOAD` | `https://github.com/hasantr/snipdeck/releases` (latest `snipdeck.exe`) |
| `DOCS` | `/en/docs/snipdeck` |
| `GITHUB` | `https://github.com/hasantr/snipdeck` |
| `REVISETOUCH` | `https://revisetouch.com` |

---

## 1. Hero

- **Section name:** `hero`
- **Eyebrow / badge:** `FREE · OPEN SOURCE · WINDOWS 10 / 11`
- **Heading (line 1):** Capture once.
- **Heading (line 2, accent):** Keep it on top, mark it up, search it later.
- **Subhead / lede:**
  > Snipdeck is a fast, keyboard-driven screen-snipping deck for Windows. Hold `Win` and drag a region — your snip floats on top of everything, gets OCR-indexed in the background, and lands in a dense, searchable gallery. Native Rust, a software-rendered Slint UI, no account, no installer.
- **Trust strip (small mono chips under the lede):** `Rust + Slint` · `Local OCR` · `23 languages` · `MIT licensed` · `Single portable .exe`
- **Primary CTA button:** **Download for Windows** → `DOWNLOAD`
- **Secondary CTA button:** **Read the docs** → `DOCS`
- **Tertiary link (text):** **View source on GitHub** → `GITHUB`
- **Layout note:** Centered hero on the grid-bg with the mouse-follow dot matrix (same component as Evreka); two buttons side by side, a muted GitHub text link beneath, then a framed app mockup of the gallery.

---

## 2. What is Snipdeck

- **Section name:** `what`
- **Eyebrow:** What is it
- **Heading:** A snipping tool that doesn't forget your screenshots.
- **Body copy:**
  > Most snipping tools take a picture and walk away. Snipdeck treats every capture as something you'll come back to. Snips stay pinned on top as lightweight floating windows while you work, every one is OCR-indexed locally the moment it's taken, and the whole history lives in one virtualized gallery you can full-text search at thousands of snips.
  >
  > It's a single portable executable built in Rust with a software-rendered Slint UI — no installer, no runtime to manage, and capture plus OCR run entirely on your machine.
- **Highlight callout card:**
  > Hold `Win`, drag a region, let go. Modifiers decide the rest — float it, copy it, or send it straight to the gallery.
- **Layout note:** Two-column split (Evreka "what" pattern): copy + highlight card on the left, a small terminal/console-style mock on the right showing a gallery search matching OCR'd text.

---

## 3. Key features (cards)

- **Section name:** `features`
- **Heading:** Everything a screenshot deserves
- **Subhead:** One capture gesture, then a full toolkit for what happens next.
- **Cards:**

| Card title | One-line description |
| --- | --- |
| **Capture on `Win` + drag** | Hold `Win` and drag a region; modifiers send the snip to a floating window, the clipboard, the gallery, or all at once. |
| **Freeze-first capture** | `Win`+`Shift`+`Space` dims and freezes the screen *before* you select, so tooltips and hover-only UI stay captured. |
| **Floating snips** | Captures stay pinned on top as borderless overlays you can drag, resize, pin, crop, rotate, flip, and dismiss. |
| **Annotation editor** | Pen, highlighter, shapes, arrows, and text with a full HSV colour wheel, size and opacity sliders, zoom, and undo/redo. |
| **Built-in OCR** | Every snip is OCR-indexed in the background by the OS engine (`Windows.Media.Ocr`) — no model download, no network, fully local. |
| **OCR + translate** | Read a snip's text in another language inline, with 30+ targets and favourites you can star. |
| **Powerful gallery** | Full-text search over OCR text, window metadata, monitor, and path; filter by status, content, size, and date in a virtualized grid. |
| **Share & upload** | Send to the Windows share sheet, a new email (Simple MAPI), the system image editor, or an image host — uploads are opt-in. |

- **Layout note:** Three-column bordered card grid (Evreka `features` pattern), each card with an icon tile, bold title, and one-line description.

---

## 4. Capture-shortcut matrix

- **Section name:** `shortcuts`
- **Eyebrow:** Muscle memory
- **Heading:** One gesture, six results
- **Body copy:**
  > Hold `Win`, then drag a region with the **left** mouse button. Add modifiers to change what happens to the snip. You can also right-click the tray icon → **New Snip** to arm a one-shot drag capture without holding `Win`.
- **Table:**

| Shortcut | Result |
| --- | --- |
| `Win` + drag | Floating snip on screen |
| `Win`+`Ctrl` + drag | Floating snip **and** copy to clipboard |
| `Win`+`Alt` + drag | Clipboard only (not added to the gallery) |
| `Win`+`Shift` + drag | Gallery only (no floating window) |
| `Win`+`Ctrl`+`Shift` + drag | Gallery **and** clipboard, no floating window |
| `Win`+`Shift`+`Space`, then drag | **Freeze-first** — freeze the screen, then select on the still image |

- **CTA link (text):** **See all shortcuts and right-click actions** → `DOCS`
- **Layout note:** Full-width Markdown table on `bg-soft`, monospace key chips; a single muted docs link below.

---

## 5. Privacy & open source

- **Section name:** `privacy`
- **Eyebrow:** Yours, on your machine
- **Heading:** Local by default. Open by license.
- **Body copy:**
  > Capture and OCR happen entirely on your computer — nothing is sent anywhere to take or index a snip. The only features that touch the network are ones you trigger on purpose, and each one tells you exactly where your data goes.
- **Trust cards:**

| Card title | One-line description |
| --- | --- |
| **Capture & OCR are fully local** | OCR uses the built-in `Windows.Media.Ocr` engine; nothing leaves your machine to capture or index a snip. |
| **Registry-free launch-at-login** | Snipdeck drops a normal shortcut in your `shell:Startup` folder instead of writing an `HKCU\…\Run` value — the ordinary, user-visible autostart mechanism. |
| **Uploads are opt-in** | Nothing leaves your machine without an explicit consent dialog that names the destination first. |
| **Translate sends text only** | OCR + Translate sends only the recognized *text* to Google's public endpoint — never the image, and only when you ask for it. |
| **MIT licensed, source on GitHub** | The whole app is open source under the MIT license. Read it, build it, fork it. |

- **Primary CTA button:** **Read the source on GitHub** → `GITHUB`
- **Layout note:** Trust-card grid plus one prominent GitHub button; reuse the Evreka security/feature card styling.

---

## 6. Made by ReviseTouch

- **Section name:** `who`
- **Eyebrow:** Behind Snipdeck
- **Heading:** Built by ReviseTouch
- **Body copy:**
  > Snipdeck is a free, open-source app from **ReviseTouch** — we build developer tools and AI products. It's the same team and the same standards behind everything on our site: native performance, honest privacy defaults, and tools we use ourselves every day.
- **CTA button:** **See what else we build** → `REVISETOUCH`
- **Layout note:** Slim band (`bg-soft`, border-y), short copy centered, one ghost button to revisetouch.com.

---

## 7. Platforms & roadmap

- **Section name:** `roadmap`
- **Eyebrow:** Where it runs
- **Heading:** Windows today. macOS and Linux on the way.
- **Body copy:**
  > Snipdeck is Windows-only right now because capture, OCR, the tray, and sharing all use Windows-specific APIs. The UI (Slint) and most of the core are already cross-platform, so macOS and Linux builds are on the way.
- **Status list:**

| Status | Item |
| --- | --- |
| ✅ Available | Windows 10 / 11 |
| 🚧 On the way | macOS |
| 🚧 On the way | Linux |
| 🚧 Planned | In-app settings window |
| 🚧 Planned | More annotation tools (blur / pixelate, numbered step badges, move/edit shapes) |

> **Note:** Until the settings window ships, configuration lives in `%APPDATA%\Snipdeck\settings.json` — edit it directly, then restart Snipdeck. The docs cover every key.

- **CTA link (text):** **Configuration reference** → `DOCS`
- **Layout note:** Two columns — roadmap status table on one side, the settings note callout on the other; reuse Evreka tech-spec table framing.

---

## 8. Final download CTA

- **Section name:** `download`
- **Eyebrow / badge:** `FREE · OPEN SOURCE · NO ACCOUNT`
- **Heading:** Start snipping in under a minute.
- **Body copy:**
  > Grab the latest `snipdeck.exe` from the Releases page — a single portable executable, no installer. Hold `Win`, drag a region, and you're done.
- **Primary CTA button:** **Download for Windows** → `DOWNLOAD`
- **Secondary CTA button:** **Read the docs** → `DOCS`
- **Footer note (small text):**
  > Windows 10 / 11 · MIT licensed · Prefer to build it yourself? `git clone` and `cargo build --release`. [View on GitHub](https://github.com/hasantr/snipdeck) → `GITHUB`
- **Layout note:** Inverse-background CTA section with radial accent glow (Evreka `notify` pattern), centered heading and two buttons, muted build-from-source note underneath.

---

## See also

- [Snipdeck documentation](/en/docs/snipdeck)
- [Capture shortcuts reference](/en/docs/snipdeck)
- [ReviseTouch](https://revisetouch.com)
- [Snipdeck on GitHub](https://github.com/hasantr/snipdeck)
