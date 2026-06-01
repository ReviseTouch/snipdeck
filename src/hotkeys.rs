use crate::settings::Settings;
use anyhow::{anyhow, Context};
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
    _manager: GlobalHotKeyManager,
    bindings: Vec<(u32, HotkeyAction)>,
}

impl HotkeyService {
    pub fn start(settings: &Settings) -> anyhow::Result<Self> {
        let manager = GlobalHotKeyManager::new().context("init global hotkey manager")?;
        let mut bindings = Vec::new();

        let mut try_register = |s: &str, action: HotkeyAction| -> anyhow::Result<()> {
            let hk = parse_hotkey(s)?;
            manager
                .register(hk)
                .with_context(|| format!("register hotkey {s}"))?;
            bindings.push((hk.id(), action));
            Ok(())
        };

        try_register(&settings.hotkey_snip, HotkeyAction::Snip)?;
        try_register(&settings.hotkey_snip_clip, HotkeyAction::SnipToClipboard)?;
        try_register(&settings.hotkey_clip_only, HotkeyAction::ClipboardOnly)?;
        try_register(
            &settings.hotkey_toggle_visibility,
            HotkeyAction::ToggleVisibility,
        )?;
        try_register(&settings.hotkey_gallery, HotkeyAction::Gallery)?;

        Ok(Self {
            _manager: manager,
            bindings,
        })
    }

    /// Drain global hotkey events from the global-hotkey crate's receiver and
    /// translate them to actions in our inbox. Call this from the egui update loop.
    pub fn poll(&self) -> Vec<HotkeyAction> {
        let receiver = GlobalHotKeyEvent::receiver();
        let mut out = Vec::new();
        while let Ok(event) = receiver.try_recv() {
            if event.state != global_hotkey::HotKeyState::Pressed {
                continue;
            }
            if let Some(action) = self
                .bindings
                .iter()
                .find_map(|(id, a)| (*id == event.id).then_some(*a))
            {
                out.push(action);
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

/// Split a token like "CtrlShift" into ["Ctrl", "Shift"].
fn split_modifiers(s: &str) -> Vec<String> {
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
