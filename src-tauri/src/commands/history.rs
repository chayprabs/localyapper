use crate::db::queries;
use crate::models::{HistoryEntry, Stats};
use crate::state::AppState;

/// Returns history entries with pagination.
#[tauri::command]
pub async fn get_history(
    state: tauri::State<'_, AppState>,
    limit: i64,
    offset: i64,
) -> Result<Vec<HistoryEntry>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::get_history(&conn, limit, offset).map_err(|e| e.to_string())
}

/// Deletes a single history entry.
#[tauri::command]
pub async fn delete_history_entry(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::delete_history_entry(&conn, &id).map_err(|e| e.to_string())
}

/// Clears all history entries.
#[tauri::command]
pub async fn clear_history(
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::clear_history(&conn).map_err(|e| e.to_string())
}

/// Returns dashboard statistics.
#[tauri::command]
pub async fn get_stats(
    state: tauri::State<'_, AppState>,
) -> Result<Stats, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::get_stats(&conn).map_err(|e| e.to_string())
}
