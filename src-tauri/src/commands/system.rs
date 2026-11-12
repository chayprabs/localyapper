// IPC command handlers -- system utilities and permission checks
use std::sync::atomic::Ordering;

use crate::models::PermissionsStatus;
use crate::state::AppState;
use cpal::traits::HostTrait;
#[cfg(target_os = "linux")]
use std::env;
#[cfg(target_os = "linux")]
use std::path::Path;
use std::process::Command;

/// Returns the name of the currently focused application.
/// Returns system permissions status (mic + accessibility).
/// Input device names, the system default first.
#[tauri::command]
pub async fn list_input_devices() -> Result<Vec<String>, String> {
    tokio::task::spawn_blocking(crate::platform::audio_in::list_input_devices)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn check_permissions() -> Result<PermissionsStatus, String> {
    Ok(PermissionsStatus {
        microphone: microphone_available(),
        accessibility: accessibility_available(),
    })
}

/// Returns whether dictation is currently paused via the tray menu.
#[tauri::command]
pub async fn get_paused_state(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    Ok(state.paused.load(Ordering::SeqCst))
}

/// Opens the OS accessibility settings panel.
#[tauri::command]
pub async fn open_accessibility_settings() -> Result<(), String> {
    open_accessibility_settings_panel()
}

/// Opens the OS microphone settings panel.
#[tauri::command]
pub async fn open_mic_settings() -> Result<(), String> {
    open_microphone_settings_panel()
}

fn microphone_available() -> bool {
    use crate::platform::permissions::{microphone_permission, MicPermission};
    microphone_permission() != MicPermission::Denied
        && cpal::default_host().default_input_device().is_some()
}

/// Ask the OS for microphone access (macOS shows its prompt once).
#[tauri::command]
pub async fn request_microphone_permission() -> Result<(), String> {
    crate::platform::permissions::request_microphone();
    Ok(())
}

/// Check Accessibility and show the macOS prompt when it is off.
/// Returns whether access is granted now.
#[tauri::command]
pub async fn request_accessibility_permission() -> Result<bool, String> {
    Ok(crate::platform::permissions::accessibility_trusted(true))
}

fn accessibility_available() -> bool {
    crate::platform::permissions::accessibility_trusted(false)
}

fn open_accessibility_settings_panel() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        spawn_detached(
            "open",
            &["x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"],
        )
    }

    #[cfg(target_os = "windows")]
    {
        spawn_detached("explorer.exe", &["ms-settings:easeofaccess-keyboard"])
    }

    #[cfg(target_os = "linux")]
    {
        spawn_first_available(&[
            ("gnome-control-center", &["privacy"][..]),
            ("kcmshell6", &["kcm_access"][..]),
            ("systemsettings", &["kcm_access"][..]),
            ("xdg-open", &["settings://privacy"][..]),
        ])
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Err("Opening accessibility settings is not supported on this platform".to_string())
    }
}

fn open_microphone_settings_panel() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        spawn_detached(
            "open",
            &["x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone"],
        )
    }

    #[cfg(target_os = "windows")]
    {
        spawn_detached("explorer.exe", &["ms-settings:privacy-microphone"])
    }

    #[cfg(target_os = "linux")]
    {
        spawn_first_available(&[
            ("gnome-control-center", &["sound"][..]),
            ("kcmshell6", &["kcm_pulseaudio"][..]),
            ("systemsettings", &["kcm_pulseaudio"][..]),
            ("pavucontrol", &[][..]),
            ("xdg-open", &["settings://sound"][..]),
        ])
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Err("Opening microphone settings is not supported on this platform".to_string())
    }
}

#[cfg(target_os = "linux")]
fn spawn_first_available(candidates: &[(&str, &[&str])]) -> Result<(), String> {
    let mut errors = Vec::new();

    for (program, args) in candidates {
        if !command_available(program) {
            continue;
        }

        match spawn_detached(program, args) {
            Ok(()) => return Ok(()),
            Err(e) => errors.push(e),
        }
    }

    if errors.is_empty() {
        Err("No supported system settings application was found".to_string())
    } else {
        Err(errors.join("; "))
    }
}

fn spawn_detached(program: &str, args: &[&str]) -> Result<(), String> {
    Command::new(program)
        .args(args)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Failed to open system settings via {program}: {e}"))
}

#[cfg(target_os = "linux")]
fn command_available(program: &str) -> bool {
    let path_var = match env::var_os("PATH") {
        Some(path) => path,
        None => return false,
    };

    env::split_paths(&path_var).any(|dir| executable_candidate(&dir, program).is_file())
}

#[cfg(target_os = "linux")]
fn executable_candidate(dir: &Path, program: &str) -> std::path::PathBuf {
    dir.join(program)
}
