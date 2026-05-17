// IPC command handlers -- hotkey update and reset
use std::collections::HashMap;

use rusqlite::Connection;

use crate::db::queries;
use crate::hotkey::manager;
use crate::state::AppState;

/// Valid hotkey setting keys. Hands-free is no longer a separate hotkey -- it
/// is engaged by double-tapping the record hotkey.
const HOTKEY_KEYS: &[&str] = &[
    "hotkey_record",
    "hotkey_cancel",
    "hotkey_paste_last",
    "hotkey_open_app",
];

/// Default hotkey values.
const HOTKEY_DEFAULTS: &[(&str, &str)] = &[
    ("hotkey_record", "F8"),
    ("hotkey_cancel", "Escape"),
    ("hotkey_paste_last", "Ctrl+Alt+J"),
    ("hotkey_open_app", "Ctrl+Alt+O"),
];

fn hotkey_label(key: &str) -> &str {
    match key {
        "hotkey_record" => "Record",
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

fn current_hotkey_settings(conn: &Connection) -> HashMap<String, String> {
    HOTKEY_DEFAULTS
        .iter()
        .map(|(key, default_value)| {
            let value =
                queries::get_setting(conn, key).unwrap_or_else(|_| (*default_value).to_string());
            ((*key).to_string(), value)
        })
        .collect()
}

fn restore_hotkey_settings(conn: &Connection, settings: &HashMap<String, String>) {
    for (key, value) in settings {
        if let Err(e) = queries::set_setting(conn, key, value) {
            log::error!("Failed to restore hotkey setting '{key}': {e}");
        }
    }
}

fn ensure_hotkey_is_unique(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    for other_key in HOTKEY_KEYS {
        if *other_key == key {
            continue;
        }

        let existing = queries::get_setting(conn, other_key)
            .unwrap_or_else(|_| default_hotkey_value(other_key).to_string());

        if existing.eq_ignore_ascii_case(value) {
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
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err("Hotkey cannot be empty".to_string());
    }

    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let previous = current_hotkey_settings(&conn);
    ensure_hotkey_is_unique(&conn, &key, &value)?;
    queries::set_setting(&conn, &key, &value).map_err(|e| e.to_string())?;

    drop(conn);
    if let Err(register_error) = manager::reload_hotkeys(&app) {
        if let Ok(conn) = state.db.lock() {
            restore_hotkey_settings(&conn, &previous);
        }
        if let Err(rollback_error) = manager::reload_hotkeys(&app) {
            log::error!(
                "Failed to restore previous hotkeys after registration error: {rollback_error}"
            );
        }
        return Err(register_error);
    }

    Ok(())
}

/// Reset all hotkeys to platform defaults and reload.
#[tauri::command]
pub async fn reset_hotkeys(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<HashMap<String, String>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let previous = current_hotkey_settings(&conn);

    for (key, value) in HOTKEY_DEFAULTS {
        queries::set_setting(&conn, key, value).map_err(|e| e.to_string())?;
    }

    drop(conn);
    if let Err(register_error) = manager::reload_hotkeys(&app) {
        if let Ok(conn) = state.db.lock() {
            restore_hotkey_settings(&conn, &previous);
        }
        if let Err(rollback_error) = manager::reload_hotkeys(&app) {
            log::error!(
                "Failed to restore previous hotkeys after reset registration error: {rollback_error}"
            );
        }
        return Err(register_error);
    }

    let result: HashMap<String, String> = HOTKEY_DEFAULTS
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema;

    fn test_connection() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory db");
        schema::initialize_database(&conn).expect("schema initialization");
        conn
    }

    #[test]
    fn hotkey_uniqueness_is_case_insensitive() {
        let conn = test_connection();

        let result = ensure_hotkey_is_unique(&conn, "hotkey_record", "ctrl+alt+j");

        assert!(result.is_err());
        assert_eq!(
            result.err().as_deref(),
            Some("Paste Last is already using ctrl+alt+j")
        );
    }

    #[test]
    fn current_hotkey_settings_reads_seeded_defaults() {
        let conn = test_connection();

        let settings = current_hotkey_settings(&conn);

        assert_eq!(
            settings.get("hotkey_record").map(String::as_str),
            Some("F8")
        );
        assert_eq!(
            settings.get("hotkey_cancel").map(String::as_str),
            Some("Escape")
        );
        assert_eq!(
            settings.get("hotkey_paste_last").map(String::as_str),
            Some("Ctrl+Alt+J")
        );
        assert!(!settings.contains_key("hotkey_hands_free"));
    }

    #[test]
    fn restore_hotkey_settings_writes_previous_values() {
        let conn = test_connection();
        let previous = current_hotkey_settings(&conn);
        queries::set_setting(&conn, "hotkey_record", "F9").expect("set hotkey");

        restore_hotkey_settings(&conn, &previous);

        assert_eq!(
            queries::get_setting(&conn, "hotkey_record").expect("hotkey_record"),
            "F8"
        );
    }
}
