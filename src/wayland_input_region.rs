//! Per-region input transparency for Wayland surfaces.
//!
//! The problem this solves: on Wayland a client cannot place, stack or even
//! locate its own windows, so a snipping tool cannot pin snips where they were
//! captured or keep them above other apps. The way out is to stop asking the
//! compositor for those things and host every snip inside ONE screen-sized
//! surface of our own — inside it, position and z-order are just drawing.
//!
//! That trades one problem for another: a surface covering the screen swallows
//! every click, leaving the desktop underneath unusable. `wl_surface.set_input_region`
//! is the answer, and the reason panels and desktop widgets work at all: the
//! surface declares exactly which rectangles accept input, and pointer events
//! outside them go to whatever is below as if the surface were not there.
//!
//! winit exposes only the all-or-nothing `set_cursor_hittest`, so this reaches
//! for the raw `wl_surface` behind the Slint window and calls the protocol
//! directly. It attaches to the compositor connection winit already owns
//! (`Backend::from_foreign_display`) rather than opening a second one.

use anyhow::{anyhow, Context};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
use wayland_client::{
    backend::{Backend, ObjectId},
    protocol::{wl_compositor, wl_region, wl_registry, wl_surface},
    Connection, Dispatch, Proxy, QueueHandle,
};

/// A rectangle that should accept pointer input, in surface-local LOGICAL
/// pixels (the same space Slint lays out in).
#[derive(Debug, Clone, Copy)]
pub struct InputRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// Restrict pointer input on `window` to `rects`; everything else clicks
/// through to whatever is underneath.
///
/// An empty slice makes the whole surface click-through — useful for a purely
/// decorative overlay. Returns an error on anything that is not a Wayland
/// window, so callers can fall back rather than assume.
pub fn set_input_rects(window: &slint::Window, rects: &[InputRect]) -> anyhow::Result<()> {
    // Go through the winit window rather than Slint's own handle: Slint's
    // adapter can produce a window handle but not always a display handle, and
    // raw-window-handle then reports "the underlying handle cannot be
    // represented". winit owns the connection and answers both.
    use i_slint_backend_winit::WinitWindowAccessor;
    let handles = window
        .with_winit_window(|w| {
            let display = w.display_handle().ok().map(|h| h.as_raw());
            let surface = w.window_handle().ok().map(|h| h.as_raw());
            (display, surface)
        })
        .ok_or_else(|| anyhow!("not a winit window"))?;

    let display_ptr = match handles.0.context("no display handle")? {
        RawDisplayHandle::Wayland(d) => d.display.as_ptr(),
        other => return Err(anyhow!("not a Wayland display: {other:?}")),
    };
    let surface_ptr = match handles.1.context("no window handle")? {
        RawWindowHandle::Wayland(w) => w.surface.as_ptr(),
        other => return Err(anyhow!("not a Wayland surface: {other:?}")),
    };

    // Attach to the connection winit already has open. A second connection
    // would get its own object namespace and could not touch this surface.
    let backend = unsafe { Backend::from_foreign_display(display_ptr.cast()) };
    let conn = Connection::from_backend(backend);

    let mut queue = conn.new_event_queue::<State>();
    let qh = queue.handle();
    let _registry = conn.display().get_registry(&qh, ());

    let mut state = State { compositor: None };
    queue
        .roundtrip(&mut state)
        .context("registry roundtrip")?;
    let compositor = state
        .compositor
        .ok_or_else(|| anyhow!("compositor did not advertise wl_compositor"))?;

    // Borrow the existing surface by id rather than creating one.
    let surface_id = unsafe {
        ObjectId::from_ptr(wl_surface::WlSurface::interface(), surface_ptr.cast())
            .map_err(|e| anyhow!("wl_surface from pointer: {e}"))?
    };
    let surface = wl_surface::WlSurface::from_id(&conn, surface_id)
        .map_err(|e| anyhow!("wl_surface from id: {e}"))?;

    let region = compositor.create_region(&qh, ());
    for r in rects {
        region.add(r.x, r.y, r.w, r.h);
    }
    surface.set_input_region(Some(&region));
    // The region only takes effect on the next commit, and Slint will not
    // necessarily paint again just because the input shape changed.
    surface.commit();
    region.destroy();

    queue.flush().context("flushing input region")?;
    Ok(())
}

struct State {
    compositor: Option<wl_compositor::WlCompositor>,
}

impl Dispatch<wl_registry::WlRegistry, ()> for State {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            if interface == wl_compositor::WlCompositor::interface().name {
                // Version 3 is enough for create_region; asking for more than
                // the compositor offers is a protocol error.
                let v = version.min(4);
                state.compositor = Some(registry.bind::<wl_compositor::WlCompositor, _, _>(
                    name,
                    v,
                    qh,
                    (),
                ));
            }
        }
    }
}

impl Dispatch<wl_compositor::WlCompositor, ()> for State {
    fn event(
        _: &mut Self,
        _: &wl_compositor::WlCompositor,
        _: wl_compositor::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_region::WlRegion, ()> for State {
    fn event(
        _: &mut Self,
        _: &wl_region::WlRegion,
        _: wl_region::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_surface::WlSurface, ()> for State {
    fn event(
        _: &mut Self,
        _: &wl_surface::WlSurface,
        _: wl_surface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
