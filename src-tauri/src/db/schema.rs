// Database schema -- table creation, migrations, and seed data
use rusqlite::Connection;

use crate::error::LocalYapperError;

/// Creates all 6 tables and seeds default data in a single transaction.
pub fn initialize_database(conn: &Connection) -> Result<(), LocalYapperError> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS transcription_history (
            id           TEXT PRIMARY KEY,
            raw_text     TEXT NOT NULL,
            final_text   TEXT NOT NULL,
            app_name     TEXT,
            mode_id      TEXT,
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

        CREATE TABLE IF NOT EXISTS modes (
            id             TEXT PRIMARY KEY,
            name           TEXT NOT NULL,
            system_prompt  TEXT NOT NULL,
            skip_llm       INTEGER DEFAULT 0,
            is_builtin     INTEGER DEFAULT 0,
            color          TEXT DEFAULT 'purple',
            created_at     DATETIME DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS app_profiles (
            id        TEXT PRIMARY KEY,
            app_name  TEXT NOT NULL UNIQUE,
            mode_id   TEXT NOT NULL,
            FOREIGN KEY (mode_id) REFERENCES modes(id)
        );

        CREATE TABLE IF NOT EXISTS settings (
            key         TEXT PRIMARY KEY,
            value       TEXT NOT NULL,
            updated_at  DATETIME DEFAULT (datetime('now'))
        );
        ",
    )?;

    seed_settings(conn)?;
    seed_modes(conn)?;
    migrate_hotkey_defaults(conn)?;
    migrate_whisper_model_default(conn)?;

    Ok(())
}

/// Inserts default settings (22 rows). Uses INSERT OR IGNORE for idempotency.
fn seed_settings(conn: &Connection) -> Result<(), LocalYapperError> {
    let seeds = [
        ("hotkey_record", "Ctrl+Shift+Space"),
        ("hotkey_hands_free", "Ctrl+Shift+Space"),
        ("hotkey_cancel", "Escape"),
        ("hotkey_paste_last", "Alt+Shift+V"),
        ("hotkey_open_app", "Alt+L"),
        ("whisper_model", "base.en"),
        ("llm_mode", "local"),
        ("ollama_model", "qwen2.5:0.5b"),
        ("byok_provider", "openai"),
        ("byok_api_key", ""),
        ("active_mode_id", "builtin_casual"),
        ("auto_start", "true"),
        ("sound_effects", "true"),
        ("mute_media", "true"),
        ("confidence_threshold", "0.6"),
        ("correction_decay_days", "30"),
        ("language", "en"),
        ("overlay_x", "100"),
        ("overlay_y", "100"),
        ("setup_complete", "false"),
        ("model_path", ""),
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

/// Migrate whisper_model from old default "tiny.en" to "base.en".
/// Safe to run repeatedly — only updates if value is still "tiny.en".
fn migrate_whisper_model_default(conn: &Connection) -> Result<(), LocalYapperError> {
    conn.execute(
        "UPDATE settings SET value = 'base.en', updated_at = datetime('now') WHERE key = 'whisper_model' AND value = 'tiny.en'",
        [],
    )?;
    Ok(())
}

/// Migrate hotkey_record from conflicting defaults (Alt+Space, Ctrl+Space) to Ctrl+Shift+Space.
/// Safe to run repeatedly — only updates if value matches a known conflicting default.
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

/// Inserts 5 built-in modes. Uses INSERT OR IGNORE for idempotency.
fn seed_modes(conn: &Connection) -> Result<(), LocalYapperError> {
    let modes = [
        (
            "builtin_casual",
            "Casual",
            "You are a voice dictation cleanup assistant. Remove filler words (um, uh, like, you know, basically, literally). Fix punctuation and capitalization. Preserve abbreviations and casual tone. Keep it conversational. Output ONLY the cleaned text, nothing else.",
            0,
            "blue",
        ),
        (
            "builtin_formal",
            "Formal",
            "You are a voice dictation cleanup assistant. Remove filler words. Write in complete, professional sentences. Fix grammar. Ensure proper punctuation and capitalization. Maintain a formal, professional tone throughout. Output ONLY the cleaned text, nothing else.",
            0,
            "purple",
        ),
        (
            "builtin_code",
            "Code",
            "You are a voice dictation cleanup assistant for a developer. Preserve all technical terms, variable names, function names, and programming concepts exactly as spoken. Minimal cleanup only — fix obvious pronunciation errors but never change technical vocabulary. Output ONLY the cleaned text, nothing else.",
            0,
            "green",
        ),
        (
            "builtin_braindump",
            "Brain dump",
            "",
            1,
            "gray",
        ),
        (
            "builtin_translate",
            "Translate → EN",
            "You are a voice dictation assistant. The user may speak in any language. Detect the language and translate the content to English. Clean up the translation — remove fillers, fix grammar. Output ONLY the English translation, nothing else.",
            0,
            "orange",
        ),
    ];

    let tx = conn.unchecked_transaction()?;
    for (id, name, prompt, skip_llm, color) in &modes {
        tx.execute(
            "INSERT OR IGNORE INTO modes (id, name, system_prompt, skip_llm, is_builtin, color, created_at) VALUES (?1, ?2, ?3, ?4, 1, ?5, datetime('now'))",
            rusqlite::params![id, name, prompt, skip_llm, color],
        )?;
    }
    tx.commit()?;

    Ok(())
}
