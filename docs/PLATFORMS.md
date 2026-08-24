# Platform layering

How Snipdeck is meant to be split across Windows, Linux and (eventually) macOS,
and why the split is where it is.

## The shape of the problem

Snipdeck is not a portable app with a few `#[cfg]`s. Its central feature —
snips that sit on the desktop where they were captured, above everything, out
of the window list — is *entirely* a window-management feature, and window
management is exactly what the three platforms disagree about most.

So the axis that matters is not "which OS" but **"how much control over its own
windows does this process get?"**. Windows and X11 grant nearly all of it;
Wayland grants almost none; and among Wayland compositors the answer differs
again depending on whether `wlr-layer-shell` is offered.

That is why the code branches on capabilities rather than platform names.

## The three layers

### 1. `window_env` — what is possible here

The single source of truth. Detects the display server (`Backend`) and, on
Wayland, the compositor family (`Compositor`), then answers in capabilities:

| Capability | Windows | X11 / XWayland | Wayland |
|---|---|---|---|
| `absolute_position` | ✅ | ✅ | ❌ |
| `query_own_position` | ✅ | ✅ | ❌ |
| `always_on_top` | ✅ | ✅ | ❌ |
| `skip_taskbar` | ✅ | ❌ ¹ | ❌ |
| `interactive_move` | — ² | ✅ | ✅ |
| `input_region` | ❌ | ❌ | ✅ |

¹ The EWMH atoms exist; winit does not expose them.
² Windows drives moves by repositioning on pointer deltas and never needs to
hand the drag to the window manager.

Detection keys on `WAYLAND_DISPLAY`, deliberately **not** `XDG_SESSION_TYPE`: a
client started with the former unset inside a Wayland session talks X11 through
XWayland and has full window control, while `XDG_SESSION_TYPE` still says
"wayland" and would produce exactly the wrong answer.

### 2. `HostStrategy` — how snips reach the screen

Three architectures, chosen by capability, not by OS:

- **`WindowPerSnip`** — one OS window per snip. The original design; needs
  positioning, stacking and position queries. Windows, X11, XWayland.
- **`LayerShell`** — a `wlr-layer-shell` overlay surface: anchored placement, a
  stacking layer above normal windows, no taskbar entry. KWin and the wlroots
  family (sway, Hyprland, river, COSMIC). *Not implemented yet.*
- **`SingleSurface`** — every snip drawn inside one screen-sized surface, with
  `wl_surface.set_input_region` keeping the desktop behind it clickable. For
  compositors that refuse window control and offer no layer-shell: GNOME/Mutter.
  *Implemented, opt-in via `SNIPDECK_SNIP_HOST=surface`, not yet fit to default.*

`LayerShell` is preferred over `SingleSurface` wherever it exists, because it
grants the capability rather than working around its absence.

### 2a. The GNOME Shell extension — asking from the inside

GNOME is the one case where none of the three is satisfying. It has no
`wlr-layer-shell`, and `SingleSurface` cannot fix the part that matters most:
its host surface is itself an ordinary window, so on Mutter it drops behind the
focused app and takes every snip with it. Drawing snips correctly is worth
little if the whole surface cannot stay on top.

What *can* stay on top is code running inside the shell. So the default on GNOME
stays `WindowPerSnip` — one real window per snip — and
`packaging/gnome-extension/snipdeck@revisetouch.com` supplies the three things
the client is not allowed to do for itself:

| Withheld from the client | What the extension calls |
|---|---|
| placement | `meta_window.move_frame()` |
| stacking | `meta_window.make_above()` |
| absence from the window list | filters `WindowSwitcherPopup._getWindowList` and `Workspace._isOverviewWindow` ¹ |

¹ Mutter exposes only `is_skip_taskbar`, derived from the window type — the flag
cannot be *set* by anyone, application or extension. Filtering the lists the
shell builds is the remaining lever, and there are two of them: the Alt+Tab
window switcher and the Activities overview that Super opens. The overview is
the one people hit, and it is a separate code path — patching only the switcher
left every pinned snip sitting in the window picker next to real applications.
Only the *window* switcher is patched; the application switcher still shows
Snipdeck once, which is correct.

**The title is also what identifies a snip.** Not `wm_class`: that reports the
X11 WM_CLASS property, which a Wayland toplevel need not have, so requiring it
is a gate that may never open on Wayland — and while it was required, nothing
was adopted at all: no placement, no always-on-top, no switcher filtering. The
app id is still set, for the desktop-file association rather than for matching.

**Placement travels in the window title.** For floating snips the title is set
to `SnipdeckFloating <x>,<y>` in physical screen pixels (`floating.rs`, guarded
by `!caps().absolute_position` so Windows and X11 keep the bare marker). It is
never user-visible — these windows are borderless. Deliberately not D-Bus: a
title is readable the instant the window appears, needs no service running, no
bus name owned, and no start-up ordering between app and extension.

**The extension converts before it moves.** Physical pixels are what the app
captures in; Mutter places windows in logical coordinates, and on a scaled
display the two differ by the scale factor — a 3200x2000 panel at 1.667 is a
1920x1200 desktop. Passing the title's coordinates through unchanged put every
snip 1.667x too far right and down and pushed anything captured past the logical
width off the screen, so `_toLogical` divides by the scale of whichever monitor
the converted point lands on. Monitors with *different* scales are not solvable
from inside the extension: the physical origin the app uses for the second
monitor comes from its capture stack, which Mutter does not expose. That case
falls back to the primary monitor's scale.

Two properties this arrangement is designed to keep:

- **Optional.** Without the extension Snipdeck behaves like any other Wayland
  client. Nothing detects it, nothing waits for it, nothing fails without it.
- **Reversible.** `disable()` restores the switcher unconditionally. A monkey
  patch left behind after disable is how an extension breaks a desktop it is no
  longer running in.

Known limit: the extension reads the title once, when the window is first
titled. A snip dragged afterwards is moved by Mutter, and since
`query_own_position` is false the app never learns where it ended up — so
`last_position` is not persisted on GNOME Wayland and the snip reopens where it
was captured.

### 3. Backends — one file per platform mechanism

Where a feature genuinely has no shared implementation, the module carries the
per-platform bodies behind `#[cfg]`, with the non-native side either a real
alternative or an honest stub:

| Feature | Windows | Linux | macOS |
|---|---|---|---|
| Capture | DXGI + GDI (`dxgi_capture`) | xcap → portal / X11 | xcap |
| OCR (`ocr`) | `Windows.Media.Ocr` | `tesseract` if installed | stub |
| Tray (`tray`) | `Shell_NotifyIconW` | `tray-icon` + appindicator | — |
| Global shortcuts (`hotkeys`) | `global-hotkey` | portal, else `global-hotkey` | — |
| Single instance | named mutex | `flock` | `flock` |
| Sharing (`share`) | share sheet, MAPI | `xdg-open` | — |
| Window styling (`window_tamer`) | `WS_EX_TOOLWINDOW`, owner | no-op ³ | — |

³ Nothing to set: Wayland has no equivalent, and winit exposes none on X11.

## Distribution differences

Below the display server there is a second axis the Linux build has to survive:
what the distribution actually ships.

- **Shared-library sonames differ.** `libxdo` is `.so.3` on Ubuntu and `.so.4`
  on Arch. A binary linked against either refuses to start on the other, and no
  amount of documenting a runtime dependency fixes it. The answer is to not link
  optional libraries at all — `tray-icon` is taken with
  `default-features = false` for exactly this reason.
- **glibc is a floor, not a requirement.** A binary carries the glibc version it
  was built against as a minimum. Release builds therefore run on the oldest
  supported runner (`ubuntu-22.04`, glibc 2.35), not `ubuntu-latest`.
- **Optional runtime tools are detected, never assumed.** `tesseract` absent
  means OCR is skipped, not that the app fails.
- **The desktop environment is not the display server.** GNOME has no system
  tray without an extension; KDE does. The tray is built either way and simply
  has nowhere to appear on a bare GNOME.

## Adding a platform

1. Extend `window_env::detect` with its capabilities. If it can position and
   stack windows, `WindowPerSnip` works and there is nothing else to design.
2. Fill in the backend modules for capture, OCR, tray, shortcuts, sharing.
3. Only if window control is missing: pick or add a `HostStrategy`, and expect
   that to be the bulk of the work — it was on Wayland.
