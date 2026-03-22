use rusqlite::{params, Connection};
use std::collections::HashMap;

use crate::error::LocalYapperError;
use crate::models::{Correction, DictionaryWord, HistoryEntry, ImportResult, Mode, NewMode, Stats};

// --- History ---

/// Returns history entries in reverse chronological order.
pub fn get_history(
    conn: &Connection,
    limit: i64,
    offset: i64,
) -> Result<Vec<HistoryEntry>, LocalYapperError> {
    let mut stmt = conn.prepare(
        "SELECT id, raw_text, final_text, app_name, mode_id, duration_ms, word_count, created_at
         FROM transcription_history ORDER BY created_at DESC LIMIT ?1 OFFSET ?2",
    )?;
    let rows = stmt.query_map(params![limit, offset], |row| {
        Ok(HistoryEntry {
            id: row.get(0)?,
            raw_text: row.get(1)?,
            final_text: row.get(2)?,
            app_name: row.get(3)?,
            mode_id: row.get(4)?,
            duration_ms: row.get(5)?,
            word_count: row.get(6)?,
            created_at: row.get(7)?,
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
        "INSERT INTO transcription_history (id, raw_text, final_text, app_name, mode_id, duration_ms, word_count, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            entry.id,
            entry.raw_text,
            entry.final_text,
            entry.app_name,
            entry.mode_id,
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

    let total_sessions: i64 = conn.query_row(
        "SELECT COUNT(*) FROM transcription_history",
        [],
        |row| row.get(0),
    )?;

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

// --- Corrections ---

/// Returns corrections ordered by most recently used.
pub fn get_corrections(
    conn: &Connection,
    limit: i64,
    offset: i64,
) -> Result<Vec<Correction>, LocalYapperError> {
    let mut stmt = conn.prepare(
        "SELECT id, raw_word, corrected, count, confidence, last_used_at, created_at
         FROM corrections ORDER BY created_at DESC LIMIT ?1 OFFSET ?2",
    )?;
    let rows = stmt.query_map(params![limit, offset], |row| {
        Ok(Correction {
            id: row.get(0)?,
            raw_word: row.get(1)?,
            corrected: row.get(2)?,
            count: row.get(3)?,
            confidence: row.get(4)?,
            last_used_at: row.get(5)?,
            created_at: row.get(6)?,
        })
    })?;
    let mut corrections = Vec::new();
    for row in rows {
        corrections.push(row?);
    }
    Ok(corrections)
}

/// Inserts a new correction or increments count if it already exists.
pub fn insert_correction(
    conn: &Connection,
    id: &str,
    raw_word: &str,
    corrected: &str,
) -> Result<Correction, LocalYapperError> {
    conn.execute(
        "INSERT INTO corrections (id, raw_word, corrected, count, confidence, last_used_at, created_at)
         VALUES (?1, ?2, ?3, 1, 0.0, datetime('now'), datetime('now'))
         ON CONFLICT(raw_word, corrected) DO UPDATE SET count = count + 1, last_used_at = datetime('now')",
        params![id, raw_word, corrected],
    )?;

    let correction = conn.query_row(
        "SELECT id, raw_word, corrected, count, confidence, last_used_at, created_at
         FROM corrections WHERE raw_word = ?1 AND corrected = ?2",
        params![raw_word, corrected],
        |row| {
            Ok(Correction {
                id: row.get(0)?,
                raw_word: row.get(1)?,
                corrected: row.get(2)?,
                count: row.get(3)?,
                confidence: row.get(4)?,
                last_used_at: row.get(5)?,
                created_at: row.get(6)?,
            })
        },
    )?;

    Ok(correction)
}

/// Inserts a manually-added correction with count=0 and confidence=1.0 (active immediately).
/// If the pair already exists, updates confidence to 1.0.
pub fn insert_manual_correction(
    conn: &Connection,
    id: &str,
    raw_word: &str,
    corrected: &str,
) -> Result<Correction, LocalYapperError> {
    conn.execute(
        "INSERT INTO corrections (id, raw_word, corrected, count, confidence, last_used_at, created_at)
         VALUES (?1, ?2, ?3, 0, 1.0, datetime('now'), datetime('now'))
         ON CONFLICT(raw_word, corrected) DO UPDATE SET confidence = 1.0, last_used_at = datetime('now')",
        params![id, raw_word, corrected],
    )?;

    let correction = conn.query_row(
        "SELECT id, raw_word, corrected, count, confidence, last_used_at, created_at
         FROM corrections WHERE raw_word = ?1 AND corrected = ?2",
        params![raw_word, corrected],
        |row| {
            Ok(Correction {
                id: row.get(0)?,
                raw_word: row.get(1)?,
                corrected: row.get(2)?,
                count: row.get(3)?,
                confidence: row.get(4)?,
                last_used_at: row.get(5)?,
                created_at: row.get(6)?,
            })
        },
    )?;

    Ok(correction)
}

/// Returns all corrections above a confidence threshold for engine loading.
/// Results are ordered so that for each raw_word, the highest-confidence (then highest-count) row comes first.
pub fn get_all_corrections_for_engine(
    conn: &Connection,
    confidence_threshold: f64,
) -> Result<Vec<(String, String, i64, f64)>, LocalYapperError> {
    let mut stmt = conn.prepare(
        "SELECT raw_word, corrected, count, confidence
         FROM corrections WHERE confidence >= ?1
         ORDER BY raw_word ASC, confidence DESC, count DESC",
    )?;
    let rows = stmt.query_map(params![confidence_threshold], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, f64>(3)?,
        ))
    })?;
    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }
    Ok(results)
}

/// Updates the confidence score for a correction by ID.
pub fn update_correction_confidence(
    conn: &Connection,
    id: &str,
    confidence: f64,
) -> Result<(), LocalYapperError> {
    let affected = conn.execute(
        "UPDATE corrections SET confidence = ?2 WHERE id = ?1",
        params![id, confidence],
    )?;
    if affected == 0 {
        return Err(LocalYapperError::NotFound(format!(
            "Correction not found: {id}"
        )));
    }
    Ok(())
}

/// Deletes a correction by ID.
pub fn delete_correction(conn: &Connection, id: &str) -> Result<(), LocalYapperError> {
    let affected = conn.execute("DELETE FROM corrections WHERE id = ?1", params![id])?;
    if affected == 0 {
        return Err(LocalYapperError::NotFound(format!(
            "Correction not found: {id}"
        )));
    }
    Ok(())
}

/// Exports all corrections as a JSON string.
pub fn export_corrections(conn: &Connection) -> Result<String, LocalYapperError> {
    let corrections = get_corrections(conn, i64::MAX, 0)?;
    let json = serde_json::to_string_pretty(&corrections)?;
    Ok(json)
}

/// Imports corrections from a JSON string.
pub fn import_corrections(conn: &Connection, json: &str) -> Result<ImportResult, LocalYapperError> {
    let corrections: Vec<Correction> = serde_json::from_str(json)?;
    let mut imported = 0i64;
    let mut skipped = 0i64;
    let mut errors = Vec::new();

    let tx = conn.unchecked_transaction()?;
    for c in &corrections {
        match tx.execute(
            "INSERT OR IGNORE INTO corrections (id, raw_word, corrected, count, confidence, last_used_at, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![c.id, c.raw_word, c.corrected, c.count, c.confidence, c.last_used_at, c.created_at],
        ) {
            Ok(0) => skipped += 1,
            Ok(_) => imported += 1,
            Err(e) => errors.push(format!("Error importing '{}': {}", c.raw_word, e)),
        }
    }
    tx.commit()?;

    Ok(ImportResult {
        imported,
        skipped,
        errors,
    })
}

/// Returns the total number of corrections.
pub fn count_corrections(conn: &Connection) -> Result<i64, LocalYapperError> {
    conn.query_row("SELECT COUNT(*) FROM corrections", [], |row| row.get(0))
        .map_err(LocalYapperError::DatabaseError)
}

// --- Dictionary ---

/// Returns all dictionary words.
pub fn get_dictionary(conn: &Connection) -> Result<Vec<DictionaryWord>, LocalYapperError> {
    let mut stmt = conn.prepare(
        "SELECT id, word, count, added_at FROM personal_dictionary ORDER BY added_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(DictionaryWord {
            id: row.get(0)?,
            word: row.get(1)?,
            count: row.get(2)?,
            added_at: row.get(3)?,
        })
    })?;
    let mut words = Vec::new();
    for row in rows {
        words.push(row?);
    }
    Ok(words)
}

/// Adds a word to the personal dictionary.
pub fn insert_word(
    conn: &Connection,
    id: &str,
    word: &str,
) -> Result<DictionaryWord, LocalYapperError> {
    conn.execute(
        "INSERT INTO personal_dictionary (id, word, count, added_at) VALUES (?1, ?2, 1, datetime('now'))
         ON CONFLICT(word) DO UPDATE SET count = count + 1",
        params![id, word],
    )?;

    let entry = conn.query_row(
        "SELECT id, word, count, added_at FROM personal_dictionary WHERE word = ?1",
        params![word],
        |row| {
            Ok(DictionaryWord {
                id: row.get(0)?,
                word: row.get(1)?,
                count: row.get(2)?,
                added_at: row.get(3)?,
            })
        },
    )?;

    Ok(entry)
}

/// Deletes a dictionary word by ID.
pub fn delete_word(conn: &Connection, id: &str) -> Result<(), LocalYapperError> {
    let affected = conn.execute(
        "DELETE FROM personal_dictionary WHERE id = ?1",
        params![id],
    )?;
    if affected == 0 {
        return Err(LocalYapperError::NotFound(format!(
            "Dictionary word not found: {id}"
        )));
    }
    Ok(())
}

// --- Modes ---

/// Returns all modes.
pub fn get_modes(conn: &Connection) -> Result<Vec<Mode>, LocalYapperError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, system_prompt, skip_llm, is_builtin, color, created_at FROM modes ORDER BY is_builtin DESC, name ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Mode {
            id: row.get(0)?,
            name: row.get(1)?,
            system_prompt: row.get(2)?,
            skip_llm: row.get::<_, i32>(3)? != 0,
            is_builtin: row.get::<_, i32>(4)? != 0,
            color: row.get(5)?,
            created_at: row.get(6)?,
        })
    })?;
    let mut modes = Vec::new();
    for row in rows {
        modes.push(row?);
    }
    Ok(modes)
}

/// Returns a single mode by ID.
pub fn get_mode_by_id(conn: &Connection, id: &str) -> Result<Mode, LocalYapperError> {
    conn.query_row(
        "SELECT id, name, system_prompt, skip_llm, is_builtin, color, created_at FROM modes WHERE id = ?1",
        params![id],
        |row| {
            Ok(Mode {
                id: row.get(0)?,
                name: row.get(1)?,
                system_prompt: row.get(2)?,
                skip_llm: row.get::<_, i32>(3)? != 0,
                is_builtin: row.get::<_, i32>(4)? != 0,
                color: row.get(5)?,
                created_at: row.get(6)?,
            })
        },
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            LocalYapperError::NotFound(format!("Mode not found: {id}"))
        }
        other => LocalYapperError::DatabaseError(other),
    })
}

/// Inserts a new user-created mode.
pub fn insert_mode(conn: &Connection, id: &str, mode: &NewMode) -> Result<Mode, LocalYapperError> {
    conn.execute(
        "INSERT INTO modes (id, name, system_prompt, skip_llm, is_builtin, color, created_at)
         VALUES (?1, ?2, ?3, ?4, 0, ?5, datetime('now'))",
        params![id, mode.name, mode.system_prompt, mode.skip_llm as i32, mode.color],
    )?;
    get_mode_by_id(conn, id)
}

/// Updates an existing mode.
pub fn update_mode(conn: &Connection, mode: &Mode) -> Result<(), LocalYapperError> {
    let affected = conn.execute(
        "UPDATE modes SET name = ?2, system_prompt = ?3, skip_llm = ?4, color = ?5 WHERE id = ?1",
        params![mode.id, mode.name, mode.system_prompt, mode.skip_llm as i32, mode.color],
    )?;
    if affected == 0 {
        return Err(LocalYapperError::NotFound(format!(
            "Mode not found: {}",
            mode.id
        )));
    }
    Ok(())
}

/// Deletes a mode by ID. Built-in modes cannot be deleted.
pub fn delete_mode(conn: &Connection, id: &str) -> Result<(), LocalYapperError> {
    let is_builtin: bool = conn
        .query_row(
            "SELECT is_builtin FROM modes WHERE id = ?1",
            params![id],
            |row| row.get::<_, i32>(0),
        )
        .map(|v| v != 0)
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                LocalYapperError::NotFound(format!("Mode not found: {id}"))
            }
            other => LocalYapperError::DatabaseError(other),
        })?;

    if is_builtin {
        return Err(LocalYapperError::InvalidInput(
            "Cannot delete built-in modes".to_string(),
        ));
    }

    conn.execute("DELETE FROM modes WHERE id = ?1", params![id])?;
    Ok(())
}

/// Returns the currently active mode.
pub fn get_active_mode(conn: &Connection) -> Result<Mode, LocalYapperError> {
    let active_id: String = conn.query_row(
        "SELECT value FROM settings WHERE key = 'active_mode_id'",
        [],
        |row| row.get(0),
    )?;
    get_mode_by_id(conn, &active_id)
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
