// Database queries -- typed CRUD operations for the active app tables
use rusqlite::{params, Connection};
use std::collections::HashMap;

use crate::error::LocalYapperError;
use crate::models::{HistoryEntry, Stats};

// --- History ---

/// Returns history entries in reverse chronological order, optionally
/// only those whose text or app name contains `search` (any case).
pub fn get_history(
    conn: &Connection,
    limit: i64,
    offset: i64,
    search: Option<&str>,
) -> Result<Vec<HistoryEntry>, LocalYapperError> {
    let pattern = search.map(str::trim).filter(|s| !s.is_empty()).map(|s| {
        let escaped = s
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        format!("%{escaped}%")
    });
    let mut stmt = conn.prepare(
        "SELECT id, raw_text, final_text, app_name, duration_ms, word_count, created_at
         FROM transcription_history
         WHERE ?3 IS NULL
            OR final_text LIKE ?3 ESCAPE '\\'
            OR app_name LIKE ?3 ESCAPE '\\'
         ORDER BY julianday(created_at) DESC LIMIT ?1 OFFSET ?2",
    )?;
    let rows = stmt.query_map(params![limit, offset, pattern], |row| {
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

/// Deletes history older than `days`. Returns how many rows went.
pub fn prune_history(conn: &Connection, days: i64) -> Result<usize, LocalYapperError> {
    Ok(conn.execute(
        "DELETE FROM transcription_history
         WHERE julianday(created_at) < julianday('now', ?1)",
        params![format!("-{days} days")],
    )?)
}

/// Returns dashboard statistics. `day_start` and `week_start` are the
/// local day and week boundaries as UTC timestamps (plan P5.9), so
/// "today" follows the user's clock, not UTC.
pub fn get_stats(
    conn: &Connection,
    day_start: &str,
    week_start: &str,
) -> Result<Stats, LocalYapperError> {
    let words_today: i64 = conn.query_row(
        "SELECT COALESCE(SUM(word_count), 0) FROM transcription_history
         WHERE julianday(created_at) >= julianday(?1)",
        params![day_start],
        |row| row.get(0),
    )?;

    let words_week: i64 = conn.query_row(
        "SELECT COALESCE(SUM(word_count), 0) FROM transcription_history
         WHERE julianday(created_at) >= julianday(?1)",
        params![week_start],
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
#[cfg(test)]
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

        let first_page = get_history(&conn, 2, 0, None)?;
        assert_eq!(
            first_page
                .iter()
                .map(|entry| entry.id.as_str())
                .collect::<Vec<_>>(),
            vec!["three", "two"]
        );

        let second_page = get_history(&conn, 2, 2, None)?;
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

        let (day, week) = bounds(1, 3);
        let stats = get_stats(&conn, &day, &week)?;

        assert_eq!(stats.words_today, 120);
        assert_eq!(stats.words_week, 150);
        assert_eq!(stats.words_all_time, 200);
        assert_eq!(stats.total_sessions, 3);
        assert!((stats.avg_wpm - 100.0).abs() < f64::EPSILON);

        Ok(())
    }

    /// Boundaries `hours` and `days` before now, as RFC 3339 UTC.
    fn bounds(hours: i64, days: i64) -> (String, String) {
        let now = chrono::Utc::now();
        let f =
            |d: chrono::DateTime<chrono::Utc>| d.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        (
            f(now - chrono::Duration::hours(hours)),
            f(now - chrono::Duration::days(days)),
        )
    }

    #[test]
    fn old_and_iso_timestamps_sort_count_and_prune_together() -> Result<(), LocalYapperError> {
        let conn = test_connection()?;
        let now = chrono::Utc::now();
        let iso = |d: chrono::DateTime<chrono::Utc>| {
            d.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
        };
        let old_style = (now - chrono::Duration::days(20))
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
        insert_history(
            &conn,
            &history_entry("new", "a b", &iso(now), Some(1_000), Some(2)),
        )?;
        insert_history(
            &conn,
            &history_entry(
                "mid",
                "c",
                &iso(now - chrono::Duration::days(3)),
                Some(1_000),
                Some(1),
            ),
        )?;
        insert_history(
            &conn,
            &history_entry("old", "d", &old_style, Some(1_000), Some(1)),
        )?;
        let ids: Vec<String> = get_history(&conn, 10, 0, None)?
            .into_iter()
            .map(|e| e.id)
            .collect();
        assert_eq!(ids, vec!["new", "mid", "old"]);
        let (day, week) = bounds(1, 7);
        let stats = get_stats(&conn, &day, &week)?;
        assert_eq!(
            (stats.words_today, stats.words_week, stats.words_all_time),
            (2, 3, 4)
        );
        assert_eq!(prune_history(&conn, 10)?, 1);
        assert_eq!(prune_history(&conn, 1)?, 1);
        let left: Vec<String> = get_history(&conn, 10, 0, None)?
            .into_iter()
            .map(|e| e.id)
            .collect();
        assert_eq!(left, vec!["new"]);
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

        assert!(get_history(&conn, 20, 0, None)?.is_empty());
        let (day, week) = bounds(1, 3);
        let stats = get_stats(&conn, &day, &week)?;
        assert_eq!(stats.total_sessions, 0);
        assert_eq!(stats.words_all_time, 0);
        assert_eq!(stats.avg_wpm, 0.0);

        Ok(())
    }

    #[test]
    fn history_search_matches_text_and_app_and_escapes_wildcards() -> Result<(), LocalYapperError> {
        let conn = test_connection()?;
        for (id, text) in [
            ("a", "Ship the release"),
            ("b", "100% done"),
            ("c", "snake_case name"),
        ] {
            insert_history(
                &conn,
                &history_entry(id, text, "2026-05-16T10:00:00.000Z", None, None),
            )?;
        }
        let ids = |s: &str| -> Result<Vec<String>, LocalYapperError> {
            Ok(get_history(&conn, 10, 0, Some(s))?
                .into_iter()
                .map(|e| e.id)
                .collect())
        };
        assert_eq!(ids("RELEASE")?, vec!["a"]);
        assert_eq!(ids("%")?, vec!["b"]);
        assert_eq!(ids("_")?, vec!["c"]);
        assert_eq!(ids("test app")?.len(), 3);
        assert_eq!(ids("   ")?.len(), 3);
        assert!(ids("absent")?.is_empty());
        Ok(())
    }
}
