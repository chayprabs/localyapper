use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::Manager;
use tauri_plugin_autostart::ManagerExt;

/// Set up the system tray icon with context menu.
/// Called from lib.rs setup() after AppState is managed and hotkeys are registered.
pub fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // Menu items
    let show_i = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
    let paste_last_i = MenuItem::with_id(app, "paste_last", "Paste Last", true, None::<&str>)?;

    // Autostart toggle — read current state to set initial label
    let autostart_enabled = app
        .autolaunch()
        .is_enabled()
        .unwrap_or(false);
    let autostart_label = if autostart_enabled {
        "\u{2713} Launch at Login"
    } else {
        "Launch at Login"
    };
    let autostart_i = MenuItem::with_id(app, "autostart_toggle", autostart_label, true, None::<&str>)?;

    let sep1 = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let sep3 = PredefinedMenuItem::separator(app)?;
    let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[&show_i, &sep1, &paste_last_i, &sep2, &autostart_i, &sep3, &quit_i],
    )?;

    // Build tray icon
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or("No default window icon found")?;

    TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("LocalYapper")
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.unminimize();
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "paste_last" => {
                let handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    let state = handle.state::<crate::state::AppState>();
                    let text = {
                        let last = state.last_injection.lock().ok();
                        last.and_then(|l| l.clone())
                    };
                    if let Some(t) = text {
                        if !t.is_empty() {
                            let _ = tokio::task::spawn_blocking(move || {
                                crate::injection::injector::inject(&t, false)
                            })
                            .await;
                        }
                    }
                });
            }
            "autostart_toggle" => {
                let manager = app.autolaunch();
                let currently_enabled = manager.is_enabled().unwrap_or(false);
                if currently_enabled {
                    let _ = manager.disable();
                } else {
                    let _ = manager.enable();
                }
                // Update menu item label to reflect new state
                let new_enabled = manager.is_enabled().unwrap_or(false);
                let new_label = if new_enabled {
                    "\u{2713} Launch at Login"
                } else {
                    "Launch at Login"
                };
                let _ = autostart_i.set_text(new_label);
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
