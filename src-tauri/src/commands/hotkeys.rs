// IPC command handlers -- hotkey update and reset
use std::collections::HashMap;

use rusqlite::Connection;

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
    ("hotkey_record", "F8"),
    ("hotkey_hands_free", "Ctrl+F8"),
    ("hotkey_cancel", "Escape"),
    ("hotkey_paste_last", "Ctrl+Alt+J"),
    ("hotkey_open_app", "Ctrl+Alt+O"),
];

fn hotkey_label(key: &str) -> &str {
    match key {
        "hotkey_record" => "Record",
        "hotkey_hands_free" => "Hands-free",
        "hotkey_cancel" => "Cancel",
        "hotkey_paste_last" => "Paste Last",
        "hotkey_open_app" => "Open App",
        _ => key,
    }
}

fn default_hotkey_value(key: &str) -> &str {
    HOTKEY_DEFAULTS
        .iter()
        .find_map(|(default_key, default_value)| (*default_key == key).then_some(*default_value))
        .unwrap_or("")
}

fn ensure_hotkey_is_unique(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    for other_key in HOTKEY_KEYS {
        if *other_key == key {
            continue;
        }

        let existing = queries::get_setting(conn, other_key)
            .unwrap_or_else(|_| default_hotkey_value(other_key).to_string());

        if existing == value {
            return Err(format!(
                "{} is already using {}",
                hotkey_label(other_key),
                value
            ));
        }
    }

    Ok(())
}

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

    let conn = state.db.lock().map_err(|e| e.to_string())?;
    ensure_hotkey_is_unique(&conn, &key, &value)?;
    queries::set_setting(&conn, &key, &value).map_err(|e| e.to_string())?;

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
