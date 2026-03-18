#![allow(clippy::duplicate_mod, dead_code)]

mod audio;
mod commands;
mod context;
mod correction;
mod db;
mod error;
mod hotkey;
mod injection;
mod llm;
mod models;
mod stt;
mod state;
mod tray;

use audio::capture::AudioRecorder;
use correction::engine::CorrectionEngine;
use llm::engine::{LlmEngine, LLM_MODEL_FILENAME};
use state::AppState;
use stt::whisper::{WhisperEngine, WHISPER_MODEL_FILENAME};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tauri::Manager;

/// Attempt to load the Whisper model from bundled resources or app data directory.
/// Returns `None` with a warning log if the model file is not found or fails to load.
fn load_whisper_model(app: &tauri::App) -> Option<Arc<WhisperEngine>> {
    let candidates = [
        // Bundled resource path (production builds)
        app.path().resource_dir().ok().map(|p| p.join("resources").join(WHISPER_MODEL_FILENAME)),
        // App data directory (downloaded on first launch)
        app.path().app_data_dir().ok().map(|p| p.join(WHISPER_MODEL_FILENAME)),
    ];

    for candidate in candidates.into_iter().flatten() {
        if candidate.exists() {
            log::info!("Found Whisper model at {}", candidate.display());
            match WhisperEngine::new(&candidate) {
                Ok(engine) => {
                    log::info!("Whisper engine loaded successfully");
                    return Some(Arc::new(engine));
                }
                Err(e) => {
                    log::warn!("Failed to load Whisper model from {}: {}", candidate.display(), e);
                }
            }
        }
    }

    log::warn!(
        "Whisper model ({}) not found. STT will be unavailable until the model is placed in resources/ or app data.",
        WHISPER_MODEL_FILENAME
    );
    None
}

/// Attempt to load the LLM model from app data models dir or bundled resources.
/// Returns `None` with a warning log if the model file is not found or fails to load.
fn load_llm_model(app: &tauri::App) -> Option<Arc<LlmEngine>> {
    let candidates = [
        // App data models directory (downloaded on first launch)
        app.path().app_data_dir().ok().map(|p| p.join("models").join(LLM_MODEL_FILENAME)),
        // Bundled resource path (production builds)
        app.path().resource_dir().ok().map(|p| p.join("resources").join(LLM_MODEL_FILENAME)),
    ];

    for candidate in candidates.into_iter().flatten() {
        if candidate.exists() {
            log::info!("Found LLM model at {}", candidate.display());
            match LlmEngine::new(&candidate) {
                Ok(engine) => {
                    log::info!("LLM engine loaded successfully");
                    return Some(Arc::new(engine));
                }
                Err(e) => {
                    log::warn!("Failed to load LLM model from {}: {}", candidate.display(), e);
                }
            }
        }
    }

    log::warn!(
        "LLM model ({}) not found. LLM cleanup will be skipped until the model is downloaded.",
        LLM_MODEL_FILENAME
    );
    None
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to resolve app data directory");

            let conn = db::open_database(&app_data_dir)
                .expect("Failed to initialize database");

            // Load Whisper STT model
            let whisper = load_whisper_model(app);

            // Load LLM model
            let llm = load_llm_model(app);

            // Initialize correction engine
            let correction_engine = Arc::new(CorrectionEngine::new());
            let threshold: f64 = db::queries::get_setting(&conn, "confidence_threshold")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.6);
            if let Err(e) = correction_engine.load(&conn, threshold) {
                log::warn!("Failed to load correction engine: {e}");
            }

            app.manage(AppState {
                db: Arc::new(Mutex::new(conn)),
                recorder: Arc::new(AudioRecorder::new()),
                whisper,
                llm,
                last_injection: Arc::new(Mutex::new(None)),
                correction_engine,
                download_cancel: Arc::new(AtomicBool::new(false)),
            });

            log::info!("LocalYapper initialized. DB at {:?}", app_data_dir);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Recording & pipeline (6)
            commands::recording::start_recording,
            commands::recording::stop_recording,
            commands::recording::run_pipeline,
            commands::recording::inject_text,
            commands::recording::paste_last,
            commands::recording::cancel_recording,
            // Model management (5)
            commands::models::check_ollama,
            commands::models::download_model,
            commands::models::cancel_model_download,
            commands::models::get_ollama_models,
            commands::models::test_byok_connection,
            // Modes (6)
            commands::modes::get_modes,
            commands::modes::create_mode,
            commands::modes::update_mode,
            commands::modes::delete_mode,
            commands::modes::set_active_mode,
            commands::modes::get_active_mode,
            // Corrections (5)
            commands::corrections::get_corrections,
            commands::corrections::add_correction,
            commands::corrections::delete_correction,
            commands::corrections::export_dictionary,
            commands::corrections::import_dictionary,
            // History (4)
            commands::history::get_history,
            commands::history::delete_history_entry,
            commands::history::clear_history,
            commands::history::get_stats,
            // Settings (3)
            commands::settings::get_setting,
            commands::settings::set_setting,
            commands::settings::get_all_settings,
            // System (5)
            commands::system::get_focused_app,
            commands::system::check_update,
            commands::system::check_permissions,
            commands::system::open_accessibility_settings,
            commands::system::open_mic_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
