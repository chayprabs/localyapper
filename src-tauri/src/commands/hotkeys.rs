use std::collections::HashMap;

use crate::db::queries;
use crate::hotkey::manager;
use crate::state::AppState;

/// Valid hotkey setting keys.
const HOTKEY_KEYS: &[&str] = &[
    "hotkey_record",
    "hotkey_hands_free",
    "hotkey_cancel",
    "hotkey_paste_last",
    "hotkey_open_app",
];

/// Default hotkey values.
const HOTKEY_DEFAULTS: &[(&str, &str)] = &[
    ("hotkey_record", "Alt+Space"),
    ("hotkey_hands_free", "Alt+Alt+Space"),
    ("hotkey_cancel", "Escape"),
    ("hotkey_paste_last", "Alt+Shift+V"),
    ("hotkey_open_app", "Alt+L"),
];

/// Update a single hotkey binding and reload all global shortcuts.
#[tauri::command]
pub async fn update_hotkey(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    key: String,
    value: String,
) -> Result<(), String> {
    if !HOTKEY_KEYS.contains(&key.as_str()) {
        return Err(format!("Invalid hotkey key: {key}"));
    }

    // hands_free is auto-synced, not independently editable
    if key == "hotkey_hands_free" {
        return Err("hotkey_hands_free is auto-synced with hotkey_record".to_string());
    }

    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::set_setting(&conn, &key, &value).map_err(|e| e.to_string())?;

    // Auto-sync hands_free when record changes
    if key == "hotkey_record" {
        queries::set_setting(&conn, "hotkey_hands_free", &value)
            .map_err(|e| e.to_string())?;
    }

    drop(conn);
    manager::reload_hotkeys(&app)
}

/// Reset all hotkeys to platform defaults and reload.
#[tauri::command]
pub async fn reset_hotkeys(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<HashMap<String, String>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;

    for (key, value) in HOTKEY_DEFAULTS {
        queries::set_setting(&conn, key, value).map_err(|e| e.to_string())?;
    }

    drop(conn);
    manager::reload_hotkeys(&app)?;

    let result: HashMap<String, String> = HOTKEY_DEFAULTS
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    Ok(result)
}
