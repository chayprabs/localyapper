// System tray -- menu, autostart toggle, and close-to-tray behavior
use std::sync::atomic::Ordering;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::GlobalShortcutExt;

use crate::state::AppState;

/// Tauri event emitted when the dictation paused state flips.
const PAUSED_EVENT: &str = "paused-state-changed";

fn emit_paused_state(app: &tauri::AppHandle, paused: bool) {
    if let Err(e) = app.emit(PAUSED_EVENT, paused) {
        log::warn!("Failed to emit {PAUSED_EVENT}: {e}");
    }
}

/// Set up the system tray icon with context menu.
/// Called from lib.rs setup() after AppState is managed and hotkeys are registered.
pub fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // Menu items
    let open_i = MenuItem::with_id(app, "open", "Open LocalYapper", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let pause_i = MenuItem::with_id(app, "pause_toggle", "Pause Dictation", true, None::<&str>)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&open_i, &sep1, &pause_i, &sep2, &quit_i])?;

    // Build tray icon with ID for later retrieval
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or("No default window icon found")?;

    TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("LocalYapper")
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "open" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.unminimize();
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "pause_toggle" => {
                let state = app.state::<AppState>();
                let currently_paused = state.paused.load(Ordering::SeqCst);

                if currently_paused {
                    // Resume: re-register hotkeys
                    state.paused.store(false, Ordering::SeqCst);
                    if let Err(e) = crate::hotkey::manager::reload_hotkeys(app) {
                        log::error!("Failed to re-register hotkeys on resume: {e}");
                    }
                    let _ = pause_i.set_text("Pause Dictation");
                    log::info!("Dictation resumed");
                    emit_paused_state(app, false);
                } else {
                    // Pause: unregister hotkeys
                    state.paused.store(true, Ordering::SeqCst);
                    if let Err(e) = app.global_shortcut().unregister_all() {
                        log::error!("Failed to unregister hotkeys on pause: {e}");
                    }
                    let _ = pause_i.set_text("Resume Dictation");
                    log::info!("Dictation paused");
                    emit_paused_state(app, true);
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        let _ = window.unminimize();
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        })
        .build(app)?;

    log::info!("System tray initialized");
    Ok(())
}

/// Update tray tooltip text. Call from hotkey manager on pipeline state changes.
pub fn update_tray_tooltip(app: &tauri::AppHandle, text: &str) {
    if let Some(tray) = app.tray_by_id("main-tray") {
        let _ = tray.set_tooltip(Some(text));
    }
}
