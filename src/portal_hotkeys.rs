//! Global shortcuts through the XDG Desktop Portal — the only mechanism that
//! works under Wayland.
//!
//! Why this exists: the `global-hotkey` crate ships exactly two Unix backends,
//! `x11` and `no-op`. Under Wayland the X11 backend registers against XWayland,
//! which the compositor never routes global keys to, so every shortcut silently
//! dies. `rdev`'s evdev grab is not an answer either — it reads `/dev/input/*`
//! directly, which means adding the user to the `input` group and handing every
//! process running as them a system-wide keylogger.
//!
//! `org.freedesktop.portal.GlobalShortcuts` is the sanctioned path. It is a
//! freedesktop standard, not a GNOME extension: `xdg-desktop-portal-kde` and
//! `xdg-desktop-portal-gnome` both implement it (KDE shipped it first), and
//! wlroots/COSMIC backends follow the same interface. One code path covers
//! every conforming desktop — no per-DE branching anywhere below.
//!
//! Two behavioural differences from the X11 path the UI has to live with:
//!
//!   1. The app *proposes* a trigger; the compositor and the user decide. A
//!      portal dialog appears on first bind, and the user may reassign any
//!      shortcut afterwards in their desktop settings. `settings.hotkey_*` is
//!      therefore a preference, not a guarantee.
//!   2. Bindings belong to the portal session. When the session dies (portal
//!      restart, logout) the shortcuts go with it; the listener thread simply
//!      ends and the app keeps running without global shortcuts rather than
//!      taking the process down.

use std::collections::HashMap;

use anyhow::{anyhow, Context};
use crossbeam_channel::{unbounded, Receiver, Sender};
use futures_util::StreamExt;

use crate::hotkeys::HotkeyAction;
use crate::settings::Settings;

/// Stable portal-side identifiers. These are persisted by the portal against
/// the user's chosen trigger, so they must never change once shipped —
/// renaming one silently orphans whatever the user had bound to it.
const ID_SNIP: &str = "snip";
const ID_SNIP_CLIP: &str = "snip-to-clipboard";
const ID_CLIP_ONLY: &str = "clipboard-only";
const ID_TOGGLE: &str = "toggle-visibility";
const ID_GALLERY: &str = "gallery";

/// Should we prefer the portal over the X11 backend?
///
/// Wayland is the deciding factor, not the desktop: an X11 session keeps the
/// existing `global-hotkey` path, which needs no portal dialog and lets the
/// user's configured combo apply verbatim. `SNIPDECK_HOTKEY_BACKEND=portal|x11`
/// forces either way — useful when testing the portal path from an X11 session
/// (or working around a broken portal backend).
pub fn should_use_portal() -> bool {
    match std::env::var("SNIPDECK_HOTKEY_BACKEND").as_deref() {
        Ok("portal") => return true,
        Ok("x11") => return false,
        _ => {}
    }
    crate::window_env::caps().session_is_wayland
}

/// Bind the configured shortcuts through the portal and stream activations.
///
/// Returns as soon as the bind is acknowledged; the listener keeps running on
/// its own thread for the lifetime of the process. An error here means the
/// caller should fall back to the X11 backend.
pub fn spawn(settings: &Settings) -> anyhow::Result<Receiver<HotkeyAction>> {
    let wanted = [
        (ID_SNIP, HotkeyAction::Snip, "Take a snip", &settings.hotkey_snip),
        (
            ID_SNIP_CLIP,
            HotkeyAction::SnipToClipboard,
            "Take a snip and copy it to the clipboard",
            &settings.hotkey_snip_clip,
        ),
        (
            ID_CLIP_ONLY,
            HotkeyAction::ClipboardOnly,
            // What the desktop lists beside the key in its shortcut settings.
            // It said "Copy the clipboard into the deck" — the opposite of
            // what the key does, which is snip to the clipboard and keep
            // nothing.
            "Snip to the clipboard only, keeping nothing",
            &settings.hotkey_clip_only,
        ),
        (
            ID_TOGGLE,
            HotkeyAction::ToggleVisibility,
            "Show or hide all snips",
            &settings.hotkey_toggle_visibility,
        ),
        (
            ID_GALLERY,
            HotkeyAction::Gallery,
            "Open the gallery",
            &settings.hotkey_gallery,
        ),
    ];

    let mut specs = Vec::with_capacity(wanted.len());
    let mut by_id: HashMap<String, HotkeyAction> = HashMap::new();
    for (id, action, description, configured) in wanted {
        // A combo we cannot translate is not fatal: bind the shortcut with no
        // preferred trigger and let the user pick one in the portal dialog.
        let trigger = to_portal_trigger(configured);
        if trigger.is_none() && !configured.is_empty() {
            log::warn!("hotkey {configured:?} has no portal equivalent — binding {id} unassigned");
        }
        specs.push((id.to_string(), description.to_string(), trigger));
        by_id.insert(id.to_string(), action);
    }

    // Readiness means "the portal answered and we have a session" — NOT "the
    // shortcuts are bound". BindShortcuts blocks on a user-facing consent
    // dialog that can sit unanswered indefinitely, and holding up startup for
    // it would leave the app dead on screen behind its own prompt.
    let (ready_tx, ready_rx) = unbounded::<anyhow::Result<()>>();
    let (event_tx, event_rx) = unbounded::<HotkeyAction>();

    std::thread::Builder::new()
        .name("portal-hotkeys".into())
        .spawn(move || {
            let outcome = pollster::block_on(run(specs, by_id, ready_tx.clone(), event_tx));
            // Only meaningful if we failed before signalling readiness; once the
            // stream is live this just records why it ended.
            if let Err(e) = outcome {
                log::warn!("portal global shortcuts stopped: {e:#}");
                let _ = ready_tx.send(Err(e));
            }
        })
        .context("spawning portal hotkey thread")?;

    // Bounded wait on the session handshake only. A portal that never answers
    // this must not hang startup — fall back to X11 and move on.
    match ready_rx.recv_timeout(std::time::Duration::from_secs(5)) {
        Ok(Ok(())) => Ok(event_rx),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(anyhow!("portal did not open a session within 5s")),
    }
}

async fn run(
    specs: Vec<(String, String, Option<String>)>,
    by_id: HashMap<String, HotkeyAction>,
    ready_tx: Sender<anyhow::Result<()>>,
    event_tx: Sender<HotkeyAction>,
) -> anyhow::Result<()> {
    use ashpd::desktop::global_shortcuts::{
        BindShortcutsOptions, GlobalShortcuts, NewShortcut,
    };
    use ashpd::desktop::CreateSessionOptions;

    let shortcuts = GlobalShortcuts::new()
        .await
        .context("connecting to org.freedesktop.portal.GlobalShortcuts")?;
    let session = shortcuts
        .create_session(CreateSessionOptions::default())
        .await
        .context("creating a global-shortcuts session")?;

    let new_shortcuts: Vec<NewShortcut> = specs
        .iter()
        .map(|(id, description, trigger)| {
            NewShortcut::new(id.clone(), description.clone())
                .preferred_trigger(trigger.as_deref())
        })
        .collect();

    // Subscribe BEFORE binding: the portal may activate a shortcut the moment
    // the bind lands, and a stream opened afterwards would miss it.
    let mut activated = shortcuts
        .receive_activated()
        .await
        .context("subscribing to shortcut activations")?;

    // No parent window: the bind happens at startup, long before there is a
    // window worth parenting the portal dialog to.
    // The session exists and the signal stream is live: that is as much as the
    // caller can usefully wait for. Release it before the consent dialog.
    let _ = ready_tx.send(Ok(()));

    // Blocks until the user answers the portal's dialog — seconds, or never.
    // A refusal is not an error worth tearing anything down: the app keeps
    // running, simply without global shortcuts, exactly as it would if the
    // user had cleared every binding in their desktop settings.
    match shortcuts
        .bind_shortcuts(
            &session,
            &new_shortcuts,
            None,
            BindShortcutsOptions::default(),
        )
        .await
    {
        Ok(request) => match request.response() {
            Ok(bound) => log::info!(
                "portal bound {} global shortcut(s)",
                bound.shortcuts().len()
            ),
            Err(e) => {
                log::warn!("portal shortcut binding declined: {e}");
                return Ok(());
            }
        },
        Err(e) => {
            log::warn!("BindShortcuts failed: {e}");
            return Ok(());
        }
    }

    while let Some(event) = activated.next().await {
        let Some(action) = by_id.get(event.shortcut_id()).copied() else {
            log::debug!("portal activated unknown shortcut {:?}", event.shortcut_id());
            continue;
        };
        if event_tx.send(action).is_err() {
            // Receiver dropped — the app is shutting down.
            break;
        }
        // Same contract as the X11 backend: a shortcut can fire while Snipdeck
        // sleeps in the tray, so wake the pump or the event sits unread.
        crate::pump::wake();
    }

    // Keep the session alive until the stream ends, then close it politely.
    let _ = session.close().await;
    Ok(())
}

/// Translate the app's hotkey syntax ("CtrlShift+A", "WinAlt+H") into the
/// portal's ("CTRL+SHIFT+a", "LOGO+ALT+h").
///
/// The portal spec takes modifiers as CTRL/SHIFT/ALT/LOGO and the key as an
/// XKB keysym name — lowercase for plain letters, capitalised for named keys
/// like `Print` or `Return`. Returns `None` for anything we cannot express, so
/// the caller can bind the shortcut unassigned rather than send a malformed
/// trigger the portal would reject outright.
fn to_portal_trigger(s: &str) -> Option<String> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let parts: Vec<&str> = s.split('+').map(str::trim).filter(|p| !p.is_empty()).collect();
    let (key, modifier_tokens) = parts.split_last()?;

    let mut out = Vec::new();
    for token in modifier_tokens {
        for m in crate::hotkeys::split_modifiers(token) {
            let name = match m.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => "CTRL",
                "shift" => "SHIFT",
                "alt" => "ALT",
                "win" | "super" | "cmd" | "meta" | "logo" => "LOGO",
                _ => return None,
            };
            if !out.contains(&name) {
                out.push(name);
            }
        }
    }

    out.push(portal_key_name(key)?);
    Some(out.join("+"))
}

/// Map a key token to its XKB keysym name. Kept deliberately narrow: an
/// unknown key returns `None` so the shortcut binds unassigned instead of
/// failing the whole BindShortcuts call.
fn portal_key_name(key: &str) -> Option<&'static str> {
    // Single letters and digits map to themselves, lowercase.
    const LETTERS: [&str; 26] = [
        "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r",
        "s", "t", "u", "v", "w", "x", "y", "z",
    ];
    const DIGITS: [&str; 10] = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];
    const FKEYS: [&str; 12] = [
        "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12",
    ];

    let upper = key.to_ascii_uppercase();
    if upper.len() == 1 {
        let c = upper.as_bytes()[0];
        if c.is_ascii_uppercase() {
            return Some(LETTERS[(c - b'A') as usize]);
        }
        if c.is_ascii_digit() {
            return Some(DIGITS[(c - b'0') as usize]);
        }
    }
    if let Some(idx) = FKEYS.iter().position(|f| *f == upper) {
        return Some(FKEYS[idx]);
    }
    Some(match upper.as_str() {
        "SPACE" => "space",
        "ENTER" | "RETURN" => "Return",
        "PRTSC" | "PRINTSCREEN" | "PRT" => "Print",
        "INSERT" | "INS" => "Insert",
        "DELETE" | "DEL" => "Delete",
        "HOME" => "Home",
        "END" => "End",
        "PAGEUP" | "PGUP" => "Prior",
        "PAGEDOWN" | "PGDN" => "Next",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translates_ctrl_shift_letter() {
        assert_eq!(to_portal_trigger("CtrlShift+A").as_deref(), Some("CTRL+SHIFT+a"));
    }

    #[test]
    fn translates_the_default_ctrl_alt_shift_letter() {
        assert_eq!(
            to_portal_trigger("CtrlAltShift+A").as_deref(),
            Some("CTRL+ALT+SHIFT+a")
        );
    }

    #[test]
    fn translates_win_alt_letter() {
        assert_eq!(to_portal_trigger("WinAlt+H").as_deref(), Some("LOGO+ALT+h"));
    }

    #[test]
    fn translates_plus_separated_modifiers() {
        assert_eq!(
            to_portal_trigger("Ctrl+Alt+F12").as_deref(),
            Some("CTRL+ALT+F12")
        );
    }

    #[test]
    fn translates_named_keys() {
        assert_eq!(to_portal_trigger("Ctrl+PrtSc").as_deref(), Some("CTRL+Print"));
        assert_eq!(to_portal_trigger("Shift+Space").as_deref(), Some("SHIFT+space"));
    }

    #[test]
    fn rejects_what_it_cannot_express() {
        assert_eq!(to_portal_trigger(""), None);
        assert_eq!(to_portal_trigger("Hyper+A"), None);
        assert_eq!(to_portal_trigger("Ctrl+Pause"), None);
    }

    #[test]
    fn deduplicates_repeated_modifiers() {
        assert_eq!(
            to_portal_trigger("CtrlCtrl+A").as_deref(),
            Some("CTRL+a")
        );
    }
}
