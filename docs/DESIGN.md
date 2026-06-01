# Snipdeck Design AI Brief

This document captures the current Slint UI theme and the product behavior that must be preserved for a redesign. It is meant as a brief for a design tool; the output can be HTML/CSS and then ported to Slint.

## Design AI Prompt

Design a polished desktop productivity app UI for **Snipdeck**, a screen-snipping deck for capturing, pinning, searching, OCR-indexing, and managing many screenshot snippets.

The app is not a landing page. The first screen must be the actual working interface: toolbar, filters/search, gallery grid, and snip cards. Keep it dense, fast to scan, and professional. Avoid marketing hero layouts, decorative gradients, huge empty spacing, nested cards, and explanatory feature copy.

Output can be a responsive HTML/CSS mockup. Prefer a single self-contained HTML file with CSS variables, no external assets, and clear class names. The design must be practical to port to Slint.

## Product Personality

- Utility-first desktop tool.
- Quiet, focused, fast.
- Should feel like a pro capture/archive workspace, not a website.
- Main use cases: take snip, keep floating snips on screen, search old snips via OCR/window metadata, filter by state/type/date/size, copy/save/delete/pin.
- Users may keep hundreds or thousands of snips, so scanning density matters.

## Current App Structure

Main window:

- Title: `Snipdeck`
- Preferred size: `1100 x 700`
- Minimum size: `700 x 500`
- Current base background: `#1b1b1b`

Vertical sections:

1. Error banner, optional
   - Height: `32px`
   - Background: `#782020`
   - Text + close button.

2. Main toolbar
   - Height: `52px`
   - Background: `#262626`
   - Padding: `6px`
   - Gap: `6px`
   - Left actions, flexible spacer, settings button on right.

3. Filter/search bar
   - Desktop height: `72px`
   - Narrow layout height: `116px`
   - Background: `#202020`
   - Desktop breakpoint currently: `920px`
   - Contains compact select controls plus search field.

4. Body
   - Empty state if no snips.
   - Otherwise scrollable thumbnail grid.

## Current Toolbar Actions

Keep these commands available, but visual labels/icons can be redesigned:

- New Snip
- Snip + Clipboard
- Clipboard Only
- Toggle all floating windows visible/hidden
- Toggle whether floating snips appear in taskbar
- Clear unpinned
- Delete all
- Settings

Current labels:

- `New`
- `Snip + Clip`
- `Clip only`
- `Hide on-screen snips`
- `Show snips on screen`
- `Floating in taskbar: on`
- `Floating in taskbar: off`
- `Clear unpinned`
- `Clear all`
- `Settings`

## Current Filter/Search Controls

The filter area must remain in the same functional zone as search. It should allow users to filter every useful way without hiding the search field elsewhere.

Controls:

- Status: `All`, `Pinned`, `Unpinned`, `On screen`, `Hidden`
- Content: `All`, `Has OCR`, `No OCR`, `Has window`, `No window`, `Tagged`
- Size: `All`, `Small`, `Medium`, `Large`, `Wide`, `Tall`, `Square`
- Date: `All`, `Today`, `24 hours`, `7 days`, `30 days`
- Sort: `Newest`, `Oldest`, `Pinned`, `Largest`, `Smallest`, `Width`, `Height`, `Monitor`
- Search placeholder: `text / OCR / window / monitor / path`
- Reset button: `Reset`

Current filter control sizing:

- Filter component height: `58px`
- Label height: `14px`
- Label font size: `12px`
- Input/select height: `32px`
- Search clear button: `32 x 32`
- Desktop select widths: `104`, `126`, `112`, `120`, `136px`
- Search min width: `210px`

## Gallery Grid

Current grid:

- Card width: `240px`
- Card height: `220px`
- Gap: `12px`
- Outer padding: `12px`
- Cards auto-wrap by available width.

Each snip card shows:

- Thumbnail image
- Dimensions text
- Created date/time
- Monitor/window/source text if present
- Label/OCR/window-derived snippet if present
- State indicators:
  - pinned
  - floating/on screen
- Actions:
  - Copy
  - Save
  - Toggle pin
  - Delete

Current card colors:

- Normal card: `#232323`
- Hover card: `#2c2c2c`
- Thumbnail well: `#181818`
- Normal border: `#3a3a3a`
- Hover border: `#5a8ad0`
- Pinned border/chip: `#ffc832`
- Floating border/chip: `#4cd47a`
- Floating chip text: `#0e2a17`
- Text primary: `#ddd`
- Text secondary: `#888`
- Label text: `#bbb`
- Monitor/source text: `#8ab`

Current card styling:

- Border radius: `8px`
- Thumbnail well radius: `4px`
- Pinned chip: `20 x 20`, radius `10px`
- Floating chip: `64 x 18`, radius `9px`, label `on screen`
- Dims font: `12px`
- Date font: `11px`
- Monitor font: `10px`
- Label font: `12px`

## Empty State

Current copy:

- `No snips collected yet`
- `Capture a region from the screen with Win + left mouse.`
- `OCR, window info, and filters are indexed in the background.`

For a redesign, keep empty state compact. Do not turn it into a marketing hero.

## Context Menu

Floating snips have a compact right-click menu.

Current window:

- Title: `SnipdeckContextMenu`
- Width: `220px`
- Height: `222px`
- Background: `#1c1c1c`
- Border: `1px #444`
- Radius: `6px`
- Item height: `30px`
- Item hover: `#3a3a3a`
- Label font: `13px`
- Hint font: `11px`
- Stub/disabled color: `#888`
- Delete accent: `#d66`

Menu actions:

- Copy image
- OCR + Clipboard
- Transform, currently placeholder
- Resize again
- Hide from screen
- Delete

Current labels:

- `Copy`
- `OCR + Clipboard`
- `Transform`
- `Resize`
- `Hide from screen`
- `Delete`

## Floating Snip Window

This is a transparent, always-on-top, no-frame image window. It is not a normal app panel.

Important behavior:

- The captured image is the content.
- The window has a visible border ring so it is visible over any background.
- Resize handles appear only when appropriate.
- Right click opens context menu.
- Middle click has a special action.
- Dragging moves the snip.

Current visual system:

- Active border: `#ff8800`
- Inactive border: `#888888`
- Resize handle tint: `#ffd54a`
- Outer ring: `#ffffff`
- Inner ring: `#111111`
- Pinned dot: `#ffc800`
- Border thickness is controlled by runtime, normally around 3 physical pixels.
- Border is layered: white outer, brand/accent middle, dark inner.

Do not design floating snips as heavy cards. They should remain lightweight overlays.

## Snip Selection Overlay

This is the full-screen selector while dragging a capture rectangle.

Current visual:

- Title: `SnipdeckOverlay`
- Transparent background
- Outside selection dim: `#00000060`
- Selection border: triple outline
  - Accent `#ff8800`
  - White
  - Accent `#ff8800`
- Size label:
  - `80 x 22`
  - Background: `#000a`
  - Radius: `4px`
  - Font: `12px`
  - Text format: `W × H`

Design goal: high contrast, no flicker, readable on any desktop background.

## Current Color Tokens

Base:

- App background: `#1b1b1b`
- Toolbar: `#262626`
- Filter band: `#202020`
- Card: `#232323`
- Card hover: `#2c2c2c`
- Thumbnail well: `#181818`
- Context menu: `#1c1c1c`

Text:

- Primary: `#ddd`
- Secondary: `#aaa`
- Muted: `#888`
- Label: `#a8a29a`
- Card label: `#bbb`
- Source text: `#8ab`
- White: `#ffffff`

Accents:

- Main accent/orange: `#ff8800`
- Resize handle/yellow: `#ffd54a`
- Pin yellow: `#ffc832`
- Pin dot: `#ffc800`
- Floating green: `#4cd47a`
- Floating chip text: `#0e2a17`
- Blue hover border: `#5a8ad0`
- Error red: `#782020`
- Danger red: `#d66`

Borders:

- Normal border: `#3a3a3a`
- Divider: `#555`
- Context border: `#444`
- Floating outer ring: `#ffffff`
- Floating inner ring: `#111111`

## Layout Constraints For Slint Port

Please keep the design easy to port to Slint:

- Prefer fixed, stable control heights.
- Avoid CSS effects that Slint cannot easily reproduce: backdrop blur, complex masks, nested filters, animated gradients, glassmorphism, heavy shadows.
- Avoid responsive layouts that depend on CSS grid auto-placement too deeply; simple rows, wraps, and fixed card sizes are easier.
- Avoid cards inside cards.
- Keep text compact and ensure button labels fit at 700px minimum width.
- Prefer icon + short text for toolbar commands.
- Prefer symbol/icon-only compact actions inside snip cards.
- Keep border radius modest: 4-8px for panels/cards, pill only for state chips.
- Use a palette with a few functional accents, not a one-hue theme.
- Do not hide key actions behind mystery menus; Snipdeck is a repeated-use tool.

## Suggested Redesign Direction

The current UI works but is visually plain. A better direction:

- Dark neutral workspace with warmer accent.
- Dense top command bar, not a large header.
- Filter/search area should feel like a command palette strip.
- Snip cards should look like inspectable assets, with the image dominant.
- State indicators should be readable at a glance:
  - pinned
  - on screen/floating
  - OCR exists
  - window metadata exists
- Buttons should be small and crisp.
- Reduce emoji dependency if possible; use clean icons or simple glyphs.
- Search should feel central and powerful.
- Empty state should stay understated.

## Deliverable Requested From Design AI

Please output:

1. A single responsive HTML file with embedded CSS.
2. CSS variables for all theme colors, spacing, radii, and typography.
3. Desktop layout at 1100x700.
4. Narrow layout down to 700px width.
5. At least 8 sample snip cards showing varied states:
   - normal
   - pinned
   - floating
   - pinned + floating
   - OCR available
   - window metadata available
   - tall image
   - wide image
6. Mock context menu.
7. Mock floating snip overlay style.
8. Mock selection overlay style.

All in-app text is in English.

