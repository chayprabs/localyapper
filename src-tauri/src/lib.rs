mod commands;
mod context;
pub mod controller;
pub mod core;
mod db;
mod error;
mod models;
pub mod platform;
pub mod settings;
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
/// Logging (plan P7.2): the log folder keeps the current file and one
/// rotated file of at most 1 MB each, plus stderr. `info`, or `debug`
/// with LOCALYAPPER_DEBUG=1. Transcripts and clipboard text are never
/// logged.
fn log_plugin<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    use tauri_plugin_log::{RotationStrategy, Target, TargetKind};
    let debug = std::env::var("LOCALYAPPER_DEBUG").is_ok_and(|v| v == "1");
    tauri_plugin_log::Builder::new()
        .clear_targets()
        .target(Target::new(TargetKind::LogDir { file_name: None }))
        .target(Target::new(TargetKind::Stderr))
        .max_file_size(1_000_000)
        .rotation_strategy(RotationStrategy::KeepOne)
        .level(if debug {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        })
        .build()
}

/// The argument the OS passes when it starts the app at login.
const AUTOSTART_ARG: &str = "--autostart";

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
            commands::recording::overlay_drag,
            commands::models::download_speech_model,
            commands::models::cancel_model_download,
            commands::models::model_download_active,
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
            commands::settings::reset_all_settings,
            commands::history::export_history,
            commands::recording::test_dictation,
            commands::hotkeys::update_hotkey,
            commands::hotkeys::reset_hotkeys,
            commands::hotkeys::set_hotkey_probe,
            commands::system::check_permissions,
            commands::system::list_input_devices,
            commands::system::about_info,
            commands::system::diagnostics,
            commands::system::open_releases,
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
    let args: Vec<String> = std::env::args().collect();
    let cli_action = controller::CliAction::from_args(&args);
    // Launched at login (P6.2): never build the settings window.
    let at_login = args.iter().any(|a| a == AUTOSTART_ARG);

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
        .plugin(log_plugin())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![AUTOSTART_ARG]),
        ))
        .setup(move |app| -> Result<(), Box<dyn std::error::Error>> {
            let app_data_dir = data_dir(app.handle()).map_err(|e| {
                log::error!("Failed to resolve app data directory: {e}");
                e
            })?;

            let (conn, migration) = db::open_database(&app_data_dir).map_err(|e| {
                log::error!("Failed to initialize database: {e}");
                Box::new(e) as Box<dyn std::error::Error>
            })?;
            #[cfg(feature = "test-hooks")]
            test_hooks::apply_test_settings(&conn);
            if migration.from_version < 2 && !migration.fresh {
                // Schema version 2 (P5.3): an upgraded install keeps
                // whatever launch-at-login state the OS has now.
                use tauri_plugin_autostart::ManagerExt;
                let on = app.autolaunch().is_enabled().unwrap_or(false);
                if let Err(e) = db::queries::set_setting(&conn, "auto_start", &on.to_string()) {
                    log::warn!("settings: could not record launch-at-login state: {e}");
                }
            }
            let settings_map: HashMap<String, String> =
                db::queries::get_all_settings(&conn).unwrap_or_default();
            let settings_cache = Arc::new(settings::SettingsCache::new(settings_map.clone()));
            let retention: i64 = settings_cache
                .get_or_default("history_retention_days")
                .parse()
                .unwrap_or(0);
            if retention > 0 {
                if let Err(e) = db::queries::prune_history(&conn, retention) {
                    log::warn!("history: could not prune at start: {e}");
                }
            }
            let auto_start_enabled = settings_cache.get_or_default("auto_start") == "true";
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
                downloads: commands::models::Downloads::default(),
                hotkeys: started.hotkeys,
                settings: settings_cache,
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

            // The settings window is not created at start (P6.1): only the
            // first-run wizard needs it, and never at login (P6.2). The
            // tray, the open-app hotkey or a second launch build it.
            let setup_complete = app
                .state::<AppState>()
                .settings
                .get_or_default("setup_complete")
                == "true";
            if !setup_complete && !at_login && cli_action.is_none() {
                show_or_create_main_window(app.handle());
            }
            // Opened by hand after setup, the app starts in the tray with
            // no window. Without a sign of life that looks like it failed
            // to open, so the pill says it is running. Not at login, and
            // not in test builds, which watch what the overlay shows.
            if setup_complete && !at_login && cli_action.is_none() && !cfg!(feature = "test-hooks")
            {
                let state = app.state::<AppState>();
                let hotkey = state.settings.get_or_default("hotkey_record");
                state.controller.send(controller::Msg::Notice(format!(
                    "LocalYapper is running in the tray. Hold {hotkey} to dictate."
                )));
            }

            {
                use tauri_plugin_autostart::ManagerExt;
                let manager = app.autolaunch();
                let is_enabled = manager.is_enabled().unwrap_or(false);
                if auto_start_enabled && !is_enabled {
                    if let Err(e) = manager.enable() {
                        log::warn!("Failed to enable launch at login: {e}");
                    } else {
                        log::info!("Launch at login enabled, following the setting");
                    }
                } else if !auto_start_enabled && is_enabled {
                    if let Err(e) = manager.disable() {
                        log::warn!("Failed to disable launch at login: {e}");
                    } else {
                        log::info!("Launch at login disabled, following the setting");
                    }
                }
            }

            log::info!("LocalYapper initialized. DB at {:?}", app_data_dir);
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }
            match event {
                tauri::WindowEvent::CloseRequested { .. } => {
                    // Allow the close to destroy the WebView and free its
                    // renderer process. The tray + open-app hotkey rebuild
                    // it on demand via show_or_create_main_window.
                    log::info!("Main window closed; WebView will be released");
                }
                tauri::WindowEvent::Destroyed => {
                    // The page cannot clean up after itself once it is
                    // gone. A wizard closed on its hotkey step would leave
                    // the record key in test mode, and a test dictation
                    // would keep recording with nowhere to report.
                    if let Some(state) = window.app_handle().try_state::<AppState>() {
                        state.controller.send(controller::Msg::HotkeyProbe(false));
                        state.controller.send(controller::Msg::TestDictation(false));
                    }
                }
                _ => {}
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
        .build(tauri::generate_context!())
        .map(|app| {
            app.run(|_app, event| {
                // A tray app keeps running with no window open (the
                // settings window is closed and the overlay may have been
                // released after idle). Only an explicit exit (tray Quit)
                // carries an exit code.
                if let tauri::RunEvent::ExitRequested {
                    code: None, api, ..
                } = event
                {
                    api.prevent_exit();
                }
            })
        })
    {
        log::error!("Error while running Tauri application: {e}");
    }
}
