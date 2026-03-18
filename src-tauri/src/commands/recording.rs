use std::sync::Arc;

use crate::audio::vad;
use crate::context::detector;
use crate::llm::engine::LlmEngine;
use crate::llm::prompt;
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

    let corrected_text = state.correction_engine.apply(&raw_text)
        .unwrap_or_else(|_| raw_text.clone());

    // LLM cleanup step — skipped if no model or mode says skip_llm
    let final_text = {
        let should_run_llm = state.llm.is_some() && {
            let db = state.db.lock().map_err(|e| format!("DB lock error: {e}"))?;
            match crate::db::queries::get_active_mode(&db) {
                Ok(mode) => !mode.skip_llm,
                Err(_) => false,
            }
        };

        if should_run_llm {
            let llm: Arc<LlmEngine> = state.llm.as_ref().expect("checked above").clone();

            // Get active mode system prompt
            let system_prompt = {
                let db = state.db.lock().map_err(|e| format!("DB lock error: {e}"))?;
                crate::db::queries::get_active_mode(&db)
                    .map(|m| m.system_prompt)
                    .unwrap_or_default()
            };

            let app_name = detector::get_focused_window_name();
            let llm_prompt = prompt::build_prompt(&system_prompt, &corrected_text, &app_name);
            let max_tokens = (corrected_text.len() as u32 * 2).clamp(128, 512);

            match tokio::task::spawn_blocking(move || {
                llm.generate(&llm_prompt, max_tokens)
            })
            .await
            {
                Ok(Ok(llm_output)) if !llm_output.is_empty() => {
                    log::info!("LLM cleanup applied ({} -> {} chars)", corrected_text.len(), llm_output.len());
                    llm_output
                }
                Ok(Err(e)) => {
                    log::warn!("LLM generation failed, using corrected text: {e}");
                    corrected_text
                }
                Err(e) => {
                    log::warn!("LLM task panicked, using corrected text: {e}");
                    corrected_text
                }
                _ => corrected_text,
            }
        } else {
            corrected_text
        }
    };

    Ok(PipelineResult {
        final_text,
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
    text: String,
    hold_shift: bool,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    if text.is_empty() {
        return Ok(());
    }

    // Store as last injection for paste_last
    {
        let mut last = state
            .last_injection
            .lock()
            .map_err(|e| format!("Lock error: {e}"))?;
        *last = Some(text.clone());
    }

    let t = text;
    let s = hold_shift;
    tokio::task::spawn_blocking(move || {
        crate::injection::injector::inject(&t, s)
    })
    .await
    .map_err(|e| format!("Injection task failed: {e}"))?
}

/// Re-inject the last dictated text.
#[tauri::command]
pub async fn paste_last(
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let text = {
        let last = state
            .last_injection
            .lock()
            .map_err(|e| format!("Lock error: {e}"))?;
        last.clone()
    };

    match text {
        Some(t) if !t.is_empty() => {
            tokio::task::spawn_blocking(move || {
                crate::injection::injector::inject(&t, false)
            })
            .await
            .map_err(|e| format!("Injection task failed: {e}"))?
        }
        _ => Err("No previous injection to paste".to_string()),
    }
}

/// Cancel ongoing recording and discard audio.
#[tauri::command]
pub async fn cancel_recording(
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state.recorder.cancel().map_err(|e| e.to_string())
}
