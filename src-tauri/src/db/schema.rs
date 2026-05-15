// Database schema -- table creation, migrations, and seed data
use rusqlite::Connection;

use crate::error::LocalYapperError;

/// Creates the application tables and seeds default data in a single transaction.
pub fn initialize_database(conn: &Connection) -> Result<(), LocalYapperError> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS transcription_history (
            id           TEXT PRIMARY KEY,
            raw_text     TEXT NOT NULL,
            final_text   TEXT NOT NULL,
            app_name     TEXT,
            duration_ms  INTEGER,
            word_count   INTEGER,
            created_at   DATETIME DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS settings (
            key         TEXT PRIMARY KEY,
            value       TEXT NOT NULL,
            updated_at  DATETIME DEFAULT (datetime('now'))
        );
        ",
    )?;

    seed_settings(conn)?;
    migrate_removed_dictionary_feature(conn)?;
    migrate_hotkey_defaults(conn)?;
    migrate_legacy_speech_model_settings(conn)?;

    Ok(())
}

/// Inserts default settings (15 rows). Uses INSERT OR IGNORE for idempotency.
fn seed_settings(conn: &Connection) -> Result<(), LocalYapperError> {
    let seeds = [
        ("hotkey_record", "F8"),
        ("hotkey_hands_free", "Ctrl+F8"),
        ("hotkey_cancel", "Escape"),
        ("hotkey_paste_last", "Ctrl+Alt+J"),
        ("hotkey_open_app", "Ctrl+Alt+O"),
        ("speech_model", "parakeet-110m"),
        ("auto_start", "true"),
        ("sound_effects", "true"),
        ("mute_media", "true"),
        ("language", "en"),
        ("overlay_x", "100"),
        ("overlay_y", "100"),
        ("setup_complete", "false"),
        ("max_recording_seconds", "120"),
        ("auto_inject_delay_ms", "10000"),
    ];

    let tx = conn.unchecked_transaction()?;
    for (key, value) in &seeds {
        tx.execute(
            "INSERT OR IGNORE INTO settings (key, value, updated_at) VALUES (?1, ?2, datetime('now'))",
            rusqlite::params![key, value],
        )?;
    }
    tx.commit()?;

    Ok(())
}

/// Drop legacy dictionary/correction tables and stale settings from older builds.
fn migrate_removed_dictionary_feature(conn: &Connection) -> Result<(), LocalYapperError> {
    conn.execute("DROP TABLE IF EXISTS corrections", [])?;
    conn.execute("DROP TABLE IF EXISTS personal_dictionary", [])?;
    conn.execute(
        "DELETE FROM settings WHERE key IN ('confidence_threshold', 'correction_decay_days', 'training_paragraph_index')",
        [],
    )?;
    Ok(())
}

/// Migrate legacy default hotkeys to the current safer defaults.
/// Safe to run repeatedly - only updates known historical defaults.
fn migrate_hotkey_defaults(conn: &Connection) -> Result<(), LocalYapperError> {
    let legacy_record_values = [
        "Alt+Space",
        "Ctrl+Space",
        "Alt+Alt+Space",
        "Ctrl+Shift+Space",
        "Ctrl+Alt+D",
        "Ctrl+Alt+X",
        "Ctrl+Alt+H",
    ];
    for old_val in &legacy_record_values {
        conn.execute(
            "UPDATE settings SET value = 'F8', updated_at = datetime('now') WHERE key = 'hotkey_record' AND value = ?1",
            rusqlite::params![old_val],
        )?;
        conn.execute(
            "UPDATE settings SET value = 'Ctrl+F8', updated_at = datetime('now') WHERE key = 'hotkey_hands_free' AND value = ?1",
            rusqlite::params![old_val],
        )?;
    }

    conn.execute(
        "UPDATE settings SET value = 'Ctrl+Alt+J', updated_at = datetime('now') WHERE key = 'hotkey_paste_last' AND value = 'Alt+Shift+V'",
        [],
    )?;
    conn.execute(
        "UPDATE settings SET value = 'Ctrl+Alt+O', updated_at = datetime('now') WHERE key = 'hotkey_open_app' AND value = 'Alt+L'",
        [],
    )?;

    Ok(())
}

/// Migrate the old `whisper_model` key to the current `speech_model` key.
/// Safe to run repeatedly - it preserves any existing `speech_model` value.
fn migrate_legacy_speech_model_settings(conn: &Connection) -> Result<(), LocalYapperError> {
    conn.execute(
        "
        INSERT OR IGNORE INTO settings (key, value, updated_at)
        SELECT 'speech_model',
               CASE
                   WHEN value IN ('tiny.en', 'base.en', 'small.en', 'medium.en') THEN 'parakeet-110m'
                   ELSE value
               END,
               datetime('now')
        FROM settings
        WHERE key = 'whisper_model'
        ",
        [],
    )?;

    conn.execute(
        "
        UPDATE settings
        SET value = 'parakeet-110m',
            updated_at = datetime('now')
        WHERE key = 'speech_model'
          AND value IN ('tiny.en', 'base.en', 'small.en', 'medium.en')
        ",
        [],
    )?;

    Ok(())
}
