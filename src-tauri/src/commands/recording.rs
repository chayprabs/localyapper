use std::sync::Arc;

use crate::audio::vad;
use crate::context::detector;
use crate::correction::learner;
use crate::db::queries;
use crate::llm::engine::LlmEngine;
use crate::llm::prompt;
use crate::models::PipelineResult;
use crate::state::AppState;
use crate::stt::whisper::WhisperEngine;

/// Run the full voice pipeline: VAD -> Whisper -> Correction -> LLM.
/// Does NOT inject or save to history — caller decides.
pub(crate) async fn execute_pipeline(
    raw_audio: Vec<f32>,
    state: &AppState,
) -> Result<PipelineResult, String> {
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
        .lock()
        .map_err(|e| format!("Whisper lock error: {e}"))?
        .as_ref()
        .ok_or_else(|| "Whisper model not loaded. Download it via the wizard or Models page.".to_string())?
        .clone();

    let trimmed_audio = vad_result.trimmed_audio;

    let raw_text = tokio::task::spawn_blocking(move || {
        whisper.transcribe(&trimmed_audio)
    })
    .await
    .map_err(|e| format!("Transcription task failed: {e}"))?
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
        let llm_available = state.llm.lock()
            .map(|g| g.is_some())
            .unwrap_or(false);
        let should_run_llm = llm_available && {
            let db = state.db.lock().map_err(|e| format!("DB lock error: {e}"))?;
            match queries::get_active_mode(&db) {
                Ok(mode) => !mode.skip_llm,
                Err(_) => false,
            }
        };

        if should_run_llm {
            let llm: Arc<LlmEngine> = state.llm.lock()
                .map_err(|e| format!("LLM lock error: {e}"))?
                .as_ref().expect("checked above").clone();

            let system_prompt = {
                let db = state.db.lock().map_err(|e| format!("DB lock error: {e}"))?;
                queries::get_active_mode(&db)
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

/// Save a pipeline result to history and run the correction learner asynchronously.
pub(crate) fn save_history_and_learn(state: &AppState, result: &PipelineResult, app_name: &str) {
    if result.final_text.is_empty() {
        return;
    }

    // Save to history
    let mode_id = {
        let conn = state.db.lock().ok();
        conn.and_then(|c| queries::get_active_mode(&c).ok().map(|m| m.id))
    };

    let entry = crate::models::HistoryEntry {
        id: uuid::Uuid::new_v4().to_string(),
        raw_text: result.raw_text.clone(),
        final_text: result.final_text.clone(),
        app_name: Some(app_name.to_string()),
        mode_id,
        duration_ms: Some(result.duration_ms),
        word_count: Some(result.word_count),
        created_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    };

    if let Ok(conn) = state.db.lock() {
        if let Err(e) = queries::insert_history(&conn, &entry) {
            log::warn!("Failed to save history: {e}");
        }
    }

    // Run correction learner
    let diffs = learner::compute_diffs(&result.raw_text, &result.final_text);
    if !diffs.is_empty() {
        if let Ok(conn) = state.db.lock() {
            match learner::learn_and_refresh(&conn, &diffs, &state.correction_engine) {
                Ok(count) => log::info!("Learned {count} corrections from pipeline"),
                Err(e) => log::warn!("Correction learning failed: {e}"),
            }
        }
    }
}

/// Start audio capture from the default microphone.
#[tauri::command]
pub async fn start_recording(
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state.recorder.start().map_err(|e| e.to_string())
}

/// Stop capture, run pipeline, save history, return result.
#[tauri::command]
pub async fn stop_recording(
    state: tauri::State<'_, AppState>,
    _app_handle: tauri::AppHandle,
) -> Result<PipelineResult, String> {
    let raw_audio = state.recorder.stop().map_err(|e| e.to_string())?;
    let result = execute_pipeline(raw_audio, state.inner()).await?;

    if !result.final_text.is_empty() {
        let app_name = detector::get_focused_window_name();
        save_history_and_learn(state.inner(), &result, &app_name);
    }

    Ok(result)
}

/// Run pipeline on provided audio data.
#[tauri::command]
pub async fn run_pipeline(
    audio: Vec<f32>,
    state: tauri::State<'_, AppState>,
) -> Result<PipelineResult, String> {
    execute_pipeline(audio, state.inner()).await
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
