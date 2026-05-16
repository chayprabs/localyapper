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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema;

    fn test_connection() -> Result<Connection, LocalYapperError> {
        let conn = Connection::open_in_memory()?;
        schema::initialize_database(&conn)?;
        Ok(conn)
    }

    fn history_entry(
        id: &str,
        final_text: &str,
        created_at: &str,
        duration_ms: Option<i64>,
        word_count: Option<i64>,
    ) -> HistoryEntry {
        HistoryEntry {
            id: id.to_string(),
            raw_text: final_text.to_string(),
            final_text: final_text.to_string(),
            app_name: Some("Test App".to_string()),
            duration_ms,
            word_count,
            created_at: created_at.to_string(),
        }
    }

    #[test]
    fn history_queries_return_reverse_chronological_pages() -> Result<(), LocalYapperError> {
        let conn = test_connection()?;
        insert_history(
            &conn,
            &history_entry(
                "one",
                "first entry",
                "2026-05-16 09:00:00",
                Some(1_000),
                Some(2),
            ),
        )?;
        insert_history(
            &conn,
            &history_entry(
                "two",
                "second entry",
                "2026-05-16 10:00:00",
                Some(1_000),
                Some(2),
            ),
        )?;
        insert_history(
            &conn,
            &history_entry(
                "three",
                "third entry",
                "2026-05-16 11:00:00",
                Some(1_000),
                Some(2),
            ),
        )?;

        let first_page = get_history(&conn, 2, 0)?;
        assert_eq!(
            first_page
                .iter()
                .map(|entry| entry.id.as_str())
                .collect::<Vec<_>>(),
            vec!["three", "two"]
        );

        let second_page = get_history(&conn, 2, 2)?;
        assert_eq!(
            second_page
                .iter()
                .map(|entry| entry.id.as_str())
                .collect::<Vec<_>>(),
            vec!["one"]
        );

        Ok(())
    }

    #[test]
    fn stats_aggregate_today_week_all_time_and_wpm() -> Result<(), LocalYapperError> {
        let conn = test_connection()?;
        insert_history(
            &conn,
            &history_entry(
                "today",
                "today words",
                "datetime('now')",
                Some(60_000),
                Some(120),
            ),
        )?;
        conn.execute(
            "UPDATE transcription_history SET created_at = datetime('now') WHERE id = 'today'",
            [],
        )?;

        insert_history(
            &conn,
            &history_entry(
                "week",
                "week words",
                "datetime('now', '-2 days')",
                Some(30_000),
                Some(30),
            ),
        )?;
        conn.execute(
            "UPDATE transcription_history SET created_at = datetime('now', '-2 days') WHERE id = 'week'",
            [],
        )?;

        insert_history(
            &conn,
            &history_entry(
                "old",
                "old words",
                "datetime('now', '-10 days')",
                Some(30_000),
                Some(50),
            ),
        )?;
        conn.execute(
            "UPDATE transcription_history SET created_at = datetime('now', '-10 days') WHERE id = 'old'",
            [],
        )?;

        let stats = get_stats(&conn)?;

        assert_eq!(stats.words_today, 120);
        assert_eq!(stats.words_week, 150);
        assert_eq!(stats.words_all_time, 200);
        assert_eq!(stats.total_sessions, 3);
        assert!((stats.avg_wpm - 100.0).abs() < f64::EPSILON);

        Ok(())
    }

    #[test]
    fn delete_history_entry_reports_missing_ids() -> Result<(), LocalYapperError> {
        let conn = test_connection()?;

        let error = delete_history_entry(&conn, "missing").expect_err("missing id should fail");

        match error {
            LocalYapperError::NotFound(message) => {
                assert!(message.contains("missing"));
            }
            other => panic!("expected NotFound, got {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn clear_history_removes_all_entries() -> Result<(), LocalYapperError> {
        let conn = test_connection()?;
        insert_history(
            &conn,
            &history_entry(
                "one",
                "first entry",
                "2026-05-16 09:00:00",
                Some(1_000),
                Some(2),
            ),
        )?;
        insert_history(
            &conn,
            &history_entry(
                "two",
                "second entry",
                "2026-05-16 10:00:00",
                Some(1_000),
                Some(2),
            ),
        )?;

        clear_history(&conn)?;

        assert!(get_history(&conn, 20, 0)?.is_empty());
        let stats = get_stats(&conn)?;
        assert_eq!(stats.total_sessions, 0);
        assert_eq!(stats.words_all_time, 0);
        assert_eq!(stats.avg_wpm, 0.0);

        Ok(())
    }
}
