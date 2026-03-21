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
use llm::engine::LlmEngine;
use state::AppState;
use stt::whisper::WhisperEngine;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tauri::Manager;

/// Candidate paths where the Whisper model might be found.
///
/// Checks for the selected model first, then falls back to other variants
/// for backwards compatibility (e.g. user had tiny.en before upgrading).
pub(crate) fn whisper_model_candidates(app: &tauri::AppHandle, model_setting: &str) -> Vec<std::path::PathBuf> {
    let models_dir = match app.path().app_data_dir() {
        Ok(d) => d.join("models"),
        Err(_) => return vec![],
    };

    let primary = models_dir.join(stt::whisper::whisper_model_filename(model_setting));

    // Fallback: if selected model isn't found, try tiny.en (backwards compat)
    let fallback = if model_setting != "tiny.en" {
        Some(models_dir.join(stt::whisper::whisper_model_filename("tiny.en")))
    } else {
        None
    };

    let mut candidates = vec![primary];
    if let Some(fb) = fallback {
        candidates.push(fb);
    }
    candidates
}


/// Attempt to load the Whisper model by scanning candidate paths.
/// Reads the `whisper_model` setting from DB to determine which model to load.
/// Returns `None` with a warning log if the model file is not found or fails to load.
fn load_whisper_model(app: &tauri::App, conn: &rusqlite::Connection) -> Option<Arc<WhisperEngine>> {
    let model_setting = db::queries::get_setting(conn, "whisper_model")
        .unwrap_or_else(|_| stt::whisper::DEFAULT_WHISPER_MODEL.to_string());

    let candidates = whisper_model_candidates(app.handle(), &model_setting);

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
        stt::whisper::whisper_model_filename(&model_setting)
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

            // Load Whisper at startup (safe now that llama-cpp-2 is removed).
            let whisper = load_whisper_model(app, &conn);
            if whisper.is_some() {
                log::info!("Whisper model loaded at startup");
            } else {
                log::warn!("Whisper model not found at startup — STT unavailable until downloaded");
            }
            // LLM loaded lazily via reload_models() — not at startup.
            let llm: Option<Arc<LlmEngine>> = None;
            log::info!("LLM will be loaded lazily (via reload_models or first dictation).");

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
            // Model management (12)
            commands::models::check_ollama,
            commands::models::download_model,
            commands::models::download_whisper_model,
            commands::models::cancel_model_download,
            commands::models::get_ollama_models,
            commands::models::test_byok_connection,
            commands::models::reload_models,
            commands::models::check_models_status,
            commands::models::check_llm_file_exists,
            commands::models::delete_llm_model,
            commands::models::check_whisper_file_exists,
            commands::models::delete_whisper_model,
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
