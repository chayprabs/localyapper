//! Wayland: `org.freedesktop.portal.GlobalShortcuts` through `ashpd`.
//!
//! The compositor owns the bindings; the user may be asked to confirm
//! them. Record, paste-last and open-app are bound together. The cancel
//! key is not bound on Wayland: a portal binding is permanent, so Escape
//! would be taken from every app (M13). Use the overlay or
//! `localyapper --cancel` there.

use super::{dispatch, HotkeyBackend, HotkeyId};
use crate::controller::ControllerHandle;
use ashpd::desktop::global_shortcuts::{BindShortcutsOptions, GlobalShortcuts, NewShortcut};
use futures_util::StreamExt;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Notify;

/// Convert "Ctrl+Alt+J" to the portal's "CTRL+ALT+j" trigger format.
pub fn to_portal_trigger(combo: &str) -> String {
    combo
        .split('+')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(|p| match p.to_lowercase().as_str() {
            "ctrl" | "control" => "CTRL".to_string(),
            "alt" | "option" => "ALT".to_string(),
            "shift" => "SHIFT".to_string(),
            "super" | "meta" | "cmd" | "command" | "logo" => "LOGO".to_string(),
            _ if p.chars().count() == 1 => p.to_lowercase(),
            other => keysym_name(other).map_or_else(|| p.to_string(), str::to_string),
        })
        .collect::<Vec<_>>()
        .join("+")
}

/// The XKB keysym name the portal expects for a key the settings window
/// names after its physical position ("ArrowUp", "Backquote"). Function
/// keys and letters already match and are passed through.
fn keysym_name(lower: &str) -> Option<&'static str> {
    Some(match lower {
        "space" => "space",
        "escape" | "esc" => "Escape",
        "enter" => "Return",
        "tab" => "Tab",
        "backspace" => "BackSpace",
        "delete" => "Delete",
        "insert" => "Insert",
        "home" => "Home",
        "end" => "End",
        "pageup" => "Page_Up",
        "pagedown" => "Page_Down",
        "arrowup" | "up" => "Up",
        "arrowdown" | "down" => "Down",
        "arrowleft" | "left" => "Left",
        "arrowright" | "right" => "Right",
        "backquote" => "grave",
        "minus" => "minus",
        "equal" => "equal",
        "bracketleft" => "bracketleft",
        "bracketright" => "bracketright",
        "backslash" => "backslash",
        "semicolon" => "semicolon",
        "quote" => "apostrophe",
        "comma" => "comma",
        "period" => "period",
        "slash" => "slash",
        "capslock" => "Caps_Lock",
        "scrolllock" => "Scroll_Lock",
        "pause" => "Pause",
        "printscreen" => "Print",
        "numpad0" => "KP_0",
        "numpad1" => "KP_1",
        "numpad2" => "KP_2",
        "numpad3" => "KP_3",
        "numpad4" => "KP_4",
        "numpad5" => "KP_5",
        "numpad6" => "KP_6",
        "numpad7" => "KP_7",
        "numpad8" => "KP_8",
        "numpad9" => "KP_9",
        "numpadadd" => "KP_Add",
        "numpadsubtract" => "KP_Subtract",
        "numpadmultiply" => "KP_Multiply",
        "numpaddivide" => "KP_Divide",
        "numpaddecimal" => "KP_Decimal",
        "numpadenter" => "KP_Enter",
        _ => return None,
    })
}

struct Shared {
    /// Wanted bindings (id -> combo). Cancel is tracked but never bound.
    wanted: Mutex<HashMap<HotkeyId, String>>,
    rebind: Notify,
}

pub struct PortalBackend {
    shared: Arc<Shared>,
}

impl PortalBackend {
    /// Fails fast when no portal answers, so the caller can fall back.
    pub fn start(ctl: ControllerHandle) -> Result<Self, String> {
        let probe = tauri::async_runtime::block_on(async {
            tokio::time::timeout(Duration::from_secs(3), GlobalShortcuts::new()).await
        });
        match probe {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => return Err(e.to_string()),
            Err(_) => return Err("the portal did not answer".into()),
        }
        let shared = Arc::new(Shared {
            wanted: Mutex::new(HashMap::new()),
            rebind: Notify::new(),
        });
        tauri::async_runtime::spawn(run(shared.clone(), ctl));
        Ok(Self { shared })
    }

    fn wanted(&self) -> HashMap<HotkeyId, String> {
        self.shared
            .wanted
            .lock()
            .map(|w| w.clone())
            .unwrap_or_default()
    }
}

impl HotkeyBackend for PortalBackend {
    fn register(&self, id: HotkeyId, combo: &str) -> Result<(), String> {
        let changed = match self.shared.wanted.lock() {
            Ok(mut w) => w.insert(id, combo.to_string()).as_deref() != Some(combo),
            Err(_) => false,
        };
        if changed && id != HotkeyId::Cancel {
            self.shared.rebind.notify_one();
        }
        Ok(())
    }

    fn unregister(&self, id: HotkeyId) -> Result<(), String> {
        let had = self
            .shared
            .wanted
            .lock()
            .map(|mut w| w.remove(&id).is_some())
            .unwrap_or(false);
        if had && id != HotkeyId::Cancel {
            self.shared.rebind.notify_one();
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "portal"
    }

    fn warning(&self) -> Option<String> {
        let w = self.wanted();
        w.contains_key(&HotkeyId::Cancel).then(|| {
            "On Wayland the cancel key is not bound globally. Cancel from the overlay or with `localyapper --cancel`.".to_string()
        })
    }
}

async fn run(shared: Arc<Shared>, ctl: ControllerHandle) {
    loop {
        // Coalesce the burst of register() calls at start-up.
        shared.rebind.notified().await;
        tokio::time::sleep(Duration::from_millis(200)).await;
        let wanted: Vec<(HotkeyId, String)> = shared
            .wanted
            .lock()
            .map(|w| {
                w.iter()
                    .filter(|(id, _)| **id != HotkeyId::Cancel)
                    .map(|(id, c)| (*id, c.clone()))
                    .collect()
            })
            .unwrap_or_default();
        if let Err(e) = bind_and_listen(&shared, &ctl, wanted).await {
            log::warn!("hotkey: portal session ended: {e}");
        }
    }
}

/// Bind `wanted` in a new session and forward activations until a rebind
/// is requested.
async fn bind_and_listen(
    shared: &Shared,
    ctl: &ControllerHandle,
    wanted: Vec<(HotkeyId, String)>,
) -> Result<(), String> {
    if wanted.is_empty() {
        return Ok(());
    }
    let proxy = GlobalShortcuts::new().await.map_err(|e| e.to_string())?;
    let session = proxy
        .create_session(Default::default())
        .await
        .map_err(|e| e.to_string())?;
    let shortcuts: Vec<NewShortcut> = wanted
        .iter()
        .map(|(id, combo)| {
            let trigger = to_portal_trigger(combo);
            NewShortcut::new(id.as_str(), format!("LocalYapper: {}", id.label()))
                .preferred_trigger(Some(trigger.as_str()))
        })
        .collect();
    let bound = proxy
        .bind_shortcuts(&session, &shortcuts, None, BindShortcutsOptions::default())
        .await
        .map_err(|e| e.to_string())?
        .response()
        .map_err(|e| e.to_string())?;
    let bound_ids: HashSet<String> = bound
        .shortcuts()
        .iter()
        .map(|s| s.id().to_string())
        .collect();
    log::info!("hotkey: portal bound {bound_ids:?}");

    let mut activated = proxy.receive_activated().await.map_err(|e| e.to_string())?;
    let mut deactivated = proxy
        .receive_deactivated()
        .await
        .map_err(|e| e.to_string())?;
    let active = |sid: &str| -> Option<HotkeyId> {
        let id = HotkeyId::parse(sid)?;
        let wanted = shared.wanted.lock().ok()?;
        wanted.contains_key(&id).then_some(id)
    };
    loop {
        tokio::select! {
            Some(ev) = activated.next() => {
                if let Some(id) = active(ev.shortcut_id()) {
                    dispatch(ctl, id, true, ctl.now_ms());
                }
            }
            Some(ev) = deactivated.next() => {
                if let Some(id) = active(ev.shortcut_id()) {
                    dispatch(ctl, id, false, ctl.now_ms());
                }
            }
            _ = shared.rebind.notified() => {
                // Put the request back for the outer loop, then rebind.
                shared.rebind.notify_one();
                let _ = session.close().await;
                return Ok(());
            }
            else => return Err("portal signal streams closed".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_combos_to_portal_triggers() {
        assert_eq!(to_portal_trigger("Ctrl+Alt+J"), "CTRL+ALT+j");
        assert_eq!(to_portal_trigger("F8"), "F8");
        assert_eq!(to_portal_trigger("Super+Space"), "LOGO+space");
        assert_eq!(to_portal_trigger("Shift+Escape"), "SHIFT+Escape");
        // Keys named after their position get their keysym name.
        assert_eq!(to_portal_trigger("Meta+ArrowUp"), "LOGO+Up");
        assert_eq!(to_portal_trigger("Ctrl+Backquote"), "CTRL+grave");
        assert_eq!(to_portal_trigger("Alt+Enter"), "ALT+Return");
        assert_eq!(to_portal_trigger("Ctrl+Numpad5"), "CTRL+KP_5");
        assert_eq!(to_portal_trigger("Ctrl+Shift+1"), "CTRL+SHIFT+1");
        assert_eq!(to_portal_trigger("F13"), "F13");
    }
}
