use std::sync::Arc;

use crate::audio::vad;
use crate::models::PipelineResult;
use crate::state::AppState;
use crate::stt::whisper::WhisperEngine;

/// Start audio capture from the default microphone.
#[tauri::command]
pub async fn start_recording(
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state.recorder.start().map_err(|e| e.to_string())
}

/// Stop capture, apply VAD, transcribe speech via Whisper, return result.
#[tauri::command]
pub async fn stop_recording(
    state: tauri::State<'_, AppState>,
    _app_handle: tauri::AppHandle,
) -> Result<PipelineResult, String> {
    let raw_audio = state.recorder.stop().map_err(|e| e.to_string())?;

    let vad_config = vad::default_config();
    let vad_result = vad::apply_vad(&raw_audio, &vad_config);

    if !vad_result.has_speech {
        return Ok(PipelineResult {
            raw_text: String::new(),
            final_text: String::new(),
            duration_ms: 0,
            word_count: 0,
        });
    }

    let whisper: Arc<WhisperEngine> = state
        .whisper
        .as_ref()
        .ok_or_else(|| "Whisper model not loaded. Place ggml-tiny.en.bin in resources/.".to_string())?
        .clone();

    let trimmed_audio = vad_result.trimmed_audio;

    let raw_text = tokio::task::spawn_blocking(move || {
        whisper.transcribe(&trimmed_audio)
    })
    .await
    .map_err(|e| format!("Transcription task failed: {}", e))?
    .map_err(|e| e.to_string())?;

    let word_count = if raw_text.is_empty() {
        0
    } else {
        raw_text.split_whitespace().count() as i64
    };

    // final_text = raw_text for now (correction + LLM are later phases)
    Ok(PipelineResult {
        final_text: raw_text.clone(),
        raw_text,
        duration_ms: vad_result.speech_duration_ms as i64,
        word_count,
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
