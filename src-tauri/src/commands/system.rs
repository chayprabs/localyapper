use crate::models::PermissionsStatus;

/// Returns the name of the currently focused application.
#[tauri::command]
pub async fn get_focused_app() -> Result<String, String> {
    Ok("Unknown".to_string())
}

/// Checks for app updates via GitHub releases API.
#[tauri::command]
pub async fn check_update() -> Result<Option<String>, String> {
    Ok(None)
}

/// Returns system permissions status (mic + accessibility).
#[tauri::command]
pub async fn check_permissions() -> Result<PermissionsStatus, String> {
    Ok(PermissionsStatus {
        microphone: false,
        accessibility: false,
    })
}

/// Opens the OS accessibility settings panel.
#[tauri::command]
pub async fn open_accessibility_settings() -> Result<(), String> {
    Err("Not implemented: Phase 4".to_string())
}

/// Opens the OS microphone settings panel.
#[tauri::command]
pub async fn open_mic_settings() -> Result<(), String> {
    Err("Not implemented: Phase 2".to_string())
}
