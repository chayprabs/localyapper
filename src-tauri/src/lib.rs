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
use state::AppState;
use std::sync::{Arc, Mutex};
use tauri::Manager;

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

            app.manage(AppState {
                db: Arc::new(Mutex::new(conn)),
                recorder: Arc::new(AudioRecorder::new()),
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
