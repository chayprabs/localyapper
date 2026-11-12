//! `tauri-plugin-global-shortcut` back end (Windows, macOS, Linux X11).
//!
//! Callbacks send straight into the controller channel. Nothing is
//! spawned per event, so press and release keep their order.

use crate::controller::{ControllerHandle, Msg};
use crate::core::session::Event;
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

/// The hotkey actions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HotkeyId {
    Record,
    Cancel,
    PasteLast,
    OpenApp,
}

impl HotkeyId {
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
}

/// Register `combo` for `id`, replacing nothing: the caller unregisters
/// the previous combo first.
pub fn register(
    app: &AppHandle,
    id: HotkeyId,
    combo: &str,
    ctl: &ControllerHandle,
) -> Result<(), String> {
    let ctl = ctl.clone();
    app.global_shortcut()
        .on_shortcut(combo, move |app, _shortcut, event| {
            let pressed = event.state == ShortcutState::Pressed;
            match id {
                HotkeyId::Record => {
                    let at_ms = ctl.now_ms();
                    ctl.send(Msg::Session(if pressed {
                        Event::RecordDown { at_ms }
                    } else {
                        Event::RecordUp { at_ms }
                    }));
                }
                HotkeyId::Cancel if pressed => ctl.send(Msg::Session(Event::Cancel)),
                HotkeyId::PasteLast if pressed => ctl.send(Msg::PasteLast),
                HotkeyId::OpenApp if pressed => crate::show_or_create_main_window(app),
                _ => {}
            }
        })
        .map_err(|e| format!("Failed to register {} hotkey '{combo}': {e}", id.label()))
}

pub fn unregister(app: &AppHandle, combo: &str) {
    if combo.is_empty() {
        return;
    }
    if let Err(e) = app.global_shortcut().unregister(combo) {
        log::debug!("hotkey: unregister '{combo}': {e}");
    }
}

/// Re-register record, paste-last and open-app from settings. While
/// paused only open-app stays registered. The cancel key is owned by the
/// controller and untouched here.
pub fn reload(app: &AppHandle) -> Result<(), String> {
    use tauri::Manager;
    let state = app.state::<crate::state::AppState>();
    let paused = state.paused.load(std::sync::atomic::Ordering::SeqCst);
    let wanted: Vec<(HotkeyId, String)> = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        [HotkeyId::Record, HotkeyId::PasteLast, HotkeyId::OpenApp]
            .into_iter()
            .filter(|id| !paused || *id == HotkeyId::OpenApp)
            .filter_map(|id| {
                crate::db::queries::get_setting(&conn, id.setting_key())
                    .ok()
                    .filter(|c| !c.trim().is_empty())
                    .map(|c| (id, c))
            })
            .collect()
    };
    let mut registered = state.hotkeys.lock().map_err(|e| e.to_string())?;
    for (_, combo) in registered.drain() {
        unregister(app, &combo);
    }
    let mut errors = Vec::new();
    for (id, combo) in wanted {
        match register(app, id, &combo, &state.controller) {
            Ok(()) => {
                registered.insert(id, combo);
            }
            Err(e) => errors.push(e),
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}
