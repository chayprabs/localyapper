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

/// Candidate paths where the Whisper model might be found.
pub(crate) fn whisper_model_candidates(app: &tauri::AppHandle) -> Vec<std::path::PathBuf> {
    [
        app.path().resource_dir().ok().map(|p| p.join("resources").join(WHISPER_MODEL_FILENAME)),
        app.path().app_data_dir().ok().map(|p| p.join("models").join(WHISPER_MODEL_FILENAME)),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// Candidate paths where the LLM model might be found.
pub(crate) fn llm_model_candidates(app: &tauri::AppHandle) -> Vec<std::path::PathBuf> {
    [
        app.path().app_data_dir().ok().map(|p| p.join("models").join(LLM_MODEL_FILENAME)),
        app.path().resource_dir().ok().map(|p| p.join("resources").join(LLM_MODEL_FILENAME)),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// Attempt to load the Whisper model by scanning candidate paths.
/// Returns `None` with a warning log if the model file is not found or fails to load.
fn load_whisper_model(app: &tauri::App) -> Option<Arc<WhisperEngine>> {
    let candidates = whisper_model_candidates(app.handle());

    for candidate in &candidates {
        if candidate.exists() {
            log::info!("Found Whisper model at {}", candidate.display());
            match WhisperEngine::new(candidate) {
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
        "Whisper model ({}) not found. STT will be unavailable until the model is downloaded.",
        WHISPER_MODEL_FILENAME
    );
    None
}

/// Attempt to load the LLM model by scanning candidate paths.
/// Returns `None` with a warning log if the model file is not found or fails to load.
fn load_llm_model(app: &tauri::App) -> Option<Arc<LlmEngine>> {
    let candidates = llm_model_candidates(app.handle());

    for candidate in &candidates {
        if candidate.exists() {
            log::info!("Found LLM model at {}", candidate.display());
            match LlmEngine::new(candidate) {
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
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .init();

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
                whisper: Arc::new(Mutex::new(whisper)),
                llm: Arc::new(Mutex::new(llm)),
                last_injection: Arc::new(Mutex::new(None)),
                correction_engine,
                download_cancel: Arc::new(AtomicBool::new(false)),
            });

            // Register global hotkeys (hold-to-talk, cancel, paste-last, open-app)
            if let Err(e) = hotkey::manager::register_hotkeys(app.handle()) {
                log::error!("Failed to register hotkeys: {e}");
            }

            // Setup system tray icon and menu
            if let Err(e) = tray::setup_tray(app) {
                log::error!("Failed to setup system tray: {e}");
            }

            log::info!("LocalYapper initialized. DB at {:?}", app_data_dir);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            // Recording & pipeline (6)
            commands::recording::start_recording,
            commands::recording::stop_recording,
            commands::recording::run_pipeline,
            commands::recording::inject_text,
            commands::recording::paste_last,
            commands::recording::cancel_recording,
            // Model management (8)
            commands::models::check_ollama,
            commands::models::download_model,
            commands::models::download_whisper_model,
            commands::models::cancel_model_download,
            commands::models::get_ollama_models,
            commands::models::test_byok_connection,
            commands::models::reload_models,
            commands::models::check_models_status,
            // Modes (6)
            commands::modes::get_modes,
            commands::modes::create_mode,
            commands::modes::update_mode,
            commands::modes::delete_mode,
            commands::modes::set_active_mode,
            commands::modes::get_active_mode,
            // Corrections (7)
            commands::corrections::get_corrections,
            commands::corrections::add_correction,
            commands::corrections::delete_correction,
            commands::corrections::export_dictionary,
            commands::corrections::import_dictionary,
            commands::corrections::get_corrections_count,
            commands::corrections::compute_training_diffs,
            // History (4)
            commands::history::get_history,
            commands::history::delete_history_entry,
            commands::history::clear_history,
            commands::history::get_stats,
            // Settings (3)
            commands::settings::get_setting,
            commands::settings::set_setting,
            commands::settings::get_all_settings,
            // Hotkeys (2)
            commands::hotkeys::update_hotkey,
            commands::hotkeys::reset_hotkeys,
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
