use crate::db::queries;
use crate::models::{Correction, ImportResult};
use crate::state::AppState;
use rusqlite::Connection;

/// Read the confidence_threshold setting, defaulting to 0.6.
fn get_threshold(conn: &Connection) -> f64 {
    queries::get_setting(conn, "confidence_threshold")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.6)
}

/// Returns corrections with pagination.
#[tauri::command]
pub async fn get_corrections(
    state: tauri::State<'_, AppState>,
    limit: i64,
    offset: i64,
) -> Result<Vec<Correction>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::get_corrections(&conn, limit, offset).map_err(|e| e.to_string())
}

/// Adds a new correction mapping.
#[tauri::command]
pub async fn add_correction(
    state: tauri::State<'_, AppState>,
    raw_word: String,
    corrected: String,
) -> Result<Correction, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let id = uuid::Uuid::new_v4().to_string();
    let result = queries::insert_correction(&conn, &id, &raw_word, &corrected)
        .map_err(|e| e.to_string())?;
    let threshold = get_threshold(&conn);
    state.correction_engine.refresh(&conn, threshold)
        .map_err(|e| e.to_string())?;
    Ok(result)
}

/// Deletes a correction by ID.
#[tauri::command]
pub async fn delete_correction(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::delete_correction(&conn, &id).map_err(|e| e.to_string())?;
    let threshold = get_threshold(&conn);
    state.correction_engine.refresh(&conn, threshold)
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Exports all corrections as a JSON string.
#[tauri::command]
pub async fn export_dictionary(
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::export_corrections(&conn).map_err(|e| e.to_string())
}

/// Imports corrections from a JSON string.
#[tauri::command]
pub async fn import_dictionary(
    state: tauri::State<'_, AppState>,
    json: String,
) -> Result<ImportResult, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let result = queries::import_corrections(&conn, &json)
        .map_err(|e| e.to_string())?;
    let threshold = get_threshold(&conn);
    state.correction_engine.refresh(&conn, threshold)
        .map_err(|e| e.to_string())?;
    Ok(result)
}
