// IPC command handlers -- user settings read and write
use std::collections::HashMap;

use crate::db::queries;
use crate::state::AppState;

/// Gets a single setting value by key.
#[tauri::command]
pub async fn get_setting(
    state: tauri::State<'_, AppState>,
    key: String,
) -> Result<String, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::get_setting(&conn, &key).map_err(|e| e.to_string())
}

/// Sets a setting value.
#[tauri::command]
pub async fn set_setting(
    state: tauri::State<'_, AppState>,
    key: String,
    value: String,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::set_setting(&conn, &key, &value).map_err(|e| e.to_string())
}

/// Returns all settings as key-value pairs.
#[tauri::command]
pub async fn get_all_settings(
    state: tauri::State<'_, AppState>,
) -> Result<HashMap<String, String>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::get_all_settings(&conn).map_err(|e| e.to_string())
}
