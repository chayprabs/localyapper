mod commands;
mod context;
pub mod controller;
pub mod core;
mod db;
mod error;
mod injection;
mod models;
pub mod platform;
mod state;
mod stt;
mod tray;

// Replace the system allocator with mimalloc. sherpa-onnx allocates and
// frees large scratch buffers during transcription; the Windows system
// allocator tends to retain those pages instead of returning them to the
// OS, which would partially undo our idle-unload work.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use state::AppState;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tauri::Manager;

/// Directory holding downloaded speech models.
pub(crate) fn models_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|d| d.join("models"))
        .map_err(|e| e.to_string())
}

/// Resolve the `speech_model` setting to files on disk.
pub(crate) fn resolve_model_spec(
    app: &tauri::AppHandle,
) -> Result<core::stt::manager::ModelSpec, String> {
    let setting = app
        .state::<AppState>()
        .db
        .lock()
        .ok()
        .and_then(|c| db::queries::get_setting(&c, "speech_model").ok())
        .unwrap_or_else(|| stt::paths::DEFAULT_STT_MODEL.to_string());
    stt::paths::resolve_spec(&models_dir(app)?, &setting)
}

/// Silero VAD model, if present.
fn vad_model_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    let p = models_dir(app).ok()?.join(stt::paths::SILERO_VAD_FILENAME);
    p.is_file().then_some(p)
}

/// Show the main settings window, building a new WebView when it does not
/// exist. The main window is destroyed on user close to free its WebView
/// process; this helper recreates it on demand from the tray and the
/// open-app hotkey.
pub(crate) fn show_or_create_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }

    match tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::App("/".into()))
        .title("LocalYapper")
        .inner_size(900.0, 650.0)
        .min_inner_size(560.0, 480.0)
        .center()
        .resizable(true)
        .decorations(false)
        .visible(true)
        .build()
    {
        Ok(window) => {
            let _ = window.set_focus();
            log::info!("Main settings window created on demand");
        }
        Err(e) => log::error!("Failed to recreate main window: {e}"),
    }
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
            let settings_map: HashMap<String, String> =
                db::queries::get_all_settings(&conn).unwrap_or_default();
            let auto_start_enabled = settings_map
                .get("auto_start")
                .map(|v| v.eq_ignore_ascii_case("true"))
                .unwrap_or(true);
            let speech_model_installed = stt::paths::resolve_spec(
                &models_dir(app.handle())?,
                settings_map
                    .get("speech_model")
                    .map_or(stt::paths::DEFAULT_STT_MODEL, String::as_str),
            )
            .is_ok();
            if !speech_model_installed {
                log::warn!("Speech model files not found at startup");
            }

            let started = controller::start(controller::ControllerParts {
                app: app.handle().clone(),
                settings: controller::ControllerSettings::from_map(&settings_map),
                overlay: Arc::new(platform::overlay::webview::WebviewOverlay::new(
                    app.handle().clone(),
                )),
                resolve_model: Box::new(resolve_model_spec),
                vad_model: vad_model_path(app.handle()),
            })?;

            app.manage(AppState {
                db: Arc::new(Mutex::new(conn)),
                controller: started.handle,
                models: started.manager,
                paused: started.paused,
                last_injection: started.last_injection,
                download_cancel: Arc::new(AtomicBool::new(false)),
                hotkeys: Mutex::new(HashMap::new()),
            });

            if !speech_model_installed {
                send_notification(
                    app.handle(),
                    "LocalYapper",
                    "Speech model not downloaded - open Settings to get started",
                );
            }

            match platform::hotkey::shortcut::reload(app.handle()) {
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
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                if window.label() == "main" {
                    // Allow the close to destroy the WebView and free its
                    // renderer process. The tray + open-app hotkey rebuild
                    // it on demand via show_or_create_main_window.
                    log::info!("Main window closed; WebView will be released");
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::recording::paste_last,
            commands::recording::cancel_recording,
            commands::models::download_speech_model,
            commands::models::cancel_model_download,
            commands::models::reload_models,
            commands::models::check_models_status,
            commands::models::get_model_state,
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
            commands::system::list_input_devices,
            commands::system::get_paused_state,
            commands::system::open_accessibility_settings,
            commands::system::open_mic_settings,
        ])
        .run(tauri::generate_context!())
    {
        log::error!("Error while running Tauri application: {e}");
    }
}
