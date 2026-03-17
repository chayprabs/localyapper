use crate::audio::vad;
use crate::models::PipelineResult;
use crate::state::AppState;

/// Start audio capture from the default microphone.
#[tauri::command]
pub async fn start_recording(
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state.recorder.start().map_err(|e| e.to_string())
}

/// Stop capture, apply VAD, return result with audio stats.
/// Transcription is not yet implemented (Phase 3).
#[tauri::command]
pub async fn stop_recording(
    state: tauri::State<'_, AppState>,
    _app_handle: tauri::AppHandle,
) -> Result<PipelineResult, String> {
    let raw_audio = state.recorder.stop().map_err(|e| e.to_string())?;

    let vad_config = vad::default_config();
    let vad_result = vad::apply_vad(&raw_audio, &vad_config);

    let raw_samples = raw_audio.len();
    let trimmed_samples = vad_result.trimmed_audio.len();

    let status = if vad_result.has_speech {
        format!(
            "Audio captured: {}ms speech detected ({} -> {} samples after VAD)",
            vad_result.speech_duration_ms, raw_samples, trimmed_samples
        )
    } else {
        "No speech detected in recording".to_string()
    };

    Ok(PipelineResult {
        raw_text: status.clone(),
        final_text: status,
        duration_ms: vad_result.speech_duration_ms as i64,
        word_count: 0,
    })
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

/// Cancel ongoing recording and discard audio.
#[tauri::command]
pub async fn cancel_recording(
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state.recorder.cancel().map_err(|e| e.to_string())
}
