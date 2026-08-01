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
pub enum Compositor {
    /// Mutter. Implements no layer-shell, so overlay-style windows are out.
    Gnome,
    /// KWin. Ships `wlr-layer-shell`, so positioned always-on-top surfaces work.
    Kde,
    /// sway, Hyprland, river, COSMIC — all layer-shell capable.
    WlrootsLike,
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

    /// Can a window ask to stay above others?
    ///
    /// Wayland: no. winit's `set_window_level` is an empty function there, and
    /// nothing in the core protocol replaces it.
    pub always_on_top: bool,

    /// Can a window be kept out of the taskbar and the Alt-Tab list?
    ///
    /// winit implements this on Windows only. `xdg_toplevel.set_parent` would
    /// get most of the way on Wayland — compositors treat parented toplevels as
    /// auxiliary — but winit's Wayland backend has no parent support and the
    /// `xdg_toplevel` object is not reachable from a raw window handle.
    pub skip_taskbar: bool,

    /// Can a drag be handed to the compositor to run?
    ///
    /// The one piece of window movement Wayland *does* allow
    /// (`xdg_toplevel.move`), and how floating snips stay draggable.
    pub interactive_move: bool,

    /// Can input be restricted to parts of a surface, letting clicks elsewhere
    /// fall through?
    ///
    /// `wl_surface.set_input_region`. This is what makes a single screen-sized
    /// surface usable as a host for every snip: without it such a surface would
    /// swallow every click on the desktop behind it.
    pub input_region: bool,

    /// Does a transparent window actually composite?
    ///
    /// Not a display-server property but a renderer one: Slint's software
    /// renderer draws through softbuffer, whose Wayland backend allocates
    /// `Xrgb8888` and drops the alpha channel. Recorded here because the call
    /// sites care about the outcome, not the cause.
    pub transparency: bool,
}

impl Caps {
    /// True when windows can be positioned, stacked and located — i.e. the
    /// original Win32 design works as written.
    pub fn manages_own_windows(&self) -> bool {
        self.absolute_position && self.always_on_top && self.query_own_position
    }

    /// How floating snips should be put on screen here.
    pub fn host_strategy(&self) -> HostStrategy {
        if self.manages_own_windows() {
            return HostStrategy::WindowPerSnip;
        }
        // A layer-shell compositor can give a real overlay surface: positioned,
        // stacked above normal windows, absent from the taskbar. That is a
        // strictly better answer than the input-region trick, so prefer it
        // wherever it exists.
        if matches!(self.compositor, Compositor::Kde | Compositor::WlrootsLike) {
            return HostStrategy::LayerShell;
        }
        if self.input_region {
            return HostStrategy::SingleSurface;
        }
        // Nothing left to try: snips become ordinary windows wherever the
        // compositor decides to put them. Degraded, but not broken.
        HostStrategy::WindowPerSnip
    }
}

/// How floating snips get onto the screen.
///
/// The three are genuinely different architectures, not switches on one — which
/// is why the choice lives here rather than being rediscovered at each call
/// site. Each is a separate piece of work against a separate set of protocols.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostStrategy {
    /// One OS window per snip, placed and stacked by us.
    ///
    /// The original design, and the only one that needs no tricks. Requires
    /// absolute positioning, always-on-top and position queries — so Windows
    /// and X11, including XWayland.
    WindowPerSnip,

    /// One screen-sized surface hosting every snip as a drawing.
    ///
    /// For compositors that refuse window control and offer no layer-shell —
    /// GNOME/Mutter, in practice. Inside our own surface, position and z-order
    /// are just layout, so the three things Wayland withholds stop mattering.
    /// The surface would swallow the desktop's clicks, which is what
    /// `wl_surface.set_input_region` exists to prevent: input is confined to
    /// the snip rectangles and everything else falls through.
    ///
    /// What it cannot recover: the host surface is still one ordinary window,
    /// so it cannot sit above fullscreen apps and it keeps one taskbar entry.
    SingleSurface,

    /// A `wlr-layer-shell` overlay surface.
    ///
    /// KWin and the wlroots family (sway, Hyprland, river, COSMIC) implement
    /// it. A layer surface is positioned by anchor and margin, lives on a
    /// stacking layer above normal windows, and never appears in the taskbar —
    /// everything the Win32 path gets, granted properly rather than worked
    /// around. Mutter has declined to implement it, which is the whole reason
    /// `SingleSurface` exists.
    LayerShell,
}

/// Detect once; a session cannot swap display servers under a running process.
pub fn caps() -> &'static Caps {
    static CACHE: OnceLock<Caps> = OnceLock::new();
    CACHE.get_or_init(|| {
        let caps = detect();
        log::info!(
            "window environment: {:?}/{:?} — position={} on_top={} skip_taskbar={} \
             query_pos={} interactive_move={} input_region={} transparency={} \
             -> snip host: {:?}",
            caps.backend,
            caps.compositor,
            caps.absolute_position,
            caps.always_on_top,
            caps.skip_taskbar,
            caps.query_own_position,
            caps.interactive_move,
            caps.input_region,
            caps.transparency,
            caps.host_strategy(),
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
        always_on_top: true,
        skip_taskbar: true,
        // Windows drives moves by repositioning on pointer deltas; it has no
        // need to hand the drag over, and winit's `drag_window` is not the path
        // the rest of the Win32 floating-window behaviour is built around.
        interactive_move: false,
        input_region: false,
        transparency: true,
    }
}

#[cfg(not(target_os = "windows"))]
fn detect() -> Caps {
    let backend = detect_backend();
    let compositor = if backend == Backend::Wayland {
        detect_compositor()
    } else {
        Compositor::NotApplicable
    };

    match backend {
        Backend::Wayland => Caps {
            backend,
            compositor,
            // None of these exist in the core protocol for a plain toplevel,
            // whatever the compositor. A layer-shell capable compositor could
            // offer them via a different surface role, which this build does
            // not use yet — see `wayland_input_region` for the route that
            // sidesteps them instead.
            absolute_position: false,
            query_own_position: false,
            always_on_top: false,
            skip_taskbar: false,
            interactive_move: true,
            input_region: true,
            // True only because the build selects femtovg in Cargo.toml. Going
            // back to `renderer-software` would make this false and take the
            // snip overlay and the context menu with it.
            transparency: true,
        },
        // X11 and XWayland: the classic EWMH world, where everything works.
        // `skip_taskbar` is the exception — the atoms exist but winit does not
        // expose them, so it would need raw X11 calls.
        Backend::X11 | Backend::Unknown => Caps {
            backend,
            compositor,
            absolute_position: true,
            query_own_position: true,
            always_on_top: true,
            skip_taskbar: false,
            interactive_move: true,
            input_region: false,
            transparency: true,
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
        .unwrap_or_default()
        .to_ascii_lowercase();

    if desktop.contains("gnome") {
        Compositor::Gnome
    } else if desktop.contains("kde") || desktop.contains("plasma") {
        Compositor::Kde
    } else if ["sway", "hyprland", "river", "wayfire", "cosmic"]
        .iter()
        .any(|c| desktop.contains(c))
    {
        Compositor::WlrootsLike
    } else {
        Compositor::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_like_environments_manage_their_own_windows() {
        let caps = Caps {
            backend: Backend::X11,
            compositor: Compositor::NotApplicable,
            absolute_position: true,
            query_own_position: true,
            always_on_top: true,
            skip_taskbar: false,
            interactive_move: true,
            input_region: false,
            transparency: true,
        };
        assert!(caps.manages_own_windows());
    }

    #[test]
    fn windows_and_x11_keep_one_window_per_snip() {
        let caps = Caps {
            backend: Backend::X11,
            compositor: Compositor::NotApplicable,
            absolute_position: true,
            query_own_position: true,
            always_on_top: true,
            skip_taskbar: false,
            interactive_move: true,
            input_region: false,
            transparency: true,
        };
        assert_eq!(caps.host_strategy(), HostStrategy::WindowPerSnip);
    }

    #[test]
    fn layer_shell_wins_over_the_input_region_workaround() {
        for compositor in [Compositor::Kde, Compositor::WlrootsLike] {
            let caps = Caps {
                backend: Backend::Wayland,
                compositor,
                absolute_position: false,
                query_own_position: false,
                always_on_top: false,
                skip_taskbar: false,
                interactive_move: true,
                input_region: true,
                transparency: true,
            };
            assert_eq!(
                caps.host_strategy(),
                HostStrategy::LayerShell,
                "{compositor:?} implements wlr-layer-shell"
            );
        }
    }

    #[test]
    fn gnome_falls_back_to_a_single_hosting_surface() {
        let caps = Caps {
            backend: Backend::Wayland,
            compositor: Compositor::Gnome,
            absolute_position: false,
            query_own_position: false,
            always_on_top: false,
            skip_taskbar: false,
            interactive_move: true,
            input_region: true,
            transparency: true,
        };
        assert_eq!(caps.host_strategy(), HostStrategy::SingleSurface);
    }

    #[test]
    fn wayland_does_not_manage_its_own_windows() {
        let caps = Caps {
            backend: Backend::Wayland,
            compositor: Compositor::Gnome,
            absolute_position: false,
            query_own_position: false,
            always_on_top: false,
            skip_taskbar: false,
            interactive_move: true,
            input_region: true,
            transparency: true,
        };
        assert!(!caps.manages_own_windows());
    }
}
