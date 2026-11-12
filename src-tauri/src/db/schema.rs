// Database schema -- table creation, migrations, and seed data
use rusqlite::Connection;

use crate::error::LocalYapperError;

/// The schema version `initialize_database` brings a database to.
pub const SCHEMA_VERSION: i64 = 2;

/// What `initialize_database` found and did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Migration {
    /// `PRAGMA user_version` before (0 for databases older than versioning).
    pub from_version: i64,
    /// The database had no settings yet: a fresh install.
    pub fresh: bool,
}

/// Creates the tables and migrates with `PRAGMA user_version` (plan P5.3).
///
/// - Version 1 is the schema the app shipped with before the rebuild,
///   including its old clean-ups of removed features.
/// - Version 2 adds every typed setting with its default, removes
///   `overlay_x` and `overlay_y`, and maps values that are no longer
///   valid. The caller then copies the OS launch-at-login state into
///   `auto_start` for upgraded installs.
pub fn initialize_database(conn: &Connection) -> Result<Migration, LocalYapperError> {
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
    let from_version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    let settings_rows: i64 = conn.query_row("SELECT COUNT(*) FROM settings", [], |r| r.get(0))?;
    let fresh = settings_rows == 0;

    if from_version < 1 {
        migrate_removed_features(conn)?;
        migrate_hotkey_defaults(conn)?;
        migrate_legacy_speech_model_settings(conn)?;
        conn.execute_batch("PRAGMA user_version = 1;")?;
    }
    if from_version < 2 {
        migrate_v2(conn)?;
        conn.execute_batch(&format!("PRAGMA user_version = {SCHEMA_VERSION};"))?;
    }
    Ok(Migration {
        from_version,
        fresh,
    })
}

/// Version 2: typed settings.
fn migrate_v2(conn: &Connection) -> Result<(), LocalYapperError> {
    use crate::settings::keys;
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "DELETE FROM settings WHERE key IN ('overlay_x', 'overlay_y')",
        [],
    )?;
    // Values the old app wrote that are no longer valid.
    tx.execute(
        "UPDATE settings SET value = 'auto' WHERE key = 'speech_model' AND value = 'parakeet-0.6b'",
        [],
    )?;
    for key in keys::KEYS {
        let current: Option<String> = tx
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                rusqlite::params![key.name],
                |r| r.get(0),
            )
            .ok();
        let valid = current
            .as_deref()
            .map(|v| key.kind == keys::Kind::Model || keys::validate(key, v, &[]).is_ok())
            .unwrap_or(false);
        if !valid {
            tx.execute(
                "INSERT OR REPLACE INTO settings (key, value, updated_at) VALUES (?1, ?2, datetime('now'))",
                rusqlite::params![key.name, keys::default_of(key)],
            )?;
        }
    }
    tx.commit()?;
    Ok(())
}

fn migrate_removed_features(conn: &Connection) -> Result<(), LocalYapperError> {
    conn.execute("DROP TABLE IF EXISTS corrections", [])?;
    conn.execute("DROP TABLE IF EXISTS personal_dictionary", [])?;
    conn.execute(
        "DELETE FROM settings WHERE key IN ('confidence_threshold', 'correction_decay_days', 'training_paragraph_index', 'auto_inject_delay_ms', 'sound_effects', 'mute_media', 'language', 'hotkey_hands_free')",
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
    // Never rewrite this OS's own default (Alt+Space on macOS).
    let current_default =
        crate::platform::hotkey::default_combo(crate::platform::hotkey::HotkeyId::Record);
    for old_val in legacy_record_values
        .iter()
        .filter(|v| **v != current_default)
    {
        conn.execute(
            "UPDATE settings SET value = ?2, updated_at = datetime('now') WHERE key = 'hotkey_record' AND value = ?1",
            rusqlite::params![old_val, current_default],
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
        assert_eq!(
            queries::get_setting(&conn, "hotkey_record")?,
            crate::platform::hotkey::default_combo(crate::platform::hotkey::HotkeyId::Record)
        );
        assert_eq!(queries::get_setting(&conn, "hotkey_cancel")?, "Escape");
        assert_eq!(queries::get_setting(&conn, "speech_model")?, "auto");
        // hotkey_hands_free is no longer a separate hotkey -- hands-free is a
        // double-tap of the record hotkey.
        assert!(queries::get_setting(&conn, "hotkey_hands_free").is_err());
        assert_eq!(queries::get_setting(&conn, "max_recording_seconds")?, "600");

        let settings_count: i64 =
            conn.query_row("SELECT COUNT(*) FROM settings", [], |row| row.get(0))?;
        assert_eq!(settings_count, crate::settings::keys::KEYS.len() as i64);
        assert_eq!(queries::get_setting(&conn, "cancel_key_enabled")?, "true");
        assert_eq!(
            queries::get_setting(&conn, "cancel_key_in_hands_free")?,
            "false"
        );
        assert_eq!(queries::get_setting(&conn, "idle_unload_seconds")?, "120");
        assert_eq!(queries::get_setting(&conn, "auto_start")?, "false");
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        assert_eq!(version, SCHEMA_VERSION);

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
