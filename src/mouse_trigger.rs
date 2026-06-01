use crossbeam_channel::{unbounded, Receiver, Sender};
use parking_lot::Mutex;
use rdev::{grab, Button, Event, EventType, Key};
use std::sync::atomic::{AtomicBool, Ordering};
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

/// Arm a one-shot drag-snip (called from the tray menu).
pub fn arm_snip() {
    SNIP_ARMED.store(true, Ordering::Relaxed);
}

/// Take-and-clear the one-shot arm flag. Shared with the native backend.
pub(crate) fn consume_armed() -> bool {
    SNIP_ARMED.swap(false, Ordering::Relaxed)
}

/// Clear the freeze-first intent from the app side so neither hook backend
/// stays armed after a failed, cancelled, or abandoned freeze.
pub fn clear_freeze() {
    FREEZE_ARMED.store(false, Ordering::Relaxed);
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
    Move { x: i32, y: i32 },
    Release { x: i32, y: i32 },
    Cancel,
}

pub struct MouseTriggerService {
    pub rx: Receiver<MouseTriggerEvent>,
}

impl MouseTriggerService {
    /// Backend-agnostic constructor: the caller polls `self.rx` on its
    /// own schedule (Slint timer, native loop, etc).
    pub fn start_polling() -> Self {
        let (tx, rx) = unbounded::<MouseTriggerEvent>();
        if native_backend_enabled() {
            log::info!("mouse trigger backend: native Windows hook");
            crate::native_mouse_trigger::spawn_listener(tx);
        } else {
            log::info!("mouse trigger backend: rdev");
            spawn_listener(tx);
        }
        Self { rx }
    }
}

fn native_backend_enabled() -> bool {
    #[cfg(target_os = "windows")]
    {
        static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *ENABLED.get_or_init(|| {
            ["SNIPDECK_NATIVE_MOUSE_HOOK", "SNIPPER_NATIVE_MOUSE_HOOK"]
                .iter()
                .any(|name| {
                    std::env::var(name)
                        .map(|value| {
                            matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES")
                        })
                        .unwrap_or(false)
                })
        })
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

#[derive(Default)]
struct HookState {
    win_down: AtomicBool,
    ctrl_down: AtomicBool,
    alt_down: AtomicBool,
    shift_down: AtomicBool,
    snip_active: AtomicBool,
}

fn spawn_listener(tx: Sender<MouseTriggerEvent>) {
    std::thread::spawn(move || {
        let state = Arc::new(HookState::default());
        let last_pos = Arc::new(Mutex::new((0i32, 0i32)));

        let state_cb = state.clone();
        let pos_cb = last_pos.clone();
        let tx_cb = tx.clone();

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
        //   - To still keep the Start Menu from popping after a
        //     Win+LMouse snip we inject a lone Win+F23 filler combo
        //     (see `force_release_modifiers`), which masks the menu
        //     without touching the real modifier state.
        let result = grab(move |event: Event| -> Option<Event> {
            let s = &*state_cb;
            match event.event_type {
                EventType::KeyPress(k) => {
                    match k {
                        Key::MetaLeft | Key::MetaRight => {
                            s.win_down.store(true, Ordering::Relaxed);
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
                        Key::Escape
                            if s.snip_active.load(Ordering::Relaxed)
                                || FREEZE_ARMED.load(Ordering::Relaxed) =>
                        {
                            s.snip_active.store(false, Ordering::Relaxed);
                            FREEZE_ARMED.store(false, Ordering::Relaxed);
                            let _ = tx_cb.send(MouseTriggerEvent::Cancel);
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
                                let (x, y) = cursor_pos_or_last(&pos_cb);
                                let _ = tx_cb.send(MouseTriggerEvent::FreezeArm { x, y });
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
                    let armed = consume_armed();
                    // freeze_armed lets the click start the selection even with
                    // NO modifier held — the screen is already frozen
                    // (Win+Shift+Space "freeze first", Win+Shift+S style).
                    if (win_down_now(s) || armed || FREEZE_ARMED.load(Ordering::Relaxed))
                        && !s.snip_active.load(Ordering::Relaxed)
                    {
                        s.snip_active.store(true, Ordering::Relaxed);
                        FREEZE_ARMED.store(false, Ordering::Relaxed);
                        let (x, y) = cursor_pos_or_last(&pos_cb);
                        let mods = modifier_snapshot(s);
                        let _ = tx_cb.send(MouseTriggerEvent::StartSnip { x, y, mods });
                        // Immediately give the shell a "Win+combo" signal
                        // so even if the user releases Win mid-drag, the
                        // Start Menu does NOT pop up. Done on a worker
                        // thread to avoid re-entering the LL hook from
                        // inside its own callback.
                        std::thread::spawn(inject_filler_only);
                        // Suppress the click so apps below don't react.
                        return None;
                    }
                    Some(event)
                }
                EventType::ButtonRelease(Button::Left) => {
                    if s.snip_active.load(Ordering::Relaxed) {
                        s.snip_active.store(false, Ordering::Relaxed);
                        let (x, y) = cursor_pos_or_last(&pos_cb);
                        let _ = tx_cb.send(MouseTriggerEvent::Release { x, y });
                        return None;
                    }
                    Some(event)
                }
                EventType::MouseMove { x, y } => {
                    let p = (x as i32, y as i32);
                    *pos_cb.lock() = p;
                    if s.snip_active.load(Ordering::Relaxed) {
                        let _ = tx_cb.send(MouseTriggerEvent::Move { x: p.0, y: p.1 });
                    }
                    Some(event)
                }
                _ => Some(event),
            }
        });

        if let Err(e) = result {
            eprintln!("rdev grab error: {e:?}");
        }
    });
}

fn modifier_snapshot(state: &HookState) -> ModifierSet {
    ModifierSet {
        ctrl: modifier_down_now(&state.ctrl_down, &[0x11, 0xA2, 0xA3]),
        alt: modifier_down_now(&state.alt_down, &[0x12, 0xA4, 0xA5]),
        shift: modifier_down_now(&state.shift_down, &[0x10, 0xA0, 0xA1]),
    }
}

fn win_down_now(state: &HookState) -> bool {
    modifier_down_now(&state.win_down, &[0x5B, 0x5C])
}

fn shift_down_now(state: &HookState) -> bool {
    modifier_down_now(&state.shift_down, &[0x10, 0xA0, 0xA1])
}

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

#[cfg(not(target_os = "windows"))]
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

#[cfg(not(target_os = "windows"))]
fn current_cursor_pos() -> Option<(i32, i32)> {
    None
}

/// Inject just the filler-key combo (no modifier KEYUPs). Called from
/// the Win+LMouse press handler so the Windows shell sees a proper
/// "Win+combo" early — protects against the user releasing Win
/// before the snip is finished.
fn inject_filler_only() {
    #[cfg(target_os = "windows")]
    inject_filler_only_win();
}

#[cfg(target_os = "windows")]
fn inject_filler_only_win() {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
        VIRTUAL_KEY,
    };
    const VK_F23: u16 = 0x86;
    let make = |up: bool| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(VK_F23),
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
    let inputs = [make(false), make(true)];
    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

/// Called after a Win+LMouse snip. Masks the Win key (injects a Win+F23
/// combo) so a lone Win press can't pop the Start menu — but deliberately
/// does NOT synthesize modifier KEYUPs.
///
/// Releasing Win/Ctrl/Alt/Shift here would (a) drop modifiers the user is
/// *still physically holding*, which breaks the "keep Win held and drag
/// several times to take multiple snips in a row" flow, and (b) can stop
/// Windows from later delivering the real physical Win-up, leaving the key
/// stuck down. The hook already tracks the real physical state and
/// GetAsyncKeyState corrects any drift, so only the F23 menu-mask is needed.
/// (This mirrors the native backend's `finish_snip_win`.)
pub fn force_release_modifiers() {
    if native_backend_enabled() {
        crate::native_mouse_trigger::force_release_modifiers();
        return;
    }
    #[cfg(target_os = "windows")]
    std::thread::spawn(inject_filler_only_win);
}
