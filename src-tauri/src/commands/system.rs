// IPC command handlers -- system utilities and permission checks
use std::sync::atomic::Ordering;

use crate::context::detector;
#[cfg(target_os = "linux")]
use crate::injection::platform::{self, Platform};
use crate::models::PermissionsStatus;
use crate::state::AppState;
use cpal::traits::HostTrait;
#[cfg(target_os = "linux")]
use std::env;
#[cfg(target_os = "linux")]
use std::path::Path;
use std::process::Command;

/// Returns the name of the currently focused application.
#[tauri::command]
pub async fn get_focused_app() -> Result<String, String> {
    Ok(detector::get_focused_window_name())
}

/// Returns system permissions status (mic + accessibility).
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
    cpal::default_host().default_input_device().is_some()
}

fn accessibility_available() -> bool {
    #[cfg(target_os = "macos")]
    {
        macos_accessibility_trusted()
    }

    #[cfg(target_os = "windows")]
    {
        true
    }

    #[cfg(target_os = "linux")]
    {
        match platform::detect() {
            Platform::LinuxWayland => {
                command_available("wl-copy")
                    && command_available("wl-paste")
                    && command_available("wtype")
            }
            Platform::LinuxX11 => command_available("xclip") && command_available("xdotool"),
            Platform::Windows | Platform::MacOS => true,
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        false
    }
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

#[cfg(target_os = "macos")]
fn macos_accessibility_trusted() -> bool {
    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> std::os::raw::c_uchar;
    }

    // SAFETY: AXIsProcessTrusted reads the current process TCC trust state and
    // does not require pointers or owned resources.
    unsafe { AXIsProcessTrusted() != 0 }
}
