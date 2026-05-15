// Database queries -- typed CRUD operations for the active app tables
use rusqlite::{params, Connection};
use std::collections::HashMap;

use crate::error::LocalYapperError;
use crate::models::{HistoryEntry, Stats};

// --- History ---

/// Returns history entries in reverse chronological order.
pub fn get_history(
    conn: &Connection,
    limit: i64,
    offset: i64,
) -> Result<Vec<HistoryEntry>, LocalYapperError> {
    let mut stmt = conn.prepare(
        "SELECT id, raw_text, final_text, app_name, duration_ms, word_count, created_at
         FROM transcription_history ORDER BY created_at DESC LIMIT ?1 OFFSET ?2",
    )?;
    let rows = stmt.query_map(params![limit, offset], |row| {
        Ok(HistoryEntry {
            id: row.get(0)?,
            raw_text: row.get(1)?,
            final_text: row.get(2)?,
            app_name: row.get(3)?,
            duration_ms: row.get(4)?,
            word_count: row.get(5)?,
            created_at: row.get(6)?,
        })
    })?;
    let mut entries = Vec::new();
    for row in rows {
        entries.push(row?);
    }
    Ok(entries)
}

/// Inserts a new history entry.
pub fn insert_history(conn: &Connection, entry: &HistoryEntry) -> Result<(), LocalYapperError> {
    conn.execute(
        "INSERT INTO transcription_history (id, raw_text, final_text, app_name, duration_ms, word_count, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            entry.id,
            entry.raw_text,
            entry.final_text,
            entry.app_name,
            entry.duration_ms,
            entry.word_count,
            entry.created_at,
        ],
    )?;
    Ok(())
}

/// Deletes a single history entry by ID.
pub fn delete_history_entry(conn: &Connection, id: &str) -> Result<(), LocalYapperError> {
    let affected = conn.execute(
        "DELETE FROM transcription_history WHERE id = ?1",
        params![id],
    )?;
    if affected == 0 {
        return Err(LocalYapperError::NotFound(format!(
            "History entry not found: {id}"
        )));
    }
    Ok(())
}

/// Deletes all history entries.
pub fn clear_history(conn: &Connection) -> Result<(), LocalYapperError> {
    conn.execute("DELETE FROM transcription_history", [])?;
    Ok(())
}

/// Returns dashboard statistics computed from history.
pub fn get_stats(conn: &Connection) -> Result<Stats, LocalYapperError> {
    let words_today: i64 = conn.query_row(
        "SELECT COALESCE(SUM(word_count), 0) FROM transcription_history WHERE date(created_at) = date('now')",
        [],
        |row| row.get(0),
    )?;

    let words_week: i64 = conn.query_row(
        "SELECT COALESCE(SUM(word_count), 0) FROM transcription_history WHERE created_at >= datetime('now', '-7 days')",
        [],
        |row| row.get(0),
    )?;

    let words_all_time: i64 = conn.query_row(
        "SELECT COALESCE(SUM(word_count), 0) FROM transcription_history",
        [],
        |row| row.get(0),
    )?;

    let total_sessions: i64 =
        conn.query_row("SELECT COUNT(*) FROM transcription_history", [], |row| {
            row.get(0)
        })?;

    let total_duration_ms: i64 = conn.query_row(
        "SELECT COALESCE(SUM(duration_ms), 0) FROM transcription_history",
        [],
        |row| row.get(0),
    )?;

    let avg_wpm = if total_duration_ms > 0 {
        let total_minutes = total_duration_ms as f64 / 60_000.0;
        words_all_time as f64 / total_minutes
    } else {
        0.0
    };

    Ok(Stats {
        words_today,
        words_week,
        words_all_time,
        avg_wpm,
        total_sessions,
    })
}

// --- Settings ---

/// Gets a single setting value by key.
pub fn get_setting(conn: &Connection, key: &str) -> Result<String, LocalYapperError> {
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        params![key],
        |row| row.get(0),
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            LocalYapperError::NotFound(format!("Setting not found: {key}"))
        }
        other => LocalYapperError::DatabaseError(other),
    })
}

/// Sets a setting value (upsert).
pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<(), LocalYapperError> {
    conn.execute(
        "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, datetime('now'))
         ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at = datetime('now')",
        params![key, value],
    )?;
    Ok(())
}

/// Returns all settings as a HashMap.
pub fn get_all_settings(conn: &Connection) -> Result<HashMap<String, String>, LocalYapperError> {
    let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut map = HashMap::new();
    for row in rows {
        let (k, v) = row?;
        map.insert(k, v);
    }
    Ok(map)
}
