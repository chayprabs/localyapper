mod commands;
mod context;
pub mod controller;
pub mod core;
mod db;
mod error;
mod models;
pub mod platform;
mod state;
#[cfg(feature = "test-hooks")]
mod test_hooks;
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

/// The app's data folder. Test builds can point it elsewhere with
/// LOCALYAPPER_DATA_DIR so smoke tests never touch a real profile.
pub(crate) fn data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    #[cfg(feature = "test-hooks")]
    if let Some(dir) = std::env::var_os("LOCALYAPPER_DATA_DIR") {
        return Ok(PathBuf::from(dir));
    }
    app.path().app_data_dir().map_err(|e| e.to_string())
}

/// Directory holding downloaded speech models.
pub(crate) fn models_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    data_dir(app).map(|d| d.join("models"))
}

/// Resolve the `speech_model` setting to files on disk.
pub(crate) fn resolve_model_spec(
    app: &tauri::AppHandle,
) -> Result<core::stt::manager::ModelSpec, String> {
    let state = app.state::<AppState>();
    let id = commands::models::active_model_id(state.inner())?;
    core::stt::catalog::spec(&models_dir(app)?, &id, commands::models::manifest()?)
}

/// The Silero VAD model bundled in the app's resources. When it is
/// missing the segmenter falls back to the energy detector.
fn vad_model_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    use tauri::path::BaseDirectory;
    let p = app
        .path()
        .resolve("resources/silero_vad.onnx", BaseDirectory::Resource)
        .ok()?;
    if p.is_file() {
        Some(p)
    } else {
        log::warn!("vad: bundled model missing at {}", p.display());
        None
    }
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

/// The IPC command list. Test builds add the overlay test hook.
macro_rules! app_commands {
    ($($extra:path),* $(,)?) => {
        tauri::generate_handler![
            commands::recording::paste_last,
            commands::recording::cancel_recording,
            commands::recording::overlay_copy,
            commands::recording::overlay_dismiss,
            commands::recording::overlay_current_view,
            commands::models::download_speech_model,
            commands::models::cancel_model_download,
            commands::models::reload_models,
            commands::models::check_models_status,
            commands::models::list_models,
            commands::models::download_model,
            commands::models::use_model,
            commands::models::delete_model,
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
            commands::hotkeys::set_hotkey_probe,
            commands::system::check_permissions,
            commands::system::list_input_devices,
            commands::system::request_microphone_permission,
            commands::system::request_accessibility_permission,
            commands::system::get_paused_state,
            commands::system::open_accessibility_settings,
            commands::system::open_mic_settings,
            $($extra,)*
        ]
    };
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cli_action = controller::CliAction::from_args(&std::env::args().collect::<Vec<_>>());

    // Registered first: a second launch hands its arguments to this
    // instance and exits.
    #[allow(unused_mut)]
    let mut builder =
        tauri::Builder::default().plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            match controller::CliAction::from_args(&args) {
                Some(action) => app
                    .state::<AppState>()
                    .controller
                    .send(controller::Msg::Cli(action)),
                None => show_or_create_main_window(app),
            }
        }));
    // The overlay becomes a non-activating NSPanel on macOS.
    #[cfg(target_os = "macos")]
    {
        builder = builder.plugin(tauri_nspanel::init());
    }

    if let Err(e) = builder
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(move |app| -> Result<(), Box<dyn std::error::Error>> {
            let app_data_dir = data_dir(app.handle()).map_err(|e| {
                log::error!("Failed to resolve app data directory: {e}");
                e
            })?;

            let conn = db::open_database(&app_data_dir).map_err(|e| {
                log::error!("Failed to initialize database: {e}");
                Box::new(e) as Box<dyn std::error::Error>
            })?;
            #[cfg(feature = "test-hooks")]
            test_hooks::apply_test_settings(&conn);
            let settings_map: HashMap<String, String> =
                db::queries::get_all_settings(&conn).unwrap_or_default();
            let auto_start_enabled = settings_map
                .get("auto_start")
                .map(|v| v.eq_ignore_ascii_case("true"))
                .unwrap_or(true);
            commands::models::migrate_legacy_install(&models_dir(app.handle())?);

            let controller_settings = controller::ControllerSettings::from_map(&settings_map);
            // The overlay window is created here, hidden, so a key press
            // never waits for a WebView to start.
            if let Err(e) = platform::overlay::webview::build(app.handle()) {
                log::error!("overlay: could not create the window: {e}");
            }
            let started = controller::start(controller::ControllerParts {
                app: app.handle().clone(),
                overlay: Arc::new(platform::overlay::webview::WebviewOverlay::new(
                    app.handle().clone(),
                    controller_settings.overlay,
                )),
                settings: controller_settings,
                resolve_model: Box::new(resolve_model_spec),
                vad_model: vad_model_path(app.handle()),
                hotkeys: {
                    let handle = app.handle().clone();
                    Box::new(move |ctl| platform::hotkey::create_backend(&handle, ctl))
                },
            })?;

            app.manage(AppState {
                db: Arc::new(Mutex::new(conn)),
                controller: started.handle,
                models: started.manager,
                paused: started.paused,
                last_injection: started.last_injection,
                download_cancel: Arc::new(AtomicBool::new(false)),
                hotkeys: started.hotkeys,
            });

            #[cfg(feature = "test-hooks")]
            test_hooks::start(
                app.handle().clone(),
                app.state::<AppState>().controller.clone(),
            );

            if let Some(action) = cli_action {
                app.state::<AppState>()
                    .controller
                    .send(controller::Msg::Cli(action));
            }

            match platform::hotkey::apply_settings(app.handle()) {
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
        .invoke_handler({
            #[cfg(feature = "test-hooks")]
            {
                app_commands!(test_hooks::test_overlay_seen)
            }
            #[cfg(not(feature = "test-hooks"))]
            {
                app_commands!()
            }
        })
        .run(tauri::generate_context!())
    {
        log::error!("Error while running Tauri application: {e}");
    }
}
