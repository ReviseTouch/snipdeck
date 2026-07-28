use crossbeam_channel::Sender;

use crate::mouse_trigger::MouseTriggerEvent;

#[cfg(not(target_os = "windows"))]
pub fn spawn_listener(_tx: Sender<MouseTriggerEvent>) {}

#[cfg(not(target_os = "windows"))]
pub fn force_release_modifiers() {}

#[cfg(target_os = "windows")]
pub fn spawn_listener(tx: Sender<MouseTriggerEvent>) {
    std::thread::spawn(move || imp::run(tx));
}

#[cfg(target_os = "windows")]
pub fn force_release_modifiers() {
    std::thread::spawn(imp::finish_snip_win);
}

#[cfg(target_os = "windows")]
mod imp {
    use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
    use std::sync::{Arc, OnceLock};

    use crossbeam_channel::Sender;
    use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, POINT, WPARAM};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT,
        KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_CONTROL, VK_ESCAPE, VK_LCONTROL, VK_LMENU,
        VK_LSHIFT, VK_LWIN, VK_MENU, VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SHIFT, VK_SPACE,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetCursorPos, GetMessageW, SetWindowsHookExW, TranslateMessage,
        UnhookWindowsHookEx, HC_ACTION, HHOOK, KBDLLHOOKSTRUCT, KBDLLHOOKSTRUCT_FLAGS,
        LLKHF_EXTENDED, LLKHF_INJECTED, LLKHF_LOWER_IL_INJECTED, LLMHF_INJECTED,
        LLMHF_LOWER_IL_INJECTED, MSG, MSLLHOOKSTRUCT, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN,
        WM_KEYUP, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_SYSKEYDOWN, WM_SYSKEYUP,
    };

    use crate::mouse_trigger::{ModifierSet, MouseTriggerEvent, DISGUISE_WINUP_ARMED, FREEZE_ARMED};

    const MOD_LWIN: u8 = 1 << 0;
    const MOD_RWIN: u8 = 1 << 1;
    const MOD_LCTRL: u8 = 1 << 2;
    const MOD_RCTRL: u8 = 1 << 3;
    const MOD_LALT: u8 = 1 << 4;
    const MOD_RALT: u8 = 1 << 5;
    const MOD_LSHIFT: u8 = 1 << 6;
    const MOD_RSHIFT: u8 = 1 << 7;

    const WIN_BITS: u8 = MOD_LWIN | MOD_RWIN;
    const CTRL_BITS: u8 = MOD_LCTRL | MOD_RCTRL;
    const ALT_BITS: u8 = MOD_LALT | MOD_RALT;
    const SHIFT_BITS: u8 = MOD_LSHIFT | MOD_RSHIFT;

    const SYNTHETIC_EXTRA_INFO: usize = 0x534E_4950_5045_5201;

    static HOOK_STATE: OnceLock<Arc<HookState>> = OnceLock::new();

    pub fn run(tx: Sender<MouseTriggerEvent>) {
        let state = Arc::new(HookState {
            tx,
            physical_mods: AtomicU8::new(0),
            snip_active: AtomicBool::new(false),
        });

        if HOOK_STATE.set(state).is_err() {
            log::warn!("native mouse trigger already running");
            return;
        }

        unsafe {
            let keyboard_hook = match SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(keyboard_proc),
                HINSTANCE::default(),
                0,
            ) {
                Ok(hook) => hook,
                Err(err) => {
                    log::error!("native keyboard hook failed: {err}");
                    return;
                }
            };

            let mouse_hook =
                match SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), HINSTANCE::default(), 0) {
                    Ok(hook) => hook,
                    Err(err) => {
                        log::error!("native mouse hook failed: {err}");
                        let _ = UnhookWindowsHookEx(keyboard_hook);
                        return;
                    }
                };

            log::info!("native mouse trigger active");

            let mut msg = MSG::default();
            loop {
                let result = GetMessageW(&mut msg, None, 0, 0);
                if result.0 <= 0 {
                    break;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            let _ = UnhookWindowsHookEx(mouse_hook);
            let _ = UnhookWindowsHookEx(keyboard_hook);
        }
    }

    struct HookState {
        tx: Sender<MouseTriggerEvent>,
        physical_mods: AtomicU8,
        snip_active: AtomicBool,
    }

    impl HookState {
        fn update_modifier(
            &self,
            vk: u16,
            scan_code: u32,
            flags: KBDLLHOOKSTRUCT_FLAGS,
            down: bool,
        ) {
            let Some(bit) = modifier_bit(vk, scan_code, flags) else {
                return;
            };
            if down {
                self.physical_mods.fetch_or(bit, Ordering::Relaxed);
            } else {
                self.physical_mods.fetch_and(!bit, Ordering::Relaxed);
            }
        }

        fn reconcile_modifier_state(&self) {
            let bits = self.physical_mods.load(Ordering::Relaxed);
            let mut stale = 0;
            if (bits & MOD_LWIN) != 0 && !async_key_down(VK_LWIN.0) {
                stale |= MOD_LWIN;
            }
            if (bits & MOD_RWIN) != 0 && !async_key_down(VK_RWIN.0) {
                stale |= MOD_RWIN;
            }
            if (bits & MOD_LCTRL) != 0 && !async_key_down(VK_LCONTROL.0) {
                stale |= MOD_LCTRL;
            }
            if (bits & MOD_RCTRL) != 0 && !async_key_down(VK_RCONTROL.0) {
                stale |= MOD_RCTRL;
            }
            if (bits & MOD_LALT) != 0 && !async_key_down(VK_LMENU.0) {
                stale |= MOD_LALT;
            }
            if (bits & MOD_RALT) != 0 && !async_key_down(VK_RMENU.0) {
                stale |= MOD_RALT;
            }
            if (bits & MOD_LSHIFT) != 0 && !async_key_down(VK_LSHIFT.0) {
                stale |= MOD_LSHIFT;
            }
            if (bits & MOD_RSHIFT) != 0 && !async_key_down(VK_RSHIFT.0) {
                stale |= MOD_RSHIFT;
            }
            if stale != 0 {
                self.physical_mods.fetch_and(!stale, Ordering::Relaxed);
                log::warn!("native hook cleared stale modifier bits: {stale:#010b}");
            }
        }

        fn win_down(&self) -> bool {
            self.reconcile_modifier_state();
            (self.physical_mods.load(Ordering::Relaxed) & WIN_BITS) != 0
        }

        fn shift_down(&self) -> bool {
            (self.physical_mods.load(Ordering::Relaxed) & SHIFT_BITS) != 0
        }

        fn modifier_snapshot(&self) -> ModifierSet {
            let bits = self.physical_mods.load(Ordering::Relaxed);
            ModifierSet {
                ctrl: (bits & CTRL_BITS) != 0,
                alt: (bits & ALT_BITS) != 0,
                shift: (bits & SHIFT_BITS) != 0,
            }
        }
    }

    unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code != HC_ACTION as i32 {
            return call_next(code, wparam, lparam);
        }

        let Some(state) = HOOK_STATE.get() else {
            return call_next(code, wparam, lparam);
        };

        let event = *(lparam.0 as *const KBDLLHOOKSTRUCT);
        let message = wparam.0 as u32;
        let is_down = message == WM_KEYDOWN || message == WM_SYSKEYDOWN;
        let is_up = message == WM_KEYUP || message == WM_SYSKEYUP;

        // Snapshot the tracked Win state BEFORE `update_modifier` folds this
        // event in — the stale-arm clear further down needs to know whether Win
        // was already (physically) down to tell an auto-repeat from a fresh
        // press (WH_KEYBOARD_LL carries no previous-state bit).
        let win_was_down = (state.physical_mods.load(Ordering::Relaxed) & WIN_BITS) != 0;

        if (is_down || is_up) && !keyboard_event_is_injected(event.flags) {
            state.update_modifier(event.vkCode as u16, event.scanCode, event.flags, is_down);
        }

        let vk = event.vkCode as u16;

        if is_down && vk == VK_ESCAPE.0 {
            let was_color = crate::mouse_trigger::consume_color_pick();
            let was_active = state.snip_active.swap(false, Ordering::Relaxed);
            let was_armed = FREEZE_ARMED.swap(false, Ordering::Relaxed);
            // Also cancel a one-shot arm (toolbar/tray/hotkey "New Snip") that
            // hasn't been clicked yet — otherwise Esc is dead, the hidden main
            // window never comes back, and the next ordinary left-click gets
            // hijacked into a snip.
            let was_snip_armed = crate::mouse_trigger::consume_armed();
            crate::mouse_trigger::set_snip_active(false);
            if was_color || was_active || was_armed || was_snip_armed {
                let _ = state.tx.send(MouseTriggerEvent::Cancel);
                crate::pump::wake();
            }
        }

        // A physical Win key-up while the disguise is armed (a Win+drag snip
        // happened during this hold): swallow it and synchronously replace it
        // with [Ctrl tap + Win-up] so the OS never sees a "lone Win" and the
        // Start menu stays shut. Injected events must pass through untouched —
        // the replacement's own Win-up arrives right back here.
        if is_up
            && (vk == VK_LWIN.0 || vk == VK_RWIN.0)
            && !keyboard_event_is_injected(event.flags)
            && DISGUISE_WINUP_ARMED.swap(false, Ordering::Relaxed)
        {
            if inject_disguised_winup_sync(vk) {
                return LRESULT(1);
            }
            // Injection failed (UIPI — an elevated window has focus?). Let
            // the physical Win-up through: the Start menu may open, but a
            // suppressed-and-unreplaced up would leave the Win key stuck
            // down at the OS level, which is far worse.
        }
        // A physical Win press *may* clear a stale arm (a release our hook never
        // saw, e.g. one that landed on the secure desktop), so normal Start-menu
        // usage isn't eaten later — but ONLY on a genuinely FRESH press. A held
        // Win key streams typematic-repeat KEYDOWNs, and WH_KEYBOARD_LL cannot
        // tell a repeat from a fresh press (KBDLLHOOKSTRUCT has no previous-state
        // bit). Clearing on a repeat would drop the disguise mid-hold — user
        // holds Win, drags (arm set on WM_LBUTTONDOWN), a Win auto-repeat clears
        // the arm, then Win-up passes through undisguised and pops Start. This is
        // deterministic on keyboards/drivers that deliver held-modifier repeats
        // (many laptops, gaming keyboards, RDP): the "works on dev machines,
        // fails on some PCs" signature. So: if Win wasn't already tracked down
        // this is unambiguously fresh → clear. If it was, it's either a real
        // repeat or a stale bit (a release we never saw); disambiguate with
        // GetAsyncKeyState's pre-event hardware state — DOWN = real repeat (keep
        // the arm), UP = stale bit (treat as fresh → clear).
        if is_down
            && (vk == VK_LWIN.0 || vk == VK_RWIN.0)
            && !keyboard_event_is_injected(event.flags)
            && (!win_was_down || !async_key_down(vk))
        {
            DISGUISE_WINUP_ARMED.store(false, Ordering::Relaxed);
        }

        // Win+Shift+Space → freeze the screen now (capture hover tooltips),
        // then drag-select on the still image. Suppress Space so it doesn't
        // leak. `win_down()` reconciles modifier state.
        if is_down
            && vk == VK_SPACE.0
            && !state.snip_active.load(Ordering::Relaxed)
            && state.win_down()
            && state.shift_down()
        {
            // Swallow EVERY Win+Shift+Space (including key-repeat); fire
            // FreezeArm only once.
            if !FREEZE_ARMED.swap(true, Ordering::Relaxed) {
                // The Space that would have counted as "a key during the Win
                // hold" never reaches the OS (we swallow it), so the eventual
                // Win-up could still read as a lone Win. Arm the release-time
                // disguise here too — Win is held by definition of the chord.
                DISGUISE_WINUP_ARMED.store(true, Ordering::Relaxed);
                if let Some((x, y)) = cursor_pos() {
                    let _ = state.tx.send(MouseTriggerEvent::FreezeArm { x, y });
                    crate::pump::wake();
                }
            }
            return LRESULT(1);
        }

        // Win+Shift+Space is "freeze first, Win+Shift+S style": once armed,
        // releasing Win/Shift does NOT cancel — the user lets go and then drags
        // on the frozen image. Only Esc (or the click) clears it.

        call_next(code, wparam, lparam)
    }

    unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code != HC_ACTION as i32 {
            return call_next(code, wparam, lparam);
        }

        let Some(state) = HOOK_STATE.get() else {
            return call_next(code, wparam, lparam);
        };

        let event = *(lparam.0 as *const MSLLHOOKSTRUCT);
        let message = wparam.0 as u32;
        let injected = mouse_event_is_injected(event.flags);

        match message {
            WM_LBUTTONDOWN if !injected => {
                // Colour-pick mode: grab the pixel under the cursor and
                // suppress the click (don't let it reach the app below).
                if crate::mouse_trigger::consume_color_pick() {
                    let _ = state.tx.send(MouseTriggerEvent::ColorPick {
                        x: event.pt.x,
                        y: event.pt.y,
                    });
                    crate::pump::wake();
                    return LRESULT(1);
                }
                let armed = crate::mouse_trigger::consume_armed();
                let win = state.win_down();
                // freeze_armed lets the click start the selection with NO
                // modifier held — the screen is already frozen (Win+Shift+Space
                // "freeze first", Win+Shift+S style).
                if win || armed || FREEZE_ARMED.load(Ordering::Relaxed) {
                    if !state.snip_active.swap(true, Ordering::Relaxed) {
                        crate::mouse_trigger::set_snip_active(true);
                        FREEZE_ARMED.store(false, Ordering::Relaxed);
                        // The user is holding Win: arm the release-time
                        // disguise so the eventual Win-up (whenever it comes —
                        // mid-drag or several snips later) can't pop Start.
                        if win {
                            DISGUISE_WINUP_ARMED.store(true, Ordering::Relaxed);
                        }
                        let mods = state.modifier_snapshot();
                        let _ = state.tx.send(MouseTriggerEvent::StartSnip {
                            x: event.pt.x,
                            y: event.pt.y,
                            mods,
                        });
                        crate::pump::wake();
                    }
                    return LRESULT(1);
                }
            }
            WM_LBUTTONUP => {
                if state.snip_active.swap(false, Ordering::Relaxed) {
                    crate::mouse_trigger::set_snip_active(false);
                    let _ = state.tx.send(MouseTriggerEvent::Release {
                        x: event.pt.x,
                        y: event.pt.y,
                    });
                    crate::pump::wake();
                    return LRESULT(1);
                }
            }
            WM_MOUSEMOVE => {
                if state.snip_active.load(Ordering::Relaxed) {
                    let _ = state.tx.send(MouseTriggerEvent::Move {
                        x: event.pt.x,
                        y: event.pt.y,
                    });
                    crate::pump::wake();
                }
            }
            _ => {}
        }

        call_next(code, wparam, lparam)
    }

    unsafe fn call_next(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        CallNextHookEx(HHOOK::default(), code, wparam, lparam)
    }

    fn keyboard_event_is_injected(flags: KBDLLHOOKSTRUCT_FLAGS) -> bool {
        (flags.0 & (LLKHF_INJECTED.0 | LLKHF_LOWER_IL_INJECTED.0)) != 0
    }

    fn mouse_event_is_injected(flags: u32) -> bool {
        (flags & (LLMHF_INJECTED | LLMHF_LOWER_IL_INJECTED)) != 0
    }

    fn modifier_bit(vk: u16, scan_code: u32, flags: KBDLLHOOKSTRUCT_FLAGS) -> Option<u8> {
        match VIRTUAL_KEY(vk) {
            VK_LWIN => Some(MOD_LWIN),
            VK_RWIN => Some(MOD_RWIN),
            VK_LCONTROL => Some(MOD_LCTRL),
            VK_RCONTROL => Some(MOD_RCTRL),
            VK_LMENU => Some(MOD_LALT),
            VK_RMENU => Some(MOD_RALT),
            VK_LSHIFT => Some(MOD_LSHIFT),
            VK_RSHIFT => Some(MOD_RSHIFT),
            VK_CONTROL => {
                if (flags.0 & LLKHF_EXTENDED.0) != 0 {
                    Some(MOD_RCTRL)
                } else {
                    Some(MOD_LCTRL)
                }
            }
            VK_MENU => {
                if (flags.0 & LLKHF_EXTENDED.0) != 0 {
                    Some(MOD_RALT)
                } else {
                    Some(MOD_LALT)
                }
            }
            VK_SHIFT => {
                if scan_code == 0x36 {
                    Some(MOD_RSHIFT)
                } else {
                    Some(MOD_LSHIFT)
                }
            }
            _ => None,
        }
    }

    fn async_key_down(vk: u16) -> bool {
        unsafe { (GetAsyncKeyState(vk as i32) as u16 & 0x8000) != 0 }
    }

    fn cursor_pos() -> Option<(i32, i32)> {
        let mut p = POINT::default();
        unsafe { GetCursorPos(&mut p).is_ok().then_some((p.x, p.y)) }
    }

    pub fn finish_snip_win() {
        // Native hook tracks physical modifier state from non-injected
        // events. Do not synthesize Win/Ctrl/Alt/Shift KEYUP here:
        // doing so can prevent Windows from delivering the later real
        // physical Win-up, leaving our physical state stuck down. The
        // Start-menu problem itself is handled at release time (the armed
        // Win-up is swallowed and replaced in `keyboard_proc`), so all
        // that's left here is drift correction.
        if let Some(state) = HOOK_STATE.get().cloned() {
            std::thread::spawn(move || {
                for delay_ms in [30, 120, 300] {
                    std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                    state.reconcile_modifier_state();
                }
            });
        }
    }

    /// Swallow-and-replace for a post-snip Win release: the physical Win-up
    /// was suppressed by the hook; emit `[Ctrl down, Ctrl up, Win up]` in its
    /// place. The Ctrl tap counts as a key pressed during the Win hold, so
    /// the trailing Win-up is no longer a "lone Win" and the Start menu stays
    /// shut — AutoHotkey's MenuMaskKey technique, applied at the exact moment
    /// of release. Ctrl (not F23): inert keys never cancelled the lone-Win on
    /// Win10, and F23 formed the hardware Copilot chord (Win+Shift+F23)
    /// during Win+Shift+drag snips on Copilot-key machines.
    ///
    /// Runs SYNCHRONOUSLY inside the hook callback: spawning a thread here
    /// opened a race where the replacement landed after other input — or
    /// never (UIPI) — leaving Win logically stuck down. SYNTHETIC_EXTRA_INFO
    /// + the injected flag keep our own hook from re-processing the
    /// replacement. Returns whether all three events were injected; on
    /// failure the caller must let the physical Win-up through.
    fn inject_disguised_winup_sync(win_vk: u16) -> bool {
        use windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS;
        // The Ctrl tap's only job is to be "a key pressed during the Win hold"
        // so the replacement Win-up isn't a lone Win (which would pop Start).
        // But if another modifier is ALREADY physically held at release time,
        // the OS has already seen a key during the hold — the Win-up is non-lone
        // on its own — and injecting Ctrl would form a real chord: Ctrl+Shift is
        // the default "switch keyboard layout" hotkey on multi-layout systems
        // (ubiquitous on Turkish setups), so a Win+Shift+drag snip would
        // silently flip the layout on every release. (A Ctrl tap while the user
        // holds Ctrl would also inject a spurious Ctrl-up.) So skip the mask tap
        // whenever Shift/Alt/Ctrl is down and inject only the Win-up; still
        // swallow+replace the physical up. GetAsyncKeyState reads live hardware.
        let other_mod_down = async_key_down(VK_SHIFT.0)
            || async_key_down(VK_MENU.0)
            || async_key_down(VK_CONTROL.0);
        let key = |vk: u16, up: bool| INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(vk),
                    wScan: 0,
                    dwFlags: if up {
                        KEYEVENTF_KEYUP
                    } else {
                        KEYBD_EVENT_FLAGS(0)
                    },
                    time: 0,
                    dwExtraInfo: SYNTHETIC_EXTRA_INFO,
                },
            },
        };
        // `full` is [Ctrl down, Ctrl up, Win up]; the masked case sends only the
        // trailing Win-up (`&full[2..]`), so `winup` stays referenced either way.
        let full = [
            key(VK_LCONTROL.0, false),
            key(VK_LCONTROL.0, true),
            key(win_vk, true),
        ];
        let inputs: &[INPUT] = if other_mod_down { &full[2..] } else { &full };
        let sent = unsafe { SendInput(inputs, std::mem::size_of::<INPUT>() as i32) };
        let ok = sent as usize == inputs.len();
        if ok {
            if other_mod_down {
                log::info!("disguised Win-up injected (bare Win up; held modifier already masks)");
            } else {
                log::info!("disguised Win-up injected (Ctrl tap + Win up)");
            }
        } else {
            log::warn!(
                "disguised Win-up injection incomplete: SendInput sent {sent}/{} \
                 (elevated window focused?); passing physical Win-up through",
                inputs.len()
            );
        }
        ok
    }
}
