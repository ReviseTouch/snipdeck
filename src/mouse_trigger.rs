use crossbeam_channel::{unbounded, Receiver, Sender};
#[cfg(not(target_os = "linux"))]
use parking_lot::Mutex;
#[cfg(not(target_os = "linux"))]
use rdev::{grab, Button, Event, EventType, Key};
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(not(target_os = "linux"))]
use std::sync::Arc;

/// One-shot arm flag. The tray's "Yeni Snip Al" command sets this so the
/// next left-button drag begins a snip without the user holding Win. It is
/// consumed (cleared) on the next button-press regardless of outcome, so a
/// stale arm can't linger.
static SNIP_ARMED: AtomicBool = AtomicBool::new(false);

/// Win+Shift+Space "freeze first" intent, shared by BOTH hook backends and
/// clearable by the app (`clear_freeze`). A stuck flag would hijack the next
/// ordinary click into a snip, so the app clears it on capture failure / cancel
/// and expires it via a watchdog (see the tick loop).
pub(crate) static FREEZE_ARMED: AtomicBool = AtomicBool::new(false);

/// Armed when a Win+drag snip starts (so the user is holding Win). When they
/// release Win, that lone Win-up would open the Start menu. We SUPPRESS the
/// physical Win-up and inject a replacement "Ctrl tap + Win-up" — the proven
/// AutoHotkey technique: the Ctrl registers as a key pressed during the Win
/// hold, so the replacement Win-up is no longer "lone" and the menu stays shut.
/// Consumed by the very next Win key-up (the injected replacement is NOT armed,
/// so it passes through and actually releases Win) and cleared by a fresh Win
/// key-down (covers a release our hook never saw, e.g. on the secure desktop).
///
/// Used on BOTH Windows 10 and 11 and by BOTH hook backends. This replaced the
/// old press-time "inert key" masks (F23 on 11, early Ctrl tap on 10): inert
/// keys never cancelled the lone-Win on Win10 at all, press-time taps raced
/// the user's release (thread-spawn latency), and F23 collided with the
/// Copilot key chord (hardware Copilot = Win+Shift+F23 — exactly what our
/// injection produced during a Win+Shift+drag snip on Copilot-era machines).
#[cfg_attr(target_os = "linux", allow(dead_code))] // set by the hooks, which Linux has none of
pub(crate) static DISGUISE_WINUP_ARMED: AtomicBool = AtomicBool::new(false);

/// Colour-picker mode: when armed, the next left-click captures the pixel under
/// the cursor (sent as `ColorPick`) instead of starting a snip, and Esc cancels.
/// One-shot — cleared on the click or cancel.
static COLOR_PICK_ARMED: AtomicBool = AtomicBool::new(false);

/// Set when the pointer hook could not be installed at all.
///
/// rdev's `grab` needs read access to `/dev/input`, which an ordinary desktop
/// user does not have unless they are in the `input` group. When it fails, no
/// click and no Esc ever reaches this module — and callers that hide a window
/// and then wait for one would wait forever. The failure used to go to stderr
/// and nowhere else, so nothing could ask about it.
static POINTER_HOOK_FAILED: AtomicBool = AtomicBool::new(false);

/// Did the pointer hook fail to install?
///
/// False also means "not yet known": `grab` blocks for the process lifetime
/// when it succeeds, so this can only ever report a failure that has already
/// happened. In practice that is immediate — it fails on the first attempt to
/// open the device — and long before a user reaches a toolbar button.
#[cfg_attr(target_os = "linux", allow(dead_code))] // hook-driven; Linux picks through the portal
pub fn pointer_hook_failed() -> bool {
    POINTER_HOOK_FAILED.load(Ordering::Relaxed)
}

/// For the native Windows hook, which lives in its own module: record that it
/// could not be installed, as the rdev listener below does for itself.
#[cfg(target_os = "windows")]
pub(crate) fn note_pointer_hook_failed() {
    POINTER_HOOK_FAILED.store(true, Ordering::Relaxed);
}

/// Arm a one-shot drag-snip (called from the tray menu).
pub fn arm_snip() {
    SNIP_ARMED.store(true, Ordering::Relaxed);
}

/// Take-and-clear the one-shot arm flag. Shared with the native backend.
pub(crate) fn consume_armed() -> bool {
    SNIP_ARMED.swap(false, Ordering::Relaxed)
}

/// Enter colour-pick mode (called when the eyedropper tool starts).
#[cfg_attr(target_os = "linux", allow(dead_code))] // hook-driven; Linux picks through the portal
pub fn arm_color_pick() {
    COLOR_PICK_ARMED.store(true, Ordering::Relaxed);
}

/// Leave colour-pick mode (after a pick or a cancel).
pub fn disarm_color_pick() {
    COLOR_PICK_ARMED.store(false, Ordering::Relaxed);
}

/// Take-and-clear the colour-pick arm (the click that picks the pixel, or an
/// Esc that cancels the mode). Shared with the native backend.
#[cfg_attr(target_os = "linux", allow(dead_code))] // hook-driven; Linux picks through the portal
pub(crate) fn consume_color_pick() -> bool {
    COLOR_PICK_ARMED.swap(false, Ordering::Relaxed)
}

/// Clear the freeze-first intent from the app side so neither hook backend
/// stays armed after a failed, cancelled, or abandoned freeze.
pub fn clear_freeze() {
    FREEZE_ARMED.store(false, Ordering::Relaxed);
}

/// True while a snip selection drag is physically in progress. Mirrored by
/// whichever hook backend is active (rdev or native) so the pump — which
/// can't see the backend's private state — knows to keep ticking and stream
/// the live selection rectangle even if the cursor momentarily stops moving.
#[cfg_attr(target_os = "linux", allow(dead_code))] // set by the hooks, which Linux has none of
static SNIP_ACTIVE: AtomicBool = AtomicBool::new(false);

#[cfg_attr(target_os = "linux", allow(dead_code))] // set by the hooks, which Linux has none of
pub(crate) fn set_snip_active(active: bool) {
    SNIP_ACTIVE.store(active, Ordering::Relaxed);
}

/// Any snip/eyedropper interaction the [`crate::pump`] must stay awake for:
/// an in-progress drag, a one-shot tray arm, an armed freeze-first frame, or
/// the colour picker waiting for its click.
pub fn snip_in_progress() -> bool {
    SNIP_ACTIVE.load(Ordering::Relaxed)
        || SNIP_ARMED.load(Ordering::Relaxed)
        || FREEZE_ARMED.load(Ordering::Relaxed)
        || COLOR_PICK_ARMED.load(Ordering::Relaxed)
}

#[derive(Debug, Clone, Copy)]
pub struct ModifierSet {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum MouseTriggerEvent {
    StartSnip { x: i32, y: i32, mods: ModifierSet },
    /// Win+Shift+Space: freeze the screen at this cursor point NOW (before any
    /// click, so hover tooltips are still on screen). The next left-button
    /// drag selects on that frozen snapshot.
    FreezeArm { x: i32, y: i32 },
    /// Colour-pick mode: the user clicked at this point to grab its pixel colour.
    /// Hook-driven; on Linux the colour comes from the desktop portal instead.
    #[cfg_attr(target_os = "linux", allow(dead_code))]
    ColorPick { x: i32, y: i32 },
    Move { x: i32, y: i32 },
    Release { x: i32, y: i32 },
    Cancel,
}

pub struct MouseTriggerService {
    pub rx: Receiver<MouseTriggerEvent>,
    /// Producer side, kept so non-hook sources (the overlay-driven selection
    /// on Linux/Wayland) can feed the same pipeline.
    pub tx: Sender<MouseTriggerEvent>,
}

impl MouseTriggerService {
    /// Backend-agnostic constructor: the caller polls `self.rx` on its
    /// own schedule (Slint timer, native loop, etc).
    pub fn start_polling() -> Self {
        let (tx, rx) = unbounded::<MouseTriggerEvent>();
        // No global pointer hook on Linux, by design. rdev's grab reads
        // /dev/input directly, which works only for a user in the `input`
        // group — i.e. one who has handed every process they run a system-wide
        // keylogger — and where it did work it did harm: it swallowed the
        // overlay's own clicks, made up coordinates from raw relative motion,
        // and left Super stuck down after Super+Shift+Space. Capture runs
        // through the overlay and the GNOME extension instead, and the colour
        // picker through the desktop portal.
        #[cfg(target_os = "linux")]
        {
            log::info!("mouse trigger: no global pointer hook on Linux");
            POINTER_HOOK_FAILED.store(true, Ordering::Relaxed);
        }
        #[cfg(not(target_os = "linux"))]
        if native_backend_enabled() {
            log::info!("mouse trigger backend: native Windows hook");
            crate::native_mouse_trigger::spawn_listener(tx.clone());
        } else {
            log::info!("mouse trigger backend: rdev");
            spawn_listener(tx.clone());
        }
        Self { rx, tx }
    }
}

fn native_backend_enabled() -> bool {
    #[cfg(target_os = "windows")]
    {
        static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *ENABLED.get_or_init(|| {
            // The native Win32 hook is the DEFAULT on Windows: unlike rdev it
            // can tell injected input apart (LLKHF_INJECTED + extra-info tag),
            // which the release-time Start-menu disguise depends on, and it
            // lets us inject the replacement synchronously from the hook.
            // Set SNIPDECK_NATIVE_MOUSE_HOOK=0 to fall back to rdev.
            let disabled = ["SNIPDECK_NATIVE_MOUSE_HOOK", "SNIPPER_NATIVE_MOUSE_HOOK"]
                .iter()
                .any(|name| {
                    std::env::var(name)
                        .map(|value| matches!(value.as_str(), "0" | "false" | "FALSE" | "no" | "NO"))
                        .unwrap_or(false)
                });
            !disabled
        })
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

#[cfg(not(target_os = "linux"))]
#[derive(Default)]
struct HookState {
    win_down: AtomicBool,
    ctrl_down: AtomicBool,
    alt_down: AtomicBool,
    shift_down: AtomicBool,
    snip_active: AtomicBool,
}

#[cfg(not(target_os = "linux"))]
fn spawn_listener(tx: Sender<MouseTriggerEvent>) {
    std::thread::spawn(move || {
        let state = Arc::new(HookState::default());
        let last_pos = Arc::new(Mutex::new((0i32, 0i32)));

        let state_cb = state.clone();
        let pos_cb = last_pos.clone();
        let tx_cb = tx.clone();

        // Funnel every surfaced event through here: send it, then wake the
        // pump (it may have gone to sleep while idle). Routing all sends
        // through one place means none can silently miss its wake.
        let emit = move |ev: MouseTriggerEvent| {
            let _ = tx_cb.send(ev);
            crate::pump::wake();
        };

        // Robust input model:
        //   - We use `grab` so we CAN suppress the LMouse press/release
        //     that triggers a snip (otherwise it leaks to apps under
        //     the cursor and starts text selection / window moves).
        //   - We NEVER suppress any keyboard event. Modifiers always
        //     flow to the OS untouched, so keys cannot get latched in
        //     a "stuck down" state by our hook chain.
        //   - We do NOT synthesize modifier KEYUPs after a snip: doing
        //     so would drop a Win key the user is still physically
        //     holding and break the "hold Win, drag several times for
        //     multiple snips" flow. Instead the tracked modifier flags
        //     are reconciled against GetAsyncKeyState on every check
        //     (see `modifier_down_now`), so a KeyUp our hook happened to
        //     miss can never leave a modifier stuck "down".
        //   - To keep the Start Menu from popping after a Win+LMouse
        //     snip, the armed post-snip Win-up is suppressed and replaced
        //     with [Ctrl tap + Win-up] (see `DISGUISE_WINUP_ARMED`),
        //     without touching the real modifier state.
        let result = grab(move |event: Event| -> Option<Event> {
            let s = &*state_cb;
            match event.event_type {
                EventType::KeyPress(k) => {
                    match k {
                        Key::MetaLeft | Key::MetaRight => {
                            // A physical Win press *may* clear a stale disguise
                            // arm (a release this hook never saw, e.g. on the
                            // secure desktop) — but ONLY on a genuinely FRESH
                            // press. A held Win key streams typematic-repeat
                            // KEYDOWNs a hook can't tell from a fresh press, and
                            // clearing on a repeat would drop the disguise
                            // mid-hold → the post-snip Win-up pops Start. `swap`
                            // returns the PRIOR tracked state: if Win wasn't
                            // already down it's unambiguously fresh → clear; if it
                            // was, disambiguate a real repeat from a stale bit via
                            // GetAsyncKeyState's pre-event state — DOWN = repeat
                            // (keep the arm), UP = stale (treat as fresh → clear).
                            // Mirrors the native backend.
                            let win_vk = if matches!(k, Key::MetaLeft) { 0x5B } else { 0x5C };
                            let win_was_down = s.win_down.swap(true, Ordering::Relaxed);
                            if !win_was_down || !async_key_down(win_vk) {
                                DISGUISE_WINUP_ARMED.store(false, Ordering::Relaxed);
                            }
                        }
                        Key::ControlLeft | Key::ControlRight => {
                            s.ctrl_down.store(true, Ordering::Relaxed);
                        }
                        Key::Alt | Key::AltGr => {
                            s.alt_down.store(true, Ordering::Relaxed);
                        }
                        Key::ShiftLeft | Key::ShiftRight => {
                            s.shift_down.store(true, Ordering::Relaxed);
                        }
                        // Esc leaves colour-pick mode.
                        Key::Escape if COLOR_PICK_ARMED.load(Ordering::Relaxed) => {
                            COLOR_PICK_ARMED.store(false, Ordering::Relaxed);
                            emit(MouseTriggerEvent::Cancel);
                        }
                        Key::Escape
                            if s.snip_active.load(Ordering::Relaxed)
                                || FREEZE_ARMED.load(Ordering::Relaxed)
                                || SNIP_ARMED.load(Ordering::Relaxed) =>
                        {
                            s.snip_active.store(false, Ordering::Relaxed);
                            set_snip_active(false);
                            FREEZE_ARMED.store(false, Ordering::Relaxed);
                            // Also cancel an un-clicked one-shot arm (toolbar/
                            // tray/hotkey "New Snip"); otherwise Esc is dead, the
                            // hidden main window never returns, and the next
                            // ordinary click gets hijacked into a snip.
                            consume_armed();
                            emit(MouseTriggerEvent::Cancel);
                        }
                        // Win+Alt+Space → freeze the screen now (capture hover
                        // tooltips), then drag-select on the still image.
                        // Suppress Space so Alt+Space doesn't open the window
                        // system menu.
                        Key::Space
                            if win_down_now(s)
                                && shift_down_now(s)
                                && !s.snip_active.load(Ordering::Relaxed) =>
                        {
                            // Win+Shift+Space: freeze the screen now (capture
                            // hover tooltips), then drag-select on the still
                            // image. Swallow EVERY Win+Shift+Space (incl.
                            // key-repeat); fire FreezeArm only on the first.
                            if !FREEZE_ARMED.swap(true, Ordering::Relaxed) {
                                // The swallowed Space never reaches the OS, so
                                // the eventual Win-up could still read as a
                                // lone Win — arm the release-time disguise
                                // here too (Win is held by definition).
                                DISGUISE_WINUP_ARMED.store(true, Ordering::Relaxed);
                                let (x, y) = cursor_pos_or_last(&pos_cb);
                                emit(MouseTriggerEvent::FreezeArm { x, y });
                            }
                            return None;
                        }
                        _ => {}
                    }
                    Some(event)
                }
                EventType::KeyRelease(k) => {
                    match k {
                        Key::MetaLeft | Key::MetaRight => {
                            s.win_down.store(false, Ordering::Relaxed);
                            // Win10: replace this lone post-snip Win-up with a
                            // disguised one so the Start menu can't open. We
                            // suppress the physical up here; `inject_disguised_winup`
                            // then sends [Ctrl tap + Win-up] — that Win-up is no
                            // longer armed, so it passes through and releases Win.
                            if DISGUISE_WINUP_ARMED.swap(false, Ordering::Relaxed) {
                                let left = matches!(k, Key::MetaLeft);
                                std::thread::spawn(move || inject_disguised_winup(left));
                                return None;
                            }
                        }
                        Key::ControlLeft | Key::ControlRight => {
                            s.ctrl_down.store(false, Ordering::Relaxed);
                        }
                        Key::Alt | Key::AltGr => {
                            s.alt_down.store(false, Ordering::Relaxed);
                        }
                        Key::ShiftLeft | Key::ShiftRight => {
                            s.shift_down.store(false, Ordering::Relaxed);
                        }
                        _ => {}
                    }
                    // Win+Shift+Space is "freeze first, Win+Shift+S style":
                    // once armed, releasing Win/Shift does NOT cancel — the user
                    // lets go and then drags on the frozen image. Only Esc (or
                    // the click that starts the snip) clears the freeze.
                    Some(event)
                }
                EventType::ButtonPress(Button::Left) => {
                    // Colour-pick mode: grab the pixel under the cursor and
                    // suppress the click (don't let it reach the app below).
                    if COLOR_PICK_ARMED.swap(false, Ordering::Relaxed) {
                        let (x, y) = cursor_pos_or_last(&pos_cb);
                        emit(MouseTriggerEvent::ColorPick { x, y });
                        return None;
                    }
                    let armed = consume_armed();
                    let win = win_down_now(s);
                    // freeze_armed lets the click start the selection even with
                    // NO modifier held — the screen is already frozen
                    // (Win+Shift+Space "freeze first", Win+Shift+S style).
                    if (win || armed || FREEZE_ARMED.load(Ordering::Relaxed))
                        && !s.snip_active.load(Ordering::Relaxed)
                    {
                        s.snip_active.store(true, Ordering::Relaxed);
                        set_snip_active(true);
                        FREEZE_ARMED.store(false, Ordering::Relaxed);
                        // Win held: arm the Win-up replacement so the post-snip
                        // Win release can't pop the Start menu (see
                        // DISGUISE_WINUP_ARMED). Only when Win is the trigger —
                        // an armed/freeze click has no Win-up to disguise.
                        // Release-time replacement is the ONE technique proven
                        // on both Win10 and Win11; the old press-time filler
                        // taps (F23 / early Ctrl) are gone.
                        if win && cfg!(target_os = "windows") {
                            DISGUISE_WINUP_ARMED.store(true, Ordering::Relaxed);
                        }
                        let (x, y) = cursor_pos_or_last(&pos_cb);
                        let mods = modifier_snapshot(s);
                        emit(MouseTriggerEvent::StartSnip { x, y, mods });
                        // Suppress the click so apps below don't react.
                        return None;
                    }
                    Some(event)
                }
                EventType::ButtonRelease(Button::Left) => {
                    if s.snip_active.load(Ordering::Relaxed) {
                        s.snip_active.store(false, Ordering::Relaxed);
                        set_snip_active(false);
                        let (x, y) = cursor_pos_or_last(&pos_cb);
                        emit(MouseTriggerEvent::Release { x, y });
                        return None;
                    }
                    Some(event)
                }
                EventType::MouseMove { x, y } => {
                    let p = (x as i32, y as i32);
                    *pos_cb.lock() = p;
                    if s.snip_active.load(Ordering::Relaxed) {
                        emit(MouseTriggerEvent::Move { x: p.0, y: p.1 });
                    }
                    Some(event)
                }
                _ => Some(event),
            }
        });

        if let Err(e) = result {
            // Record it, don't just print it. Everything that waits on a click
            // from this hook — the colour picker above all, which hides the
            // main window first — needs to be able to find out that no click
            // is ever coming.
            POINTER_HOOK_FAILED.store(true, Ordering::Relaxed);
            log::warn!(
                "pointer hook unavailable ({e:?}); click-driven tools will not \
                 receive events. On Linux rdev needs read access to /dev/input."
            );
        }
    });
}

#[cfg(not(target_os = "linux"))]
fn modifier_snapshot(state: &HookState) -> ModifierSet {
    ModifierSet {
        ctrl: modifier_down_now(&state.ctrl_down, &[0x11, 0xA2, 0xA3]),
        alt: modifier_down_now(&state.alt_down, &[0x12, 0xA4, 0xA5]),
        shift: modifier_down_now(&state.shift_down, &[0x10, 0xA0, 0xA1]),
    }
}

#[cfg(not(target_os = "linux"))]
fn win_down_now(state: &HookState) -> bool {
    modifier_down_now(&state.win_down, &[0x5B, 0x5C])
}

#[cfg(not(target_os = "linux"))]
fn shift_down_now(state: &HookState) -> bool {
    modifier_down_now(&state.shift_down, &[0x10, 0xA0, 0xA1])
}

#[cfg(not(target_os = "linux"))]
fn modifier_down_now(flag: &AtomicBool, virtual_keys: &[u16]) -> bool {
    #[cfg(target_os = "windows")]
    {
        // On Windows the OS async key-state is authoritative. A low-level
        // keyboard hook has a hard per-callback timeout, so under load (the
        // burst of mouse-move events while dragging a snip) Windows can skip
        // calling our hook for an event — and we miss the physical KeyUp.
        // That used to leave this flag stuck `true`, so every later
        // left-click started a fresh snip until the user tapped the key
        // again. GetAsyncKeyState reports the real hardware state, so when it
        // says the key is up we clear the stale flag and report up; the flag
        // is only a fast-path that must never override reality.
        // (Mirrors the native backend's `reconcile_modifier_state`.)
        let physically_down = virtual_keys.iter().copied().any(async_key_down);
        if !physically_down {
            flag.store(false, Ordering::Relaxed);
        }
        physically_down
    }
    #[cfg(not(target_os = "windows"))]
    {
        // No reliable async key-state API here; trust the tracked flag.
        let _ = virtual_keys;
        flag.load(Ordering::Relaxed)
    }
}

#[cfg(not(target_os = "linux"))]
fn cursor_pos_or_last(last_pos: &Mutex<(i32, i32)>) -> (i32, i32) {
    let pos = current_cursor_pos().unwrap_or_else(|| *last_pos.lock());
    *last_pos.lock() = pos;
    pos
}

#[cfg(target_os = "windows")]
fn async_key_down(vk: u16) -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;

    unsafe { (GetAsyncKeyState(vk as i32) as u16 & 0x8000) != 0 }
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn async_key_down(_vk: u16) -> bool {
    false
}

#[cfg(target_os = "windows")]
fn current_cursor_pos() -> Option<(i32, i32)> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

    let mut point = POINT::default();
    unsafe {
        GetCursorPos(&mut point)
            .is_ok()
            .then_some((point.x, point.y))
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn current_cursor_pos() -> Option<(i32, i32)> {
    None
}

/// Replacement for a lone post-snip Win key-up (the physical one was
/// suppressed in the hook). Injects `[Ctrl down, Ctrl up, <Win> up]`: while Win
/// is still logically held (we ate its real up), the Ctrl tap registers as a
/// key pressed during the hold, so the trailing Win-up is no longer a "lone
/// Win" and the Start menu stays shut. `left` picks the same Win key (L/R) the
/// user released. This is the proven AutoHotkey disguise, done at the exact
/// moment of release — which is what makes it work everywhere an early /
/// stand-alone disguise tap does not. (rdev fallback path; the default native
/// backend injects synchronously from its own hook instead.)
#[cfg(target_os = "windows")]
fn inject_disguised_winup(left: bool) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
        VIRTUAL_KEY, VK_CONTROL, VK_LCONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
    };
    let win_vk = if left { VK_LWIN } else { VK_RWIN };
    let key = |vk: VIRTUAL_KEY, up: bool| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: if up {
                    KEYEVENTF_KEYUP
                } else {
                    KEYBD_EVENT_FLAGS(0)
                },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    // Skip the Ctrl mask tap when another modifier is already physically held:
    // the Win-up is then non-lone on its own, and an injected Ctrl would form a
    // real chord — Ctrl+Shift is the default "switch keyboard layout" hotkey on
    // multi-layout (e.g. Turkish) systems, so a Win+Shift+drag snip would flip
    // the layout on every release. Inject only the Win-up in that case; still
    // swallow the physical up (done by the caller). Mirrors the native backend.
    let other_mod_down =
        async_key_down(VK_SHIFT.0) || async_key_down(VK_MENU.0) || async_key_down(VK_CONTROL.0);
    // `full` is [Ctrl down, Ctrl up, Win up]; the masked case sends only the
    // trailing Win-up (`&full[2..]`).
    let full = [
        key(VK_LCONTROL, false),
        key(VK_LCONTROL, true),
        key(win_vk, true),
    ];
    let inputs: &[INPUT] = if other_mod_down { &full[2..] } else { &full };
    let sent = unsafe { SendInput(inputs, std::mem::size_of::<INPUT>() as i32) };
    log::info!(
        "disguised Win-up injected ({}); SendInput sent {sent}/{}",
        if other_mod_down { "bare Win up" } else { "Ctrl tap + Win up" },
        inputs.len()
    );
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn inject_disguised_winup(_left: bool) {}

/// Called after a Win+LMouse snip finishes or cancels. Deliberately does NOT
/// synthesize modifier KEYUPs: releasing Win/Ctrl/Alt/Shift here would (a)
/// drop modifiers the user is *still physically holding*, which breaks the
/// "keep Win held and drag several times to take multiple snips in a row"
/// flow, and (b) can stop Windows from later delivering the real physical
/// Win-up, leaving the key stuck down.
///
/// The Start-menu problem itself is handled at release time (the armed Win-up
/// is swallowed and replaced in the hook — see `DISGUISE_WINUP_ARMED`), so
/// all that's left here is the native backend's modifier-drift reconciliation.
pub fn force_release_modifiers() {
    if native_backend_enabled() {
        crate::native_mouse_trigger::force_release_modifiers();
    }
    // rdev fallback: nothing to do — its tracked state self-corrects against
    // GetAsyncKeyState on every check (see `modifier_down_now`).
}
