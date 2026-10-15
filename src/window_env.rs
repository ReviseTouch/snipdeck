//! What the windowing environment actually lets us do.
//!
//! Snipdeck's floating snips lean on window management the way a Win32 app can
//! take for granted: put this window exactly here, keep it above everything,
//! hide it from the taskbar, tell me where it ended up. X11 grants most of
//! that; Wayland grants almost none of it, and *which* Wayland compositor is
//! running changes the answer again.
//!
//! Scattering `cfg!(target_os = ...)` and `WAYLAND_DISPLAY` probes across the
//! call sites made that impossible to reason about — the overlay, the context
//! menu and the floating windows each grew their own slightly different guess.
//! This module is the single place that decides, and it answers in terms of
//! *capabilities* rather than platform names, so call sites read as "can I
//! place this window?" instead of "am I on Wayland?".
//!
//! Detected once, at first use: a session does not change its display server
//! underneath a running process.

use std::sync::OnceLock;

/// Which display server the process is actually talking to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// A Windows build only ever detects `Windows`; the rest are compared against.
#[cfg_attr(target_os = "windows", allow(dead_code))]
pub enum Backend {
    Windows,
    /// X11, including XWayland — from the client's side they behave the same,
    /// which is precisely why running under XWayland restores window placement.
    X11,
    Wayland,
    /// Something we could not identify. Treated as the most permissive case so
    /// an unknown environment degrades to "try it and see" rather than
    /// pre-emptively disabling features.
    Unknown,
}

/// The compositor behind a Wayland session, where it changes what is possible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(target_os = "windows", allow(dead_code))]
pub enum Compositor {
    /// Mutter: where the Snipdeck shell extension can do what a client may
    /// not, and where Super+Alt+drag is the capture gesture.
    Gnome,
    Other,
    /// Not a Wayland session.
    NotApplicable,
}

/// Capabilities the current environment grants.
///
/// Every field is a thing Snipdeck genuinely needs somewhere, and every `false`
/// below corresponds to a behaviour that had to be redesigned rather than
/// fixed.
#[derive(Debug, Clone, Copy)]
pub struct Caps {
    pub backend: Backend,
    pub compositor: Compositor,

    /// Can a window be placed at chosen screen coordinates?
    ///
    /// Wayland: no. `Window::set_position` is silently ignored, which is why
    /// floating snips cannot be pinned where they were captured.
    pub absolute_position: bool,

    /// Can a window ask where it currently is?
    ///
    /// Wayland: no — `position()` answers 0,0. Without this a click inside a
    /// floating snip cannot be turned into a screen coordinate, which is what
    /// stopped the context menu opening at the cursor.
    pub query_own_position: bool,



    /// Can a drag be handed to the compositor to run?
    ///
    /// The one piece of window movement Wayland *does* allow
    /// (`xdg_toplevel.move`), and how floating snips stay draggable.
    pub interactive_move: bool,


    /// Is the *session* driven by a Wayland compositor, regardless of how this
    /// client happens to be connected?
    ///
    /// Deliberately different from `backend == Wayland`, and the difference
    /// matters: a client started with `WAYLAND_DISPLAY` unset inside a Wayland
    /// session talks X11 through XWayland and gets X11 window management — but
    /// the compositor is still Mutter or KWin, and global shortcuts still have
    /// to go through the portal, because GNOME does not route them to XWayland
    /// clients. Keying the shortcut backend off `backend` silently disabled
    /// every shortcut the moment the app was run under XWayland.
    ///
    /// So: window management follows how the client connects; global shortcuts
    /// follow what the compositor is.
    #[cfg_attr(target_os = "windows", allow(dead_code))]
    pub session_is_wayland: bool,

}

/// Detect once; a session cannot swap display servers under a running process.
pub fn caps() -> &'static Caps {
    static CACHE: OnceLock<Caps> = OnceLock::new();
    CACHE.get_or_init(|| {
        let caps = detect();
        log::info!(
            "window environment: {:?}/{:?} — position={} query_pos={} interactive_move={}",
            caps.backend,
            caps.compositor,
            caps.absolute_position,
            caps.query_own_position,
            caps.interactive_move,
        );
        caps
    })
}

#[cfg(target_os = "windows")]
fn detect() -> Caps {
    Caps {
        backend: Backend::Windows,
        compositor: Compositor::NotApplicable,
        absolute_position: true,
        query_own_position: true,
        // Windows drives moves by repositioning on pointer deltas; it has no
        // need to hand the drag over, and winit's `drag_window` is not the path
        // the rest of the Win32 floating-window behaviour is built around.
        interactive_move: false,
        session_is_wayland: false,
    }
}

#[cfg(not(target_os = "windows"))]
fn detect() -> Caps {
    let backend = detect_backend();
    // XDG_SESSION_TYPE describes the SESSION, which is exactly what is wanted
    // here and exactly what must not be used for `backend`.
    let session_is_wayland = std::env::var("XDG_SESSION_TYPE")
        .is_ok_and(|v| v.eq_ignore_ascii_case("wayland"))
        || backend == Backend::Wayland;
    let compositor = if backend == Backend::Wayland {
        detect_compositor()
    } else {
        Compositor::NotApplicable
    };

    match backend {
        Backend::Wayland => Caps {
            backend,
            compositor,
            // Neither exists in the core protocol for a plain toplevel,
            // whatever the compositor.
            absolute_position: false,
            query_own_position: false,
            interactive_move: true,
            session_is_wayland,
        },
        // X11 and XWayland: the classic EWMH world, where windows are placed
        // and located as asked.
        Backend::X11 | Backend::Unknown => Caps {
            backend,
            compositor,
            absolute_position: true,
            query_own_position: true,
            interactive_move: true,
            session_is_wayland,
        },
        Backend::Windows => unreachable!("handled by the windows cfg"),
    }
}

#[cfg(not(target_os = "windows"))]
fn detect_backend() -> Backend {
    // WAYLAND_DISPLAY is the deciding signal, not XDG_SESSION_TYPE: a client
    // launched with WAYLAND_DISPLAY unset inside a Wayland session talks X11
    // through XWayland and gets X11 semantics, which is exactly what we must
    // report. XDG_SESSION_TYPE would still say "wayland" and mislead.
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty());
    if wayland {
        return Backend::Wayland;
    }
    if std::env::var_os("DISPLAY").is_some_and(|v| !v.is_empty()) {
        return Backend::X11;
    }
    Backend::Unknown
}

#[cfg(not(target_os = "windows"))]
fn detect_compositor() -> Compositor {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP")
        .or_else(|_| std::env::var("XDG_SESSION_DESKTOP"))
        .unwrap_or_default();
    compositor_for(&desktop)
}

/// `XDG_CURRENT_DESKTOP` can be a list ("ubuntu:GNOME", "pop:GNOME").
#[cfg(not(target_os = "windows"))]
fn compositor_for(desktop: &str) -> Compositor {
    if desktop.to_ascii_lowercase().contains("gnome") {
        Compositor::Gnome
    } else {
        Compositor::Other
    }
}

#[cfg(all(test, not(target_os = "windows")))]
mod tests {
    use super::*;

    #[test]
    fn gnome_is_recognised_inside_a_desktop_list() {
        assert_eq!(compositor_for("ubuntu:GNOME"), Compositor::Gnome);
        assert_eq!(compositor_for("GNOME"), Compositor::Gnome);
        assert_eq!(compositor_for("KDE"), Compositor::Other);
        assert_eq!(compositor_for(""), Compositor::Other);
    }
}

/// Give a framed window a dark or light title bar.
///
/// Slint takes the frame's colours from the desktop's light/dark preference,
/// not from the window it frames: the annotation and collage editors, dark
/// whatever the desktop says, wore a white title bar on a light desktop. Call
/// after every `show()` — on Wayland a hidden window is destroyed and the next
/// show builds a new one. A window shown during this turn of the event loop
/// has no native window until the next turn, so it is tried again then.
pub fn set_frame_theme<C: slint::ComponentHandle + 'static>(c: &C, dark: bool) {
    use i_slint_backend_winit::winit::window::Theme;
    use i_slint_backend_winit::WinitWindowAccessor;
    let want = if dark { Theme::Dark } else { Theme::Light };
    let apply = move |w: &slint::Window| {
        w.with_winit_window(|native| {
            if native.theme() != Some(want) {
                native.set_theme(Some(want));
            }
        })
        .is_some()
    };
    if !apply(c.window()) {
        let weak = c.as_weak();
        slint::Timer::single_shot(std::time::Duration::ZERO, move || {
            if let Some(c) = weak.upgrade() {
                apply(c.window());
            }
        });
    }
}

/// Put a just-shown always-on-top window above the others on X11. Call after
/// every `show()` of a window whose `.slint` says `always-on-top`.
///
/// winit asks for `_NET_WM_STATE_ABOVE` once, when it creates the window — and
/// Slint creates windows hidden, so the request reaches the window manager
/// before the window is mapped, and EWMH has the window manager ignore a state
/// message for a window it does not manage yet. Mutter does: the snips came up
/// as ordinary windows and went behind whatever was clicked next. Slint only
/// repeats the request when `always-on-top` changes, which it never does. A
/// window hidden and shown again is withdrawn and loses the state as well.
///
/// Sent after the map request, so the window manager reads them in that order.
/// Again on the next turn, for a window whose native window is created there.
/// Nothing to do on Wayland, where the level travels with the surface, or on
/// Windows, where it is a window style.
pub fn keep_above<C: slint::ComponentHandle + 'static>(c: &C) {
    if !matches!(caps().backend, Backend::X11 | Backend::Unknown) {
        return;
    }
    use i_slint_backend_winit::winit::window::WindowLevel;
    use i_slint_backend_winit::WinitWindowAccessor;
    let apply = |w: &slint::Window| {
        w.with_winit_window(|native| native.set_window_level(WindowLevel::AlwaysOnTop));
    };
    apply(c.window());
    let weak = c.as_weak();
    slint::Timer::single_shot(std::time::Duration::from_millis(50), move || {
        if let Some(c) = weak.upgrade() {
            apply(c.window());
        }
    });
}
