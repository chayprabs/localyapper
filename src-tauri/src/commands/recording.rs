use crate::models::PipelineResult;
use crate::state::AppState;

/// Start audio capture (begins pre-roll buffer).
#[tauri::command]
pub async fn start_recording(
    _state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    Err("Not implemented: Phase 2".to_string())
}

/// Stop capture, run full pipeline, return result.
#[tauri::command]
pub async fn stop_recording(
    _state: tauri::State<'_, AppState>,
    _app_handle: tauri::AppHandle,
) -> Result<PipelineResult, String> {
    Err("Not implemented: Phase 2".to_string())
}

/// Run pipeline on provided audio data.
#[tauri::command]
pub async fn run_pipeline(
    _audio: Vec<f32>,
    _state: tauri::State<'_, AppState>,
) -> Result<PipelineResult, String> {
    Err("Not implemented: Phase 7".to_string())
}

/// Inject text into the currently focused application.
#[tauri::command]
pub async fn inject_text(
    _text: String,
    _hold_shift: bool,
) -> Result<(), String> {
    Err("Not implemented: Phase 4".to_string())
}

/// Re-inject the last dictated text.
#[tauri::command]
pub async fn paste_last(
    _state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    Err("Not implemented: Phase 4".to_string())
}

/// Cancel ongoing recording/processing.
#[tauri::command]
pub async fn cancel_recording(
    _state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    Err("Not implemented: Phase 2".to_string())
}
