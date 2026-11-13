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
    search: Option<String>,
) -> Result<Vec<HistoryEntry>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    queries::get_history(&conn, limit, offset, search.as_deref()).map_err(|e| e.to_string())
}

/// Plain-text export: one block per dictation, newest first, with the
/// local time and app.
fn export_text(entries: &[HistoryEntry]) -> String {
    let mut out = String::new();
    for e in entries {
        let when = chrono::DateTime::parse_from_rfc3339(&e.created_at)
            .map(|d| {
                d.with_timezone(&chrono::Local)
                    .format("%Y-%m-%d %H:%M")
                    .to_string()
            })
            .unwrap_or_else(|_| e.created_at.clone());
        out.push_str(&when);
        if let Some(app) = e.app_name.as_deref().filter(|a| !a.is_empty()) {
            out.push_str("  ");
            out.push_str(app);
        }
        out.push('\n');
        out.push_str(&e.final_text);
        out.push_str("\n\n");
    }
    out
}

/// Ask where to save, then write all history there as text. Returns the
/// path, or `None` when the user cancelled.
#[tauri::command]
pub async fn export_history(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let entries = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        queries::get_history(&conn, i64::MAX, 0, None).map_err(|e| e.to_string())?
    };
    let name = format!(
        "localyapper-history-{}.txt",
        chrono::Local::now().format("%Y-%m-%d")
    );
    let picked = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_file_name(name)
            .add_filter("Text", &["txt"])
            .blocking_save_file()
    })
    .await
    .map_err(|e| e.to_string())?;
    let Some(path) = picked.and_then(|p| p.into_path().ok()) else {
        return Ok(None);
    };
    std::fs::write(&path, export_text(&entries))
        .map_err(|e| format!("Could not write {}: {e}", path.display()))?;
    Ok(Some(path.display().to_string()))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_lists_newest_first_with_app_and_text() {
        let e = |id: &str, text: &str, at: &str, app: Option<&str>| HistoryEntry {
            id: id.into(),
            raw_text: text.into(),
            final_text: text.into(),
            app_name: app.map(str::to_string),
            duration_ms: None,
            word_count: None,
            created_at: at.into(),
        };
        let out = export_text(&[
            e("2", "second", "2026-05-16T10:00:00.000Z", Some("Notes")),
            e("1", "first", "2026-05-16 09:00:00", None),
        ]);
        let blocks: Vec<&str> = out.trim_end().split("\n\n").collect();
        assert_eq!(blocks.len(), 2);
        assert!(blocks[0].ends_with("Notes\nsecond"), "{out}");
        // A pre-ISO timestamp is kept as written.
        assert_eq!(blocks[1], "2026-05-16 09:00:00\nfirst");
    }

    #[test]
    fn local_boundaries_are_utc_iso_and_ordered() {
        let (day, week) = local_boundaries();
        assert!(day.ends_with('Z') && week.ends_with('Z'));
        assert!(week <= day);
    }
}
