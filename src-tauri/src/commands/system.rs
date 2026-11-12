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

/// What the About section shows.
#[derive(serde::Serialize)]
pub struct AboutInfo {
    pub version: String,
    pub data_dir: String,
    pub log_dir: String,
}

#[tauri::command]
pub fn about_info(app: tauri::AppHandle) -> Result<AboutInfo, String> {
    use tauri::Manager;
    let data_dir = crate::data_dir(&app).map_err(|e| e.to_string())?;
    let log_dir = app.path().app_log_dir().map_err(|e| e.to_string())?;
    Ok(AboutInfo {
        version: app.package_info().version.to_string(),
        data_dir: data_dir.display().to_string(),
        log_dir: log_dir.display().to_string(),
    })
}

/// Facts that help with a bug report: OS, CPU, RAM, the speech model and
/// the last dictation's stage timings. Never any dictated text.
pub(crate) fn diagnostics_text(
    version: &str,
    model: &str,
    loaded: Option<&str>,
    timings: Option<&crate::controller::StageTimings>,
) -> String {
    let gib = |b: u64| format!("{:.1} GiB", b as f64 / (1u64 << 30) as f64);
    let mut lines = vec![
        format!("LocalYapper {version}"),
        format!(
            "OS: {} {} ({})",
            std::env::consts::OS,
            os_version(),
            std::env::consts::ARCH
        ),
        format!(
            "CPU: {} logical, {} physical",
            std::thread::available_parallelism().map_or(0, |n| n.get()),
            crate::platform::memory::physical_cores()
        ),
        format!(
            "RAM: {}",
            crate::platform::memory::total_ram_bytes().map_or("unknown".into(), gib)
        ),
        format!(
            "Speech model: {model} ({})",
            loaded.map_or("not loaded".into(), |l| format!("{l} loaded"))
        ),
    ];
    match timings {
        Some(t) => lines.push(format!(
            "Last dictation: capture_start {} ms, model_ready {} ms, segments {}, audio {} ms, decode {} ms, tail_decode {} ms, inject {} ms, release_to_text {} ms",
            t.capture_start_ms,
            t.model_ready_ms,
            t.segments,
            t.audio_ms,
            t.decode_ms,
            t.tail_decode_ms,
            t.inject_ms,
            t.release_to_text_ms
        )),
        None => lines.push("Last dictation: none since start".into()),
    }
    lines.join("\n")
}

fn os_version() -> String {
    #[cfg(target_os = "linux")]
    {
        let pretty = std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|s| {
                s.lines().find_map(|l| {
                    l.strip_prefix("PRETTY_NAME=")
                        .map(|v| v.trim_matches('"').to_string())
                })
            });
        let session = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
        format!("{} {session}", pretty.unwrap_or_default())
            .trim()
            .to_string()
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default()
    }
    #[cfg(windows)]
    {
        Command::new("cmd")
            .args(["/C", "ver"])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    {
        String::new()
    }
}

#[tauri::command]
pub async fn diagnostics(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let snapshot = state.controller.query().await;
    let model = crate::commands::models::active_model_id(&state).unwrap_or_else(|e| e);
    Ok(diagnostics_text(
        &app.package_info().version.to_string(),
        &model,
        snapshot.as_ref().and_then(|s| s.model_loaded.as_deref()),
        snapshot.as_ref().and_then(|s| s.last_timings.as_ref()),
    ))
}

#[cfg(test)]
mod diagnostics_tests {
    use super::*;

    #[test]
    fn diagnostics_has_facts_and_no_text() {
        let t = crate::controller::StageTimings {
            segments: 2,
            release_to_text_ms: 180,
            ..Default::default()
        };
        let d = diagnostics_text("2.0.0", "parakeet-110m", Some("parakeet-110m"), Some(&t));
        assert!(d.starts_with("LocalYapper 2.0.0"));
        assert!(d.contains("RAM: "));
        assert!(d.contains("Speech model: parakeet-110m (parakeet-110m loaded)"));
        assert!(d.contains("release_to_text 180 ms"));
        let none = diagnostics_text("2.0.0", "auto", None, None);
        assert!(none.contains("not loaded") && none.contains("none since start"));
    }
}

/// Where releases are published (plan P7.4: no auto-updater).
pub const RELEASES_URL: &str = "https://github.com/chayprabs/localyapper/releases";

/// Open the releases page in the default browser.
#[tauri::command]
pub fn open_releases(app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(RELEASES_URL, None::<&str>)
        .map_err(|e| e.to_string())
}
