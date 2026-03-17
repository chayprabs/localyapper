use crate::db::queries;
use crate::models::{Mode, NewMode};
use crate::state::AppState;

/// Returns all modes.
#[tauri::command]
pub async fn get_modes(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Mode>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::get_modes(&conn).map_err(|e| e.to_string())
}

/// Creates a new user mode.
#[tauri::command]
pub async fn create_mode(
    state: tauri::State<'_, AppState>,
    mode: NewMode,
) -> Result<Mode, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let id = uuid::Uuid::new_v4().to_string();
    queries::insert_mode(&conn, &id, &mode).map_err(|e| e.to_string())
}

/// Updates an existing mode.
#[tauri::command]
pub async fn update_mode(
    state: tauri::State<'_, AppState>,
    mode: Mode,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::update_mode(&conn, &mode).map_err(|e| e.to_string())
}

/// Deletes a user mode by ID.
#[tauri::command]
pub async fn delete_mode(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::delete_mode(&conn, &id).map_err(|e| e.to_string())
}

/// Sets the active mode.
#[tauri::command]
pub async fn set_active_mode(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    // Verify the mode exists
    queries::get_mode_by_id(&conn, &id).map_err(|e| e.to_string())?;
    queries::set_setting(&conn, "active_mode_id", &id).map_err(|e| e.to_string())
}

/// Returns the currently active mode.
#[tauri::command]
pub async fn get_active_mode(
    state: tauri::State<'_, AppState>,
) -> Result<Mode, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::get_active_mode(&conn).map_err(|e| e.to_string())
}
