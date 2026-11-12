//! Global hotkey back ends.
//!
//! Back ends send `RecordDown`, `RecordUp`, cancel and the other actions
//! straight into the controller channel from their callbacks. Nothing is
//! spawned per event, so press and release keep their order.

#[cfg(target_os = "linux")]
pub mod portal;
// Appendix C of docs/REBUILD_PLAN.md, kept byte-for-byte.
#[rustfmt::skip]
pub mod repeat_filter;
pub mod shortcut;

use crate::controller::{ControllerHandle, Msg};
use crate::core::session::Event;
use std::sync::Arc;

/// The hotkey actions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HotkeyId {
    Record,
    Cancel,
    PasteLast,
    OpenApp,
}

impl HotkeyId {
    pub const ALL: [HotkeyId; 4] = [Self::Record, Self::Cancel, Self::PasteLast, Self::OpenApp];

    pub fn setting_key(self) -> &'static str {
        match self {
            Self::Record => "hotkey_record",
            Self::Cancel => "hotkey_cancel",
            Self::PasteLast => "hotkey_paste_last",
            Self::OpenApp => "hotkey_open_app",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Record => "Record",
            Self::Cancel => "Cancel",
            Self::PasteLast => "Paste Last",
            Self::OpenApp => "Open App",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Record => "record",
            Self::Cancel => "cancel",
            Self::PasteLast => "paste_last",
            Self::OpenApp => "open_app",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|id| id.as_str() == s)
    }
}

/// Default combo per action and OS (plan P2.2).
pub fn default_combo(id: HotkeyId) -> &'static str {
    match id {
        HotkeyId::Record => {
            if cfg!(target_os = "macos") {
                "Alt+Space"
            } else {
                "F8"
            }
        }
        HotkeyId::Cancel => "Escape",
        HotkeyId::PasteLast => "Ctrl+Alt+J",
        HotkeyId::OpenApp => "Ctrl+Alt+O",
    }
}

pub trait HotkeyBackend: Send + Sync {
    fn register(&self, id: HotkeyId, combo: &str) -> Result<(), String>;
    fn unregister(&self, id: HotkeyId) -> Result<(), String>;
    /// Short name for diagnostics ("global-shortcut", "portal").
    fn name(&self) -> &'static str;
    /// A problem the UI should show, such as a missing portal.
    fn warning(&self) -> Option<String> {
        None
    }
}

/// Turn a hotkey event into controller messages. Shared by back ends.
pub(crate) fn dispatch(ctl: &ControllerHandle, id: HotkeyId, pressed: bool, at_ms: u64) {
    match (id, pressed) {
        (HotkeyId::Record, true) => ctl.send(Msg::Session(Event::RecordDown { at_ms })),
        (HotkeyId::Record, false) => ctl.send(Msg::Session(Event::RecordUp { at_ms })),
        (HotkeyId::Cancel, true) => ctl.send(Msg::CancelKey),
        (HotkeyId::PasteLast, true) => ctl.send(Msg::PasteLast),
        (HotkeyId::OpenApp, true) => ctl.send(Msg::OpenApp),
        _ => {}
    }
}

/// Pick the back end for this session: the GlobalShortcuts portal on
/// Wayland, the global-shortcut plugin everywhere else (and on Wayland
/// when the portal is missing, which then works only for XWayland apps).
pub fn create_backend(app: &tauri::AppHandle, ctl: &ControllerHandle) -> Arc<dyn HotkeyBackend> {
    #[cfg(target_os = "linux")]
    if crate::platform::inject::is_wayland_session() {
        match portal::PortalBackend::start(ctl.clone()) {
            Ok(b) => return Arc::new(b),
            Err(e) => {
                log::warn!("hotkey: GlobalShortcuts portal unavailable: {e}");
                return Arc::new(shortcut::ShortcutBackend::new(
                    app.clone(),
                    ctl.clone(),
                    Some(format!(
                        "Global shortcuts need the desktop's GlobalShortcuts portal, which is not available ({e}). Bind a desktop shortcut to `localyapper --toggle` instead."
                    )),
                ));
            }
        }
    }
    Arc::new(shortcut::ShortcutBackend::new(
        app.clone(),
        ctl.clone(),
        None,
    ))
}

/// Register record, paste-last and open-app from settings through the
/// app's back end. While paused only open-app stays registered (P2.4).
/// The cancel key belongs to the controller and is not touched here.
pub fn apply_settings(app: &tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    let state = app.state::<crate::state::AppState>();
    let paused = state.paused.load(std::sync::atomic::Ordering::SeqCst);
    let combos: Vec<(HotkeyId, String)> =
        [HotkeyId::Record, HotkeyId::PasteLast, HotkeyId::OpenApp]
            .into_iter()
            .map(|id| {
                let combo = state
                    .settings
                    .get(id.setting_key())
                    .filter(|c| !c.trim().is_empty())
                    .unwrap_or_else(|| default_combo(id).to_string());
                (id, combo)
            })
            .collect();
    let mut errors = Vec::new();
    for (id, combo) in combos {
        let r = if paused && id != HotkeyId::OpenApp {
            state.hotkeys.unregister(id)
        } else {
            state.hotkeys.register(id, &combo)
        };
        if let Err(e) = r {
            errors.push(e);
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip_and_defaults_exist() {
        for id in HotkeyId::ALL {
            assert_eq!(HotkeyId::parse(id.as_str()), Some(id));
            assert!(!default_combo(id).is_empty());
        }
        let rec = default_combo(HotkeyId::Record);
        if cfg!(target_os = "macos") {
            assert_eq!(rec, "Alt+Space");
        } else {
            assert_eq!(rec, "F8");
        }
    }
}
