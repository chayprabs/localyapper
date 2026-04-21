#![allow(clippy::duplicate_mod, dead_code)]

mod audio;
mod commands;
mod context;
mod correction;
mod db;
mod error;
mod hotkey;
mod injection;
mod models;
mod state;
mod stt;
mod tray;

use audio::capture::AudioRecorder;
use audio::vad::SileroVad;
use correction::engine::CorrectionEngine;
use state::AppState;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use stt::whisper::WhisperEngine;
use tauri::Manager;

/// Candidate directories where the STT model might be found.
///
/// Checks for the selected model first, then falls back to the default Parakeet model.
pub(crate) fn speech_model_candidates(
    app: &tauri::AppHandle,
    model_setting: &str,
) -> Vec<std::path::PathBuf> {
    let models_dir = match app.path().app_data_dir() {
        Ok(d) => d.join("models"),
        Err(_) => return vec![],
    };

    let primary = models_dir.join(stt::whisper::stt_model_dir_name(model_setting));
    let fallback = if model_setting != stt::whisper::DEFAULT_WHISPER_MODEL {
        Some(models_dir.join(stt::whisper::stt_model_dir_name(
            stt::whisper::DEFAULT_WHISPER_MODEL,
        )))
    } else {
        None
    };

    let mut candidates = vec![primary];
    if let Some(fallback) = fallback {
        candidates.push(fallback);
    }
    candidates
}

/// Attempt to load the STT model by scanning candidate directories.
/// Reads the `speech_model` setting from DB to determine which model to load.
/// Returns `None` with a warning log if the model is not found or fails to load.
fn load_speech_model(app: &tauri::App, conn: &rusqlite::Connection) -> Option<Arc<WhisperEngine>> {
    let model_setting = db::queries::get_setting(conn, "speech_model")
        .unwrap_or_else(|_| stt::whisper::DEFAULT_WHISPER_MODEL.to_string());

    let candidates = speech_model_candidates(app.handle(), &model_setting);
    println!(
        "STT: Startup load - model setting='{}', candidates: {:?}",
        model_setting,
        candidates
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
    );

    for candidate in &candidates {
        if candidate.exists() && candidate.is_dir() {
            println!(
                "STT: Found model dir at {}, loading...",
                candidate.display()
            );
            log::info!("Found STT model at {}", candidate.display());
            match WhisperEngine::new(candidate) {
                Ok(engine) => {
                    println!("STT: Engine loaded successfully");
                    log::info!("STT engine loaded successfully");
                    return Some(Arc::new(engine));
                }
                Err(e) => {
                    println!("STT: Load FAILED from {}: {}", candidate.display(), e);
                    log::warn!(
                        "Failed to load STT model from {}: {}",
                        candidate.display(),
                        e
                    );
                }
            }
        } else {
            println!("STT: Directory not found at {}", candidate.display());
        }
    }

    println!("STT: No model loaded - STT unavailable until downloaded");
    log::warn!(
        "STT model ({}) not found. STT will be unavailable until the model is downloaded.",
        stt::whisper::stt_model_dir_name(&model_setting)
    );
    None
}

/// Send a system notification via tauri-plugin-notification.
fn send_notification(handle: &tauri::AppHandle, title: &str, body: &str) {
    use tauri_plugin_notification::NotificationExt;
    if let Err(e) = handle
        .notification()
        .builder()
        .title(title)
        .body(body)
        .show()
    {
        log::warn!("Failed to send notification: {e}");
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

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

            let conn = db::open_database(&app_data_dir).expect("Failed to initialize database");

            let whisper = load_speech_model(app, &conn);
            if whisper.is_some() {
                log::info!("STT model loaded at startup");
            } else {
                log::warn!("STT model not found at startup - STT unavailable until downloaded");
            }

            let correction_engine = Arc::new(CorrectionEngine::new());
            let threshold: f64 = db::queries::get_setting(&conn, "confidence_threshold")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.6);
            if let Err(e) = correction_engine.load(&conn, threshold) {
                log::warn!("Failed to load correction engine: {e}");
            }

            let silero_vad = {
                let vad_path = app_data_dir
                    .join("models")
                    .join(stt::whisper::SILERO_VAD_FILENAME);
                if vad_path.exists() {
                    match SileroVad::new(&vad_path) {
                        Ok(vad) => {
                            println!("VAD: Silero VAD loaded successfully");
                            Some(vad)
                        }
                        Err(e) => {
                            println!("VAD: Failed to load Silero VAD: {e}, using energy fallback");
                            log::warn!("Failed to load Silero VAD: {e}");
                            None
                        }
                    }
                } else {
                    println!(
                        "VAD: Silero model not found at {}, using energy fallback",
                        vad_path.display()
                    );
                    None
                }
            };

            app.manage(AppState {
                db: Arc::new(Mutex::new(conn)),
                recorder: Arc::new(AudioRecorder::new()),
                whisper: Arc::new(Mutex::new(whisper)),
                vad: Arc::new(Mutex::new(silero_vad)),
                last_injection: Arc::new(Mutex::new(None)),
                correction_engine,
                download_cancel: Arc::new(AtomicBool::new(false)),
                paused: Arc::new(AtomicBool::new(false)),
            });

            let speech_model_loaded = app
                .state::<AppState>()
                .whisper
                .lock()
                .map(|g| g.is_some())
                .unwrap_or(false);
            if speech_model_loaded {
                send_notification(
                    app.handle(),
                    "LocalYapper",
                    "Ready - voice dictation is active",
                );
            } else {
                send_notification(
                    app.handle(),
                    "LocalYapper",
                    "Speech model not downloaded - open Settings to get started",
                );
            }

            match hotkey::manager::register_hotkeys(app.handle()) {
                Ok(()) => println!("STARTUP: Hotkeys registered successfully"),
                Err(e) => println!("STARTUP: FAILED to register hotkeys: {e}"),
            }

            if let Err(e) = tray::setup_tray(app) {
                log::error!("Failed to setup system tray: {e}");
            }

            {
                use tauri_plugin_autostart::ManagerExt;
                let manager = app.autolaunch();
                if !manager.is_enabled().unwrap_or(false) {
                    if let Err(e) = manager.enable() {
                        log::warn!("Failed to enable autostart: {e}");
                    } else {
                        log::info!("Autostart enabled by default");
                    }
                }
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
            commands::recording::start_recording,
            commands::recording::stop_recording,
            commands::recording::run_pipeline,
            commands::recording::inject_text,
            commands::recording::paste_last,
            commands::recording::cancel_recording,
            commands::models::download_speech_model,
            commands::models::cancel_model_download,
            commands::models::reload_models,
            commands::models::check_models_status,
            commands::models::check_speech_model_file_exists,
            commands::models::delete_speech_model,
            commands::corrections::get_corrections,
            commands::corrections::add_correction,
            commands::corrections::delete_correction,
            commands::corrections::export_dictionary,
            commands::corrections::import_dictionary,
            commands::corrections::get_corrections_count,
            commands::corrections::compute_training_diffs,
            commands::history::get_history,
            commands::history::delete_history_entry,
            commands::history::clear_history,
            commands::history::get_stats,
            commands::settings::get_setting,
            commands::settings::set_setting,
            commands::settings::get_all_settings,
            commands::hotkeys::update_hotkey,
            commands::hotkeys::reset_hotkeys,
            commands::system::get_focused_app,
            commands::system::check_update,
            commands::system::check_permissions,
            commands::system::open_accessibility_settings,
            commands::system::open_mic_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
