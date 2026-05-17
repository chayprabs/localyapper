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
    migrate_removed_features(conn)?;
    migrate_hotkey_defaults(conn)?;
    migrate_legacy_speech_model_settings(conn)?;

    Ok(())
}

/// Inserts default settings. Uses INSERT OR IGNORE for idempotency.
///
/// `setup_step` tracks the active wizard step so the user resumes where they
/// left off if they quit the app mid-onboarding. Valid values:
/// `welcome | microphone | hotkey | files | done`.
fn seed_settings(conn: &Connection) -> Result<(), LocalYapperError> {
    let seeds = [
        ("hotkey_record", "F8"),
        ("hotkey_cancel", "Escape"),
        ("hotkey_paste_last", "Ctrl+Alt+J"),
        ("hotkey_open_app", "Ctrl+Alt+O"),
        ("speech_model", "parakeet-110m"),
        ("auto_start", "true"),
        ("overlay_x", "100"),
        ("overlay_y", "100"),
        ("setup_complete", "false"),
        ("setup_step", "welcome"),
        ("idle_unload_seconds", "60"),
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

/// Drop removed feature tables and stale settings from older builds.
fn migrate_removed_features(conn: &Connection) -> Result<(), LocalYapperError> {
    conn.execute("DROP TABLE IF EXISTS corrections", [])?;
    conn.execute("DROP TABLE IF EXISTS personal_dictionary", [])?;
    conn.execute(
        "DELETE FROM settings WHERE key IN ('confidence_threshold', 'correction_decay_days', 'training_paragraph_index', 'auto_inject_delay_ms', 'sound_effects', 'mute_media', 'language', 'max_recording_seconds', 'hotkey_hands_free')",
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::queries;

    #[test]
    fn initialize_database_seeds_current_defaults() -> Result<(), LocalYapperError> {
        let conn = Connection::open_in_memory()?;

        initialize_database(&conn)?;

        assert_eq!(queries::get_setting(&conn, "setup_complete")?, "false");
        assert_eq!(queries::get_setting(&conn, "setup_step")?, "welcome");
        assert_eq!(queries::get_setting(&conn, "hotkey_record")?, "F8");
        assert_eq!(queries::get_setting(&conn, "hotkey_cancel")?, "Escape");
        assert_eq!(
            queries::get_setting(&conn, "speech_model")?,
            "parakeet-110m"
        );
        // hotkey_hands_free is no longer a separate hotkey -- hands-free is a
        // double-tap of the record hotkey.
        assert!(queries::get_setting(&conn, "hotkey_hands_free").is_err());

        let settings_count: i64 =
            conn.query_row("SELECT COUNT(*) FROM settings", [], |row| row.get(0))?;
        assert_eq!(settings_count, 11);
        assert_eq!(queries::get_setting(&conn, "idle_unload_seconds")?, "60");

        Ok(())
    }

    #[test]
    fn initialize_database_removes_legacy_hands_free_hotkey() -> Result<(), LocalYapperError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "
            CREATE TABLE settings (
                key         TEXT PRIMARY KEY,
                value       TEXT NOT NULL,
                updated_at  DATETIME DEFAULT (datetime('now'))
            );
            INSERT INTO settings (key, value) VALUES ('hotkey_hands_free', 'Ctrl+F8');
            ",
        )?;

        initialize_database(&conn)?;

        assert!(queries::get_setting(&conn, "hotkey_hands_free").is_err());

        Ok(())
    }

    #[test]
    fn initialize_database_removes_legacy_feature_state() -> Result<(), LocalYapperError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "
            CREATE TABLE corrections (id TEXT PRIMARY KEY);
            CREATE TABLE personal_dictionary (id TEXT PRIMARY KEY);
            CREATE TABLE settings (
                key         TEXT PRIMARY KEY,
                value       TEXT NOT NULL,
                updated_at  DATETIME DEFAULT (datetime('now'))
            );
            INSERT INTO settings (key, value) VALUES
                ('confidence_threshold', '0.8'),
                ('training_paragraph_index', '4'),
                ('auto_inject_delay_ms', '10000'),
                ('language', 'en');
            ",
        )?;

        initialize_database(&conn)?;

        let corrections_exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'corrections'",
            [],
            |row| row.get(0),
        )?;
        let dictionary_exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'personal_dictionary'",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(corrections_exists, 0);
        assert_eq!(dictionary_exists, 0);

        let stale_settings_count: i64 = conn.query_row(
            "
            SELECT COUNT(*) FROM settings
            WHERE key IN (
                'confidence_threshold',
                'training_paragraph_index',
                'auto_inject_delay_ms',
                'language'
            )
            ",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(stale_settings_count, 0);
        assert_eq!(queries::get_setting(&conn, "setup_complete")?, "false");

        Ok(())
    }

    #[test]
    fn initialize_database_normalizes_removed_speech_model_setting() -> Result<(), LocalYapperError>
    {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "
            CREATE TABLE settings (
                key         TEXT PRIMARY KEY,
                value       TEXT NOT NULL,
                updated_at  DATETIME DEFAULT (datetime('now'))
            );
            INSERT INTO settings (key, value) VALUES ('speech_model', 'base.en');
            ",
        )?;

        initialize_database(&conn)?;

        assert_eq!(
            queries::get_setting(&conn, "speech_model")?,
            "parakeet-110m"
        );

        Ok(())
    }
}
