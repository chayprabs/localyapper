// IPC command handlers -- recording pipeline and text injection
use std::time::Instant;

use crate::audio::vad;
use crate::context::detector;
use crate::db::queries;
use crate::models::PipelineResult;
use crate::state::AppState;

/// Run the full voice pipeline: VAD -> STT.
/// Does NOT inject or save to history — caller decides.
/// Uses the preloaded speech engine when available and falls back to on-demand load.
pub(crate) async fn execute_pipeline(
    raw_audio: Vec<f32>,
    state: &AppState,
    app_handle: &tauri::AppHandle,
) -> Result<PipelineResult, String> {
    // Run VAD synchronously — extract Silero ref briefly, then drop the lock
    let vad_result = {
        let silero_guard = state.vad.lock().ok();
        let silero_ref = silero_guard.as_ref().and_then(|g| g.as_ref());
        vad::apply_vad(&raw_audio, silero_ref)
        // MutexGuard dropped here before any .await
    };

    if !vad_result.has_speech {
        log::info!("STT: No speech detected in audio");
        return Ok(PipelineResult {
            raw_text: String::new(),
            final_text: String::new(),
            duration_ms: 0,
            word_count: 0,
        });
    }

    let trimmed_audio = vad_result.trimmed_audio;
    let duration_ms = vad_result.speech_duration_ms as i64;
    drop(raw_audio);

    let whisper = crate::commands::models::ensure_speech_model_loaded(app_handle, state).await?;

    log::info!("STT: Transcribing...");
    let stt_start = Instant::now();
    let raw_text = tokio::task::spawn_blocking(move || whisper.transcribe(&trimmed_audio))
        .await
        .map_err(|e| format!("Transcription task failed: {e}"))?
        .map_err(|e| e.to_string())?;
    log::info!(
        "STT: Result received: {} chars ({}ms)",
        raw_text.len(),
        stt_start.elapsed().as_millis()
    );

    let word_count = if raw_text.is_empty() {
        0
    } else {
        raw_text.split_whitespace().count() as i64
    };

    let final_text = raw_text.clone();

    Ok(PipelineResult {
        final_text,
        raw_text,
        duration_ms,
        word_count,
    })
}

/// Save a pipeline result to history.
pub(crate) fn save_history_entry(state: &AppState, result: &PipelineResult, app_name: &str) {
    if result.final_text.is_empty() {
        return;
    }

    let entry = crate::models::HistoryEntry {
        id: uuid::Uuid::new_v4().to_string(),
        raw_text: result.raw_text.clone(),
        final_text: result.final_text.clone(),
        app_name: Some(app_name.to_string()),
        duration_ms: Some(result.duration_ms),
        word_count: Some(result.word_count),
        created_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    };

    if let Ok(conn) = state.db.lock() {
        if let Err(e) = queries::insert_history(&conn, &entry) {
            log::warn!("Failed to save history: {e}");
        }
    }
}

/// Start audio capture from the default microphone.
#[tauri::command]
pub async fn start_recording(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.recorder.start().map_err(|e| e.to_string())
}

/// Stop capture, run pipeline, save history, return result.
#[tauri::command]
pub async fn stop_recording(
    state: tauri::State<'_, AppState>,
    _app_handle: tauri::AppHandle,
) -> Result<PipelineResult, String> {
    let raw_audio = state.recorder.stop().map_err(|e| e.to_string())?;
    let result = execute_pipeline(raw_audio, state.inner(), &_app_handle).await?;

    if !result.final_text.is_empty() {
        let app_name = detector::get_focused_window_name();
        save_history_entry(state.inner(), &result, &app_name);
    }

    Ok(result)
}

/// Run pipeline on provided audio data.
#[tauri::command]
pub async fn run_pipeline(
    audio: Vec<f32>,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<PipelineResult, String> {
    execute_pipeline(audio, state.inner(), &app_handle).await
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
    tokio::task::spawn_blocking(move || crate::injection::injector::inject(&t, s))
        .await
        .map_err(|e| format!("Injection task failed: {e}"))?
}

/// Re-inject the last dictated text.
#[tauri::command]
pub async fn paste_last(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let text = {
        let last = state
            .last_injection
            .lock()
            .map_err(|e| format!("Lock error: {e}"))?;
        last.clone()
    };

    match text {
        Some(t) if !t.is_empty() => {
            tokio::task::spawn_blocking(move || crate::injection::injector::inject(&t, false))
                .await
                .map_err(|e| format!("Injection task failed: {e}"))?
        }
        _ => Err("No previous injection to paste".to_string()),
    }
}

/// Cancel ongoing recording and discard audio.
#[tauri::command]
pub async fn cancel_recording(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.recorder.cancel().map_err(|e| e.to_string())
}
