// IPC command handlers -- typed settings: read, validate, store, apply
use std::collections::HashMap;

use tauri::{AppHandle, Emitter, Manager};

use crate::db::queries;
use crate::settings::{self, keys, Effects, Store};
use crate::state::AppState;

/// The settings table behind the cache.
struct DbStore<'a> {
    state: &'a AppState,
}

impl Store for DbStore<'_> {
    fn read(&self, key: &str) -> Option<String> {
        self.state.settings.get(key)
    }

    fn write(&mut self, key: &str, value: &str) -> Result<(), String> {
        let conn = self.state.db.lock().map_err(|e| e.to_string())?;
        queries::set_setting(&conn, key, value).map_err(|e| e.to_string())?;
        self.state.settings.set(key, value);
        Ok(())
    }
}

/// What a settings change does in the running app.
pub(crate) struct AppEffects<'a> {
    pub app: &'a AppHandle,
}

impl Effects for AppEffects<'_> {
    fn reload_controller(&self) {
        self.app
            .state::<AppState>()
            .controller
            .send(crate::controller::Msg::ReloadSettings);
    }

    fn reload_hotkeys(&self) -> Result<(), String> {
        crate::platform::hotkey::apply_settings(self.app)?;
        // The cancel key is registered only while recording; check now that
        // the combo can be registered at all.
        let state = self.app.state::<AppState>();
        let cancel = state.settings.get_or_default("hotkey_cancel");
        state
            .hotkeys
            .register(crate::platform::hotkey::HotkeyId::Cancel, &cancel)?;
        state
            .hotkeys
            .unregister(crate::platform::hotkey::HotkeyId::Cancel)
    }

    fn set_autostart(&self, enabled: bool) -> Result<(), String> {
        use tauri_plugin_autostart::ManagerExt;
        let launcher = self.app.autolaunch();
        let r = if enabled {
            launcher.enable()
        } else {
            launcher.disable()
        };
        r.map_err(|e| format!("Could not change launch at login: {e}"))
    }

    fn theme_changed(&self, theme: &str) {
        let _ = self.app.emit("theme-changed", theme);
    }

    fn speech_changed(&self) {
        let models = self.app.state::<AppState>().models.clone();
        tauri::async_runtime::spawn_blocking(move || {
            models.evict_blocking();
        });
        let _ = self.app.emit("models-changed", ());
    }

    fn prune_history(&self, days: i64) {
        prune_history(self.app, days);
    }

    fn changed(&self, key: &str, value: &str) {
        #[derive(serde::Serialize, Clone)]
        struct Change<'a> {
            key: &'a str,
            value: &'a str,
        }
        let _ = self.app.emit("settings-changed", Change { key, value });
    }
}

/// Delete history older than `days` (0 keeps everything).
pub(crate) fn prune_history(app: &AppHandle, days: i64) {
    if days <= 0 {
        return;
    }
    let state = app.state::<AppState>();
    let result = state
        .db
        .lock()
        .map_err(|e| e.to_string())
        .and_then(|c| queries::prune_history(&c, days).map_err(|e| e.to_string()));
    match result {
        Ok(n) if n > 0 => {
            log::info!("history: pruned {n} entries older than {days} days");
            let _ = app.emit("history-updated", ());
        }
        Ok(_) => {}
        Err(e) => log::warn!("history: prune failed: {e}"),
    }
}

fn model_ids() -> Vec<String> {
    crate::commands::models::manifest()
        .map(|m| m.models.keys().cloned().collect())
        .unwrap_or_default()
}

/// Validate, store and apply a setting through `settings::apply`.
pub(crate) fn set_setting_now(app: &AppHandle, key: &str, value: &str) -> Result<String, String> {
    let state = app.state::<AppState>();
    let mut store = DbStore {
        state: state.inner(),
    };
    settings::apply(&mut store, &AppEffects { app }, key, value, &model_ids())
}

/// Gets a single setting value by key (its default when unset).
#[tauri::command]
pub async fn get_setting(state: tauri::State<'_, AppState>, key: String) -> Result<String, String> {
    if keys::find(&key).is_none() {
        return Err(format!("Unknown setting {key:?}"));
    }
    Ok(state.settings.get_or_default(&key))
}

/// Sets a setting: unknown keys and invalid values are rejected, and the
/// change takes effect at once.
#[tauri::command]
pub async fn set_setting(app: AppHandle, key: String, value: String) -> Result<(), String> {
    set_setting_now(&app, &key, &value).map(|_| ())
}

/// Returns every known setting, with defaults filled in.
#[tauri::command]
pub async fn get_all_settings(
    state: tauri::State<'_, AppState>,
) -> Result<HashMap<String, String>, String> {
    let stored = state.settings.snapshot();
    Ok(keys::KEYS
        .iter()
        .map(|k| {
            let v = stored
                .get(k.name)
                .cloned()
                .unwrap_or_else(|| keys::default_of(k));
            (k.name.to_string(), v)
        })
        .collect())
}

/// Put every user setting back to its default (wizard state is kept).
#[tauri::command]
pub async fn reset_all_settings(app: AppHandle) -> Result<(), String> {
    let mut errors = Vec::new();
    for key in keys::KEYS.iter().filter(|k| k.hook != keys::Hook::Store) {
        if let Err(e) = set_setting_now(&app, key.name, &keys::default_of(key)) {
            errors.push(format!("{}: {e}", key.name));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}
