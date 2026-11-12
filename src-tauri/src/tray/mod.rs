// System tray -- menu, autostart toggle, and close-to-tray behavior
use std::sync::atomic::Ordering;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::Manager;

use crate::controller::Msg;
use crate::core::session::Tray;
use crate::state::AppState;

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
                crate::show_or_create_main_window(app);
            }
            "pause_toggle" => {
                let state = app.state::<AppState>();
                let pause = !state.paused.load(Ordering::SeqCst);
                // The controller aborts any recording and mirrors the flag.
                state.paused.store(pause, Ordering::SeqCst);
                state.controller.send(Msg::SetPaused(pause));
                if let Err(e) = crate::platform::hotkey::apply_settings(app) {
                    log::error!("Failed to update hotkeys after pause toggle: {e}");
                }
                let _ = pause_i.set_text(if pause {
                    "Resume Dictation"
                } else {
                    "Pause Dictation"
                });
                log::info!("Dictation {}", if pause { "paused" } else { "resumed" });
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
                match app.get_webview_window("main") {
                    Some(window) if window.is_visible().unwrap_or(false) => {
                        let _ = window.hide();
                    }
                    _ => crate::show_or_create_main_window(app),
                }
            }
        })
        .build(app)?;

    log::info!("System tray initialized");
    Ok(())
}

/// Update tray tooltip text. Call from hotkey manager on pipeline state changes.
/// Reflect the dictation state in the tray. Icons per state arrive in P3.8.
pub fn set_state(app: &tauri::AppHandle, state: Tray) {
    let tip = match state {
        Tray::Idle => "LocalYapper",
        Tray::Recording => "LocalYapper - recording",
        Tray::Processing => "LocalYapper - transcribing",
    };
    if let Some(tray) = app.tray_by_id("main-tray") {
        let _ = tray.set_tooltip(Some(tip));
    }
}
