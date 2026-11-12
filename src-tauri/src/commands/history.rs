// IPC command handlers -- transcription history and statistics
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
pub async fn clear_history(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::clear_history(&conn).map_err(|e| e.to_string())
}

/// Local midnight today and at the start of this week (Monday), as UTC
/// RFC 3339 timestamps.
pub(crate) fn local_boundaries() -> (String, String) {
    use chrono::{Datelike, Duration, Local, TimeZone, Utc};
    let now = Local::now();
    let midnight = now.date_naive().and_hms_opt(0, 0, 0).unwrap_or_default();
    let day = Local
        .from_local_datetime(&midnight)
        .earliest()
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);
    let week = day - Duration::days(i64::from(now.weekday().num_days_from_monday()));
    let fmt = |d: chrono::DateTime<Utc>| d.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    (fmt(day), fmt(week))
}

/// Returns dashboard statistics using the local day and week.
#[tauri::command]
pub async fn get_stats(state: tauri::State<'_, AppState>) -> Result<Stats, String> {
    let (day, week) = local_boundaries();
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::get_stats(&conn, &day, &week).map_err(|e| e.to_string())
}
