use crate::settings::Settings;
use anyhow::{anyhow, Context};
use crossbeam_channel::{unbounded, Receiver};
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HotkeyAction {
    Snip,
    SnipToClipboard,
    ClipboardOnly,
    ToggleVisibility,
    Gallery,
}

pub struct HotkeyService {
    backend: Backend,
}

/// Which mechanism actually owns the shortcuts.
///
/// `Native` is the `global-hotkey` crate: Win32 on Windows, X11 on Unix. Under
/// Wayland the X11 side registers against XWayland and never receives a press,
/// so `Portal` takes over there — see `portal_hotkeys` for why that is the only
/// workable route and what changes for the user.
enum Backend {
    Native {
        _manager: GlobalHotKeyManager,
        bindings: Vec<(u32, HotkeyAction)>,
        rx: Receiver<GlobalHotKeyEvent>,
    },
    #[cfg(target_os = "linux")]
    Portal {
        rx: Receiver<HotkeyAction>,
    },
}

impl HotkeyService {
    pub fn start(settings: &Settings) -> anyhow::Result<Self> {
        // Wayland first, and only on Wayland: an X11 session keeps the native
        // path, where the user's configured combo applies verbatim with no
        // portal dialog and no compositor veto.
        #[cfg(target_os = "linux")]
        if crate::portal_hotkeys::should_use_portal() {
            match crate::portal_hotkeys::spawn(settings) {
                Ok(rx) => {
                    log::info!("hotkey backend: xdg-desktop-portal GlobalShortcuts");
                    return Ok(Self {
                        backend: Backend::Portal { rx },
                    });
                }
                // Not fatal: a desktop whose portal lacks GlobalShortcuts still
                // gets whatever XWayland manages to deliver, which is better
                // than refusing to start.
                Err(e) => log::warn!(
                    "portal global shortcuts unavailable ({e:#}); falling back to the X11 backend"
                ),
            }
        }

        let manager = GlobalHotKeyManager::new().context("init global hotkey manager")?;
        let mut bindings = Vec::new();

        // Route hotkey events through our own channel via a handler (rather
        // than the crate's default global receiver) so we can WAKE THE PUMP
        // on each press. A hotkey can fire while Snipdeck sleeps in the tray;
        // the handler runs on the crate's own thread, forwards the event and
        // wakes us so `poll()` delivers it. `wake()` only fires on a real
        // registered hotkey — not on every keystroke.
        let (tx, rx) = unbounded();
        GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
            let _ = tx.send(event);
            crate::pump::wake();
        }));

        // Register each hotkey independently. A single combo that Windows or
        // another app has already reserved (more likely for Win-based combos)
        // must NOT take down the rest — log it and carry on with whatever
        // registers, so the user keeps every shortcut that IS available.
        let mut register = |s: &str, action: HotkeyAction| {
            match parse_hotkey(s) {
                Ok(hk) => match manager.register(hk) {
                    Ok(()) => bindings.push((hk.id(), action)),
                    Err(e) => log::warn!(
                        "could not register hotkey {s:?}: {e} (it may be reserved by Windows or another app)"
                    ),
                },
                Err(e) => log::warn!("invalid hotkey {s:?}: {e}"),
            }
        };

        register(&settings.hotkey_snip, HotkeyAction::Snip);
        register(&settings.hotkey_snip_clip, HotkeyAction::SnipToClipboard);
        register(&settings.hotkey_clip_only, HotkeyAction::ClipboardOnly);
        register(&settings.hotkey_toggle_visibility, HotkeyAction::ToggleVisibility);
        register(&settings.hotkey_gallery, HotkeyAction::Gallery);

        log::info!("hotkey backend: global-hotkey ({} bound)", bindings.len());
        Ok(Self {
            backend: Backend::Native {
                _manager: manager,
                bindings,
                rx,
            },
        })
    }

    /// Drain pending global-hotkey events and translate them to actions in our
    /// inbox. Call this from the pump loop.
    pub fn poll(&self) -> Vec<HotkeyAction> {
        let mut out = Vec::new();
        match &self.backend {
            Backend::Native { bindings, rx, .. } => {
                while let Ok(event) = rx.try_recv() {
                    if event.state != global_hotkey::HotKeyState::Pressed {
                        continue;
                    }
                    if let Some(action) = bindings
                        .iter()
                        .find_map(|(id, a)| (*id == event.id).then_some(*a))
                    {
                        out.push(action);
                    }
                }
            }
            // The portal thread has already resolved the shortcut id to an
            // action and only emits presses, so there is nothing to filter.
            #[cfg(target_os = "linux")]
            Backend::Portal { rx } => {
                while let Ok(action) = rx.try_recv() {
                    out.push(action);
                }
            }
        }
        out
    }
}

fn parse_hotkey(s: &str) -> anyhow::Result<HotKey> {
    let parts: Vec<&str> = s.split('+').map(|p| p.trim()).collect();
    if parts.is_empty() {
        return Err(anyhow!("empty hotkey"));
    }
    let (key, mods_str) = parts.split_last().unwrap();
    let mut mods = Modifiers::empty();
    for m in mods_str {
        for token in split_modifiers(m) {
            match token.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => mods |= Modifiers::CONTROL,
                "shift" => mods |= Modifiers::SHIFT,
                "alt" => mods |= Modifiers::ALT,
                "win" | "super" | "cmd" | "meta" => mods |= Modifiers::META,
                "" => {}
                other => return Err(anyhow!("unknown modifier: {other}")),
            }
        }
    }
    let code = parse_code(key).ok_or_else(|| anyhow!("unknown key: {key}"))?;
    Ok(HotKey::new(Some(mods), code))
}

/// Split a token like "CtrlShift" into ["Ctrl", "Shift"]: the settings format
/// packs modifiers without separators. Shared with the portal backend.
pub(crate) fn split_modifiers(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for ch in s.chars() {
        if ch.is_ascii_uppercase() && !current.is_empty() {
            out.push(std::mem::take(&mut current));
        }
        current.push(ch);
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

fn parse_code(s: &str) -> Option<Code> {
    let upper = s.to_ascii_uppercase();
    let code = match upper.as_str() {
        "A" => Code::KeyA,
        "B" => Code::KeyB,
        "C" => Code::KeyC,
        "D" => Code::KeyD,
        "E" => Code::KeyE,
        "F" => Code::KeyF,
        "G" => Code::KeyG,
        "H" => Code::KeyH,
        "I" => Code::KeyI,
        "J" => Code::KeyJ,
        "K" => Code::KeyK,
        "L" => Code::KeyL,
        "M" => Code::KeyM,
        "N" => Code::KeyN,
        "O" => Code::KeyO,
        "P" => Code::KeyP,
        "Q" => Code::KeyQ,
        "R" => Code::KeyR,
        "S" => Code::KeyS,
        "T" => Code::KeyT,
        "U" => Code::KeyU,
        "V" => Code::KeyV,
        "W" => Code::KeyW,
        "X" => Code::KeyX,
        "Y" => Code::KeyY,
        "Z" => Code::KeyZ,
        "0" => Code::Digit0,
        "1" => Code::Digit1,
        "2" => Code::Digit2,
        "3" => Code::Digit3,
        "4" => Code::Digit4,
        "5" => Code::Digit5,
        "6" => Code::Digit6,
        "7" => Code::Digit7,
        "8" => Code::Digit8,
        "9" => Code::Digit9,
        "F1" => Code::F1,
        "F2" => Code::F2,
        "F3" => Code::F3,
        "F4" => Code::F4,
        "F5" => Code::F5,
        "F6" => Code::F6,
        "F7" => Code::F7,
        "F8" => Code::F8,
        "F9" => Code::F9,
        "F10" => Code::F10,
        "F11" => Code::F11,
        "F12" => Code::F12,
        "PRTSC" | "PRINTSCREEN" | "PRT" => Code::PrintScreen,
        "INSERT" | "INS" => Code::Insert,
        "DELETE" | "DEL" => Code::Delete,
        "HOME" => Code::Home,
        "END" => Code::End,
        "PAGEUP" | "PGUP" => Code::PageUp,
        "PAGEDOWN" | "PGDN" => Code::PageDown,
        "SPACE" => Code::Space,
        "ENTER" | "RETURN" => Code::Enter,
        _ => return None,
    };
    Some(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ctrl_shift_a() {
        let hk = parse_hotkey("CtrlShift+A").unwrap();
        assert_eq!(hk.mods, Modifiers::CONTROL | Modifiers::SHIFT);
        assert_eq!(hk.key, Code::KeyA);
    }

    #[test]
    fn parses_plus_separated() {
        let hk = parse_hotkey("Ctrl+Alt+F12").unwrap();
        assert_eq!(hk.mods, Modifiers::CONTROL | Modifiers::ALT);
        assert_eq!(hk.key, Code::F12);
    }
}
