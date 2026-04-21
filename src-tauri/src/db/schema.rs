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

        CREATE TABLE IF NOT EXISTS corrections (
            id            TEXT PRIMARY KEY,
            raw_word      TEXT NOT NULL,
            corrected     TEXT NOT NULL,
            count         INTEGER DEFAULT 1,
            confidence    REAL DEFAULT 0.0,
            last_used_at  DATETIME,
            created_at    DATETIME DEFAULT (datetime('now')),
            UNIQUE(raw_word, corrected)
        );

        CREATE TABLE IF NOT EXISTS personal_dictionary (
            id        TEXT PRIMARY KEY,
            word      TEXT NOT NULL UNIQUE,
            count     INTEGER DEFAULT 1,
            added_at  DATETIME DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS settings (
            key         TEXT PRIMARY KEY,
            value       TEXT NOT NULL,
            updated_at  DATETIME DEFAULT (datetime('now'))
        );
        ",
    )?;

    seed_settings(conn)?;
    migrate_hotkey_defaults(conn)?;
    migrate_legacy_speech_model_settings(conn)?;

    Ok(())
}

/// Inserts default settings (17 rows). Uses INSERT OR IGNORE for idempotency.
fn seed_settings(conn: &Connection) -> Result<(), LocalYapperError> {
    let seeds = [
        ("hotkey_record", "Ctrl+Shift+Space"),
        ("hotkey_hands_free", "Ctrl+Shift+Space"),
        ("hotkey_cancel", "Escape"),
        ("hotkey_paste_last", "Alt+Shift+V"),
        ("hotkey_open_app", "Alt+L"),
        ("speech_model", "parakeet-110m"),
        ("auto_start", "true"),
        ("sound_effects", "true"),
        ("mute_media", "true"),
        ("confidence_threshold", "0.6"),
        ("correction_decay_days", "30"),
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

/// Migrate hotkey_record from conflicting defaults (Alt+Space, Ctrl+Space) to Ctrl+Shift+Space.
/// Safe to run repeatedly - only updates if value matches a known conflicting default.
fn migrate_hotkey_defaults(conn: &Connection) -> Result<(), LocalYapperError> {
    let conflicting = ["Alt+Space", "Ctrl+Space", "Alt+Alt+Space"];
    for old_val in &conflicting {
        conn.execute(
            "UPDATE settings SET value = 'Ctrl+Shift+Space', updated_at = datetime('now') WHERE key = 'hotkey_record' AND value = ?1",
            rusqlite::params![old_val],
        )?;
        conn.execute(
            "UPDATE settings SET value = 'Ctrl+Shift+Space', updated_at = datetime('now') WHERE key = 'hotkey_hands_free' AND value = ?1",
            rusqlite::params![old_val],
        )?;
    }
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
