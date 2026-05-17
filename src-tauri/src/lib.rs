mod audio;
mod commands;
mod context;
mod db;
mod error;
mod hotkey;
mod injection;
mod models;
mod state;
mod stt;
mod tray;

use audio::capture::AudioRecorder;
use state::AppState;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use stt::whisper::WhisperEngine;
use tauri::Manager;

/// Candidate directories where the STT model might be found.
///
/// Checks for the selected model first, then falls back to the default Parakeet model.
pub(crate) fn speech_model_candidates(app: &tauri::AppHandle, model_setting: &str) -> Vec<PathBuf> {
    let models_dir = match app.path().app_data_dir() {
        Ok(d) => d.join("models"),
        Err(_) => return vec![],
    };
    let model_setting = stt::whisper::normalize_stt_model_name(model_setting);

    let primary = models_dir.join(stt::whisper::stt_model_dir_name(model_setting));
    let fallback = if model_setting != stt::whisper::DEFAULT_STT_MODEL {
        Some(models_dir.join(stt::whisper::stt_model_dir_name(
            stt::whisper::DEFAULT_STT_MODEL,
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

pub(crate) fn resolve_speech_model_dir(
    app: &tauri::AppHandle,
    model_setting: &str,
) -> Option<PathBuf> {
    speech_model_candidates(app, model_setting)
        .into_iter()
        .find(|candidate| stt::whisper::is_valid_stt_model_dir(candidate))
}

pub(crate) fn load_speech_model_from_setting(
    app: &tauri::AppHandle,
    model_setting: &str,
) -> Result<Arc<WhisperEngine>, String> {
    let candidate = resolve_speech_model_dir(app, model_setting).ok_or_else(|| {
        let model_setting = stt::whisper::normalize_stt_model_name(model_setting);
        format!(
            "Speech model files not found for {}. Open Settings > Speech to download them.",
            stt::whisper::stt_model_dir_name(model_setting)
        )
    })?;

    log::info!("STT: Loading engine from {}", candidate.display());
    WhisperEngine::new(&candidate)
        .map(Arc::new)
        .map_err(|e| format!("Failed to load STT from {}: {e}", candidate.display()))
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

    if let Err(e) = tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_notification::init())
        .setup(|app| -> Result<(), Box<dyn std::error::Error>> {
            let app_data_dir = app.path().app_data_dir().map_err(|e| {
                log::error!("Failed to resolve app data directory: {e}");
                Box::new(e) as Box<dyn std::error::Error>
            })?;

            let conn = db::open_database(&app_data_dir).map_err(|e| {
                log::error!("Failed to initialize database: {e}");
                Box::new(e) as Box<dyn std::error::Error>
            })?;
            let speech_model_setting = db::queries::get_setting(&conn, "speech_model")
                .unwrap_or_else(|_| stt::whisper::DEFAULT_STT_MODEL.to_string());
            let auto_start_enabled = db::queries::get_setting(&conn, "auto_start")
                .map(|value| value.eq_ignore_ascii_case("true"))
                .unwrap_or(true);
            let speech_model_installed =
                resolve_speech_model_dir(app.handle(), &speech_model_setting).is_some();

            if speech_model_installed {
                log::info!(
                    "Speech model files found at startup; engine will load on first dictation"
                );
            } else {
                log::warn!(
                    "Speech model files not found at startup - STT unavailable until downloaded"
                );
            }

            app.manage(AppState {
                db: Arc::new(Mutex::new(conn)),
                recorder: Arc::new(AudioRecorder::new()),
                // Lazy-loaded by ensure_speech_model_loaded on first use. Models are
                // dropped after idle_unload_seconds by ModelLifecycle to keep idle
                // RAM low; reloading from disk-cached weights costs ~1s.
                whisper: Arc::new(Mutex::new(None)),
                vad: Arc::new(Mutex::new(None)),
                lifecycle: stt::lifecycle::ModelLifecycle::new(),
                last_injection: Arc::new(Mutex::new(None)),
                download_cancel: Arc::new(AtomicBool::new(false)),
                paused: Arc::new(AtomicBool::new(false)),
            });

            if !speech_model_installed {
                send_notification(
                    app.handle(),
                    "LocalYapper",
                    "Speech model not downloaded - open Settings to get started",
                );
            }

            match hotkey::manager::register_hotkeys(app.handle()) {
                Ok(()) => log::info!("STARTUP: Hotkeys registered successfully"),
                Err(e) => log::error!("STARTUP: FAILED to register hotkeys: {e}"),
            }

            if let Err(e) = tray::setup_tray(app) {
                log::error!("Failed to setup system tray: {e}");
            }

            {
                use tauri_plugin_autostart::ManagerExt;
                let manager = app.autolaunch();
                let is_enabled = manager.is_enabled().unwrap_or(false);
                if auto_start_enabled && !is_enabled {
                    if let Err(e) = manager.enable() {
                        log::warn!("Failed to enable autostart: {e}");
                    } else {
                        log::info!("Autostart enabled by default");
                    }
                } else if !auto_start_enabled && is_enabled {
                    if let Err(e) = manager.disable() {
                        log::warn!("Failed to disable autostart: {e}");
                    } else {
                        log::info!("Autostart disabled by setting");
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
            commands::system::check_permissions,
            commands::system::get_paused_state,
            commands::system::open_accessibility_settings,
            commands::system::open_mic_settings,
        ])
        .run(tauri::generate_context!())
    {
        log::error!("Error while running Tauri application: {e}");
    }
}
