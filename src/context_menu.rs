//! Floating snip right-click context menu manager.
//!
//! One borderless slint::Window, reused across right-clicks. Rust feeds
//! in the snip id + screen position; the menu collects the user's pick
//! and pushes it into a queue that app.rs drains.

use slint::{ComponentHandle, PhysicalPosition};
use std::cell::RefCell;
use std::rc::Rc;
use uuid::Uuid;

use crate::ContextMenuWindow;

#[derive(Debug, Clone, Copy)]
pub enum ContextAction {
    Close = 0,
    Delete = 1,
    Copy = 2,
    ResizeAgain = 3,
    CopyOcr = 4,
    RotateLeft = 5,
    RotateRight = 6,
    Rotate180 = 7,
    FlipHorizontal = 8,
    FlipVertical = 9,
    Crop = 10,
    OcrTranslate = 11,
    ShareSystem = 12,
    ShareMail = 13,
    OpenEditor = 14,
    Upload = 15,
    ShowApp = 16,
    Annotate = 17,
    PickColor = 18,
}

impl ContextAction {
    fn from_int(i: i32) -> Option<Self> {
        Some(match i {
            0 => Self::Close,
            1 => Self::Delete,
            2 => Self::Copy,
            3 => Self::ResizeAgain,
            4 => Self::CopyOcr,
            5 => Self::RotateLeft,
            6 => Self::RotateRight,
            7 => Self::Rotate180,
            8 => Self::FlipHorizontal,
            9 => Self::FlipVertical,
            10 => Self::Crop,
            11 => Self::OcrTranslate,
            12 => Self::ShareSystem,
            13 => Self::ShareMail,
            14 => Self::OpenEditor,
            15 => Self::Upload,
            16 => Self::ShowApp,
            17 => Self::Annotate,
            18 => Self::PickColor,
            _ => return None,
        })
    }
}

struct Shared {
    pending: Vec<(Uuid, ContextAction)>,
}

pub struct ContextMenuManager {
    window: Option<ContextMenuWindow>,
    /// A pre-built, hidden menu window kept ready so the next right-click shows
    /// instantly. Creating a Slint window (OS window + software-renderer
    /// surface) is the slow part of opening the menu; building it ahead of time
    /// (at startup and after each use, off the open path) makes opens feel
    /// immediate. Refilled by `prewarm`.
    spare: Option<ContextMenuWindow>,
    /// Current target snip id. Closures read this *at click time* so we
    /// can reuse the same window for different snips without rebuilding.
    target: Rc<RefCell<Option<Uuid>>>,
    shared: Rc<RefCell<Shared>>,
    /// Screen point the open menu should be anchored to, in physical pixels.
    ///
    /// Only used off Windows, where the menu window covers the whole screen and
    /// the panel is positioned inside it. The offset can't be computed at open
    /// time: the fullscreen size and scale factor aren't known until the
    /// compositor has answered, so `sync_to_window` resolves it a frame later.
    #[cfg(not(target_os = "windows"))]
    pending_anchor: Option<(i32, i32)>,
    /// True only when the open menu belongs to a floating snip, so the app loop
    /// dismisses it once that snip's window closes. False for a gallery-card
    /// menu, which has no floating window to track.
    auto_dismiss: bool,
    /// Outside-click dismissal: only `true` once the click that opened the menu
    /// has fully released. Until then we ignore button state so that same click
    /// can't be read as an "outside click" that immediately re-closes the menu.
    dismiss_armed: bool,
    /// Previous tick's "any mouse button down" sample, for rising-edge
    /// detection (we dismiss on the press, not while a button is held).
    prev_mouse_down: bool,
}

impl ContextMenuManager {
    pub fn new() -> Self {
        Self {
            window: None,
            spare: None,
            target: Rc::new(RefCell::new(None)),
            shared: Rc::new(RefCell::new(Shared {
                pending: Vec::new(),
            })),
            auto_dismiss: false,
            dismiss_armed: false,
            prev_mouse_down: false,
            #[cfg(not(target_os = "windows"))]
            pending_anchor: None,
        }
    }

    /// Fit the full-screen menu surface to what the compositor actually gave
    /// us, and place the panel at the anchor point inside it.
    ///
    /// Deferred for the same reason the snip overlay defers: a fullscreen
    /// request is answered asynchronously, so immediately after `show()` the
    /// window still reports its old size (often 0x0) and a stale scale factor.
    /// Called every tick while a menu is open; a no-op once settled.
    #[cfg(not(target_os = "windows"))]
    pub fn sync_to_window(&self) {
        let (Some(win), Some((ax, ay))) = (&self.window, self.pending_anchor) else {
            return;
        };
        let size = win.window().size();
        if size.width == 0 || size.height == 0 {
            return;
        }
        let sf = win.window().scale_factor().max(0.001);
        let (sw, sh) = (size.width as f32 / sf, size.height as f32 / sf);
        win.set_outer_w(sw);
        win.set_outer_h(sh);

        let (mw, mh) = (win.get_menu_w(), win.get_menu_h());

        // Centre it, deliberately — do NOT try to honour the anchor here.
        //
        // The anchor arrives as a click position inside the floating snip's own
        // window, and turning that into a screen coordinate needs to know where
        // that window sits. Wayland never tells a client its own position
        // (`Window::position()` answers 0,0), so the sum is meaningless and the
        // menu appeared at an arbitrary spot. A predictable centre beats a
        // wrong guess; the anchor is kept only for the X11/Windows paths and
        // for whenever a compositor learns to answer that question.
        let _ = (ax, ay);
        let x = ((sw - mw) / 2.0).max(0.0);
        let y = ((sh - mh) / 2.0).max(0.0);
        log::debug!("menu centred at ({x},{y}) in {sw}x{sh} logical surface (sf={sf})");
        win.set_menu_x(x);
        win.set_menu_y(y);

        // The surface covers the screen, so without this it would swallow every
        // click on the desktop behind it. Confine input to the panel and let the
        // rest through — the same mechanism panels and desktop widgets use, and
        // the one that makes a single screen-sized surface viable as a host for
        // floating snips.
        let rects = [crate::wayland_input_region::InputRect {
            x: x as i32,
            y: y as i32,
            w: mw as i32,
            h: mh as i32,
        }];
        match crate::wayland_input_region::set_input_rects(win.window(), &rects) {
            Ok(()) => log::debug!("menu input region confined to the panel"),
            Err(e) => log::debug!("input region unavailable ({e:#}); surface stays fully clickable"),
        }
    }

    /// Build a fully-wired (but hidden, unpositioned) menu window. The picked/
    /// dismissed closures read `self.target` at click time, so one pre-built
    /// window serves any snip — only the target id and position change per open.
    fn make_window(&self) -> Result<ContextMenuWindow, slint::PlatformError> {
        let w = ContextMenuWindow::new()?;
        let shared_pick = self.shared.clone();
        let target_for_pick = self.target.clone();
        let weak_for_pick = w.as_weak();
        w.on_picked(move |code| {
            if let (Some(action), Some(id)) =
                (ContextAction::from_int(code), *target_for_pick.borrow())
            {
                shared_pick.borrow_mut().pending.push((id, action));
            }
            if let Some(c) = weak_for_pick.upgrade() {
                let _ = c.hide();
            }
        });
        let weak_for_dismiss = w.as_weak();
        w.on_dismissed(move || {
            if let Some(c) = weak_for_dismiss.upgrade() {
                let _ = c.hide();
            }
        });
        Ok(w)
    }

    /// Build the spare menu window if one isn't ready. Cheap when already warm;
    /// called each tick so the cost lands off the open path (never while the
    /// user is waiting for the menu to appear).
    pub fn prewarm(&mut self) {
        if self.spare.is_none() {
            if let Ok(w) = self.make_window() {
                self.spare = Some(w);
            }
        }
    }

    /// Show the context menu at the given physical screen coordinates.
    /// `snip_id` identifies which snip the subsequent action applies to.
    /// Reuses the pre-built spare window for an instant open (falling back to a
    /// fresh build if the spare wasn't ready yet).
    pub fn show_for(
        &mut self,
        snip_id: Uuid,
        x: i32,
        y: i32,
        from_floating: bool,
    ) -> Result<(), slint::PlatformError> {
        // Drop any previous menu first so we don't leak a stranded
        // top-most window if the user right-clicks again before dismissing.
        if let Some(w) = self.window.take() {
            let _ = w.hide();
        }
        *self.target.borrow_mut() = Some(snip_id);
        self.auto_dismiss = from_floating;
        // Re-arm fresh for this open: wait for the opening click to release
        // (see `poll_auto_close`) before any outside-click can dismiss it.
        self.dismiss_armed = false;
        self.prev_mouse_down = false;

        let w = match self.spare.take() {
            Some(w) => w,
            None => self.make_window()?,
        };
        // "Show Snipdeck" is only meaningful from a floating snip — the gallery
        // menu is already opened from the app window.
        w.set_show_app_item(from_floating);

        #[cfg(target_os = "windows")]
        {
            // The window is the menu: place it at the cursor and be done.
            w.window().set_position(PhysicalPosition::new(x, y));
        }
        #[cfg(not(target_os = "windows"))]
        {
            // Wayland ignores `set_position`, so a menu-sized window lands
            // wherever the compositor likes — which is what made right-click
            // look broken. Cover the screen instead and draw the panel at the
            // cursor inside our own surface.
            //
            // Sized to the screen rather than flagged fullscreen: a fullscreen
            // surface can be handed straight to the display without blending,
            // and the transparent background then came out solid black over the
            // whole desktop. An ordinary window that happens to be exactly
            // screen-sized keeps its alpha, and the compositor has nowhere to
            // put it but the origin.
            w.window()
                .set_position(PhysicalPosition::new(x, y));
            if let Some((sw, sh)) = crate::screen::primary_logical_size() {
                w.window()
                    .set_size(slint::LogicalSize::new(sw as f32, sh as f32));
            }
            self.pending_anchor = Some((x, y));
        }

        w.show()?;
        self.window = Some(w);
        Ok(())
    }

    /// True while a menu is actually shown. `hide()` takes the window (the
    /// prewarmed spare lives in a separate field), so this reliably reflects
    /// an open menu — the pump stays awake while one is up.
    pub fn is_shown(&self) -> bool {
        self.window.is_some()
    }

    pub fn hide(&mut self) {
        if let Some(w) = self.window.take() {
            let _ = w.hide();
        }
        *self.target.borrow_mut() = None;
        self.auto_dismiss = false;
        self.dismiss_armed = false;
        self.prev_mouse_down = false;
        #[cfg(not(target_os = "windows"))]
        {
            self.pending_anchor = None;
        }
    }

    /// Dismiss the menu when the user presses a mouse button anywhere outside
    /// its bounds — the native "click elsewhere closes the menu" behaviour.
    /// Driven from the app tick (every few ms); a cheap no-op while no menu is
    /// open. Reads global async button state, so it works even though the
    /// borderless menu window never takes keyboard focus.
    ///
    /// Arming is deferred until the opening click has fully released, so the
    /// very right-click that spawned the menu is never misread as an outside
    /// click. A press *inside* the menu is left for Slint to route to the item
    /// (or the in-window dismiss area); only presses outside close it here.
    #[cfg(target_os = "windows")]
    pub fn poll_auto_close(&mut self) {
        if self.window.is_none() {
            self.dismiss_armed = false;
            self.prev_mouse_down = false;
            return;
        }

        let down = any_mouse_button_down();

        if !self.dismiss_armed {
            // Hold off until the opening click lifts; then start watching.
            if !down {
                self.dismiss_armed = true;
            }
            self.prev_mouse_down = down;
            return;
        }

        let just_pressed = down && !self.prev_mouse_down;
        self.prev_mouse_down = down;
        if !just_pressed {
            return;
        }

        // Scope the &self.window borrow so it ends before `hide()` needs &mut.
        let inside = {
            let w = self.window.as_ref().unwrap();
            cursor_pos()
                .map(|(cx, cy)| window_contains(w, cx, cy))
                .unwrap_or(true)
        };
        if !inside {
            self.hide();
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn poll_auto_close(&mut self) {}

    /// The target snip only when the menu is anchored to a floating snip, so the
    /// app loop auto-dismisses it when that snip's floating window closes. A
    /// gallery-card menu returns None here — there is nothing to track.
    pub fn floating_target(&self) -> Option<Uuid> {
        if self.auto_dismiss {
            *self.target.borrow()
        } else {
            None
        }
    }

    pub fn drain(&mut self) -> Vec<(Uuid, ContextAction)> {
        std::mem::take(&mut self.shared.borrow_mut().pending)
    }
}

/// True while any of the left/right/middle mouse buttons is physically down.
/// Uses the global async key state so it reads correctly regardless of which
/// window (if any) currently has focus.
#[cfg(target_os = "windows")]
fn any_mouse_button_down() -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, VK_LBUTTON, VK_MBUTTON, VK_RBUTTON,
    };
    let down = |vk: u16| unsafe { (GetAsyncKeyState(vk as i32) as u16 & 0x8000) != 0 };
    down(VK_LBUTTON.0) || down(VK_RBUTTON.0) || down(VK_MBUTTON.0)
}

#[cfg(target_os = "windows")]
fn cursor_pos() -> Option<(i32, i32)> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
    let mut p = POINT::default();
    unsafe { GetCursorPos(&mut p).ok()? };
    Some((p.x, p.y))
}

/// Whether physical screen point (`x`, `y`) lies within the menu window's
/// current bounds. Read live from the window so it stays correct under HiDPI
/// (the window reports its real physical position and size).
#[cfg(target_os = "windows")]
fn window_contains(w: &ContextMenuWindow, x: i32, y: i32) -> bool {
    let pos = w.window().position();
    let size = w.window().size();
    let x0 = pos.x;
    let y0 = pos.y;
    let x1 = x0 + size.width as i32;
    let y1 = y0 + size.height as i32;
    x >= x0 && x < x1 && y >= y0 && y < y1
}
