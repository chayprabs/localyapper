use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Instant;

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tokio::sync::Mutex as TokioMutex;

use crate::commands::recording::{execute_pipeline, save_history_and_learn};
use crate::context::detector;
use crate::db::queries;
use crate::models::PipelineEvent;
use crate::state::AppState;

const MODE_IDLE: u8 = 0;
const MODE_HOLD_RECORDING: u8 = 1;
const MODE_HANDS_FREE: u8 = 2;
const MODE_PROCESSING: u8 = 3;

/// Double-tap detection window in milliseconds.
const DOUBLE_TAP_MS: u128 = 300;

/// Shared hotkey state machine.
struct HotkeyState {
    mode: AtomicU8,
    last_press_time: TokioMutex<Option<Instant>>,
}

/// Initialize global hotkeys. Must be called from Tauri setup() after AppState is managed.
pub fn register_hotkeys(app: &tauri::App) -> Result<(), String> {
    let hotkey_state = Arc::new(HotkeyState {
        mode: AtomicU8::new(MODE_IDLE),
        last_press_time: TokioMutex::new(None),
    });

    // Read hotkey bindings from DB
    let record_hotkey = {
        let state = app.state::<AppState>();
        let conn = state.db.lock().map_err(|e| format!("DB lock error: {e}"))?;
        queries::get_setting(&conn, "hotkey_record").unwrap_or_else(|_| "Alt+Space".to_string())
    };

    let paste_last_hotkey = {
        let state = app.state::<AppState>();
        let conn = state.db.lock().map_err(|e| format!("DB lock error: {e}"))?;
        queries::get_setting(&conn, "hotkey_paste_last")
            .unwrap_or_else(|_| "Alt+Shift+V".to_string())
    };

    // Register the record hotkey (hold-to-talk + double-tap hands-free)
    let state_clone = hotkey_state.clone();
    let app_handle = app.handle().clone();
    app.global_shortcut()
        .on_shortcut(record_hotkey.as_str(), move |_app, _shortcut, event| {
            let state = state_clone.clone();
            let handle = app_handle.clone();
            match event.state {
                ShortcutState::Pressed => {
                    tauri::async_runtime::spawn(async move {
                        handle_record_pressed(handle, state).await;
                    });
                }
                ShortcutState::Released => {
                    tauri::async_runtime::spawn(async move {
                        handle_record_released(handle, state).await;
                    });
                }
            }
        })
        .map_err(|e| format!("Failed to register record hotkey '{record_hotkey}': {e}"))?;

    log::info!("Record hotkey registered: {record_hotkey}");

    // Register paste-last hotkey
    let app_handle = app.handle().clone();
    app.global_shortcut()
        .on_shortcut(paste_last_hotkey.as_str(), move |_app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                let handle = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    handle_paste_last(handle).await;
                });
            }
        })
        .map_err(|e| format!("Failed to register paste-last hotkey '{paste_last_hotkey}': {e}"))?;

    log::info!("Paste-last hotkey registered: {paste_last_hotkey}");

    // NOTE: Escape is registered dynamically when recording starts to avoid
    // capturing all Escape keypresses system-wide.

    Ok(())
}

/// Handle record hotkey pressed — start recording or stop hands-free.
async fn handle_record_pressed(app: AppHandle, state: Arc<HotkeyState>) {
    let current_mode = state.mode.load(Ordering::SeqCst);

    match current_mode {
        MODE_IDLE => {
            // Check for double-tap
            let is_double_tap = {
                let mut last_press = state.last_press_time.lock().await;
                let double = last_press
                    .map(|t| t.elapsed().as_millis() < DOUBLE_TAP_MS)
                    .unwrap_or(false);
                *last_press = Some(Instant::now());
                double
            };

            // Start recording
            let app_state = app.state::<AppState>();
            if let Err(e) = app_state.recorder.start() {
                log::error!("Failed to start recording: {e}");
                emit_pipeline_event(&app, "error", None, None, None, Some(&e.to_string()));
                return;
            }

            if is_double_tap {
                state.mode.store(MODE_HANDS_FREE, Ordering::SeqCst);
                log::info!("Hands-free recording started (double-tap)");
            } else {
                state.mode.store(MODE_HOLD_RECORDING, Ordering::SeqCst);
                log::info!("Hold-to-talk recording started");
            }

            emit_pipeline_event(&app, "listening", None, None, None, None);

            // Register Escape as cancel while recording
            register_cancel_hotkey(&app, state.clone());
        }
        MODE_HANDS_FREE => {
            // Press in hands-free mode = stop and process
            state.mode.store(MODE_PROCESSING, Ordering::SeqCst);
            unregister_cancel_hotkey(&app);
            run_pipeline_and_inject(app, state).await;
        }
        _ => {
            // Ignore presses in other states (processing, hold-recording)
        }
    }
}

/// Handle record hotkey released — stop hold-to-talk recording.
async fn handle_record_released(app: AppHandle, state: Arc<HotkeyState>) {
    let current_mode = state.mode.load(Ordering::SeqCst);

    if current_mode == MODE_HOLD_RECORDING {
        state.mode.store(MODE_PROCESSING, Ordering::SeqCst);
        unregister_cancel_hotkey(&app);
        run_pipeline_and_inject(app, state).await;
    }
    // In hands-free or other modes, release is a no-op
}

/// Run the full pipeline: stop recording -> VAD -> whisper -> correction -> LLM -> inject.
async fn run_pipeline_and_inject(app: AppHandle, hotkey_state: Arc<HotkeyState>) {
    let app_state = app.state::<AppState>();

    // 1. Stop recording, get raw audio
    let raw_audio = match app_state.recorder.stop() {
        Ok(audio) => audio,
        Err(e) => {
            log::error!("Failed to stop recording: {e}");
            emit_pipeline_event(&app, "error", None, None, None, Some(&e.to_string()));
            hotkey_state.mode.store(MODE_IDLE, Ordering::SeqCst);
            return;
        }
    };

    // 2. Emit processing state
    emit_pipeline_event(&app, "processing", None, None, None, None);

    // 3. Run pipeline (VAD -> whisper -> correction -> LLM)
    let result = match execute_pipeline(raw_audio, app_state.inner()).await {
        Ok(r) => r,
        Err(e) => {
            log::error!("Pipeline failed: {e}");
            emit_pipeline_event(&app, "error", None, None, None, Some(&e));
            hotkey_state.mode.store(MODE_IDLE, Ordering::SeqCst);
            return;
        }
    };

    // 4. Check if there's any text to inject
    if result.final_text.is_empty() {
        log::info!("No speech detected, returning to idle");
        emit_pipeline_event(&app, "cancelled", None, None, None, None);
        hotkey_state.mode.store(MODE_IDLE, Ordering::SeqCst);
        return;
    }

    // 5. Emit transcribed state
    emit_pipeline_event(
        &app,
        "transcribed",
        Some(&result.final_text),
        Some(result.duration_ms),
        Some(result.word_count),
        None,
    );

    // 6. Store as last injection
    if let Ok(mut last) = app_state.last_injection.lock() {
        *last = Some(result.final_text.clone());
    }

    // 7. Save to history and run learner
    let app_name = detector::get_focused_window_name();
    save_history_and_learn(app_state.inner(), &result, &app_name);

    // 8. Inject text into focused app
    let text_for_inject = result.final_text.clone();
    match tokio::task::spawn_blocking(move || {
        crate::injection::injector::inject(&text_for_inject, false)
    })
    .await
    {
        Ok(Ok(())) => {
            emit_pipeline_event(
                &app,
                "injected",
                Some(&result.final_text),
                Some(result.duration_ms),
                Some(result.word_count),
                None,
            );
            log::info!("Text injected: {} chars", result.final_text.len());
        }
        Ok(Err(e)) => {
            log::error!("Injection failed: {e}");
            emit_pipeline_event(&app, "error", None, None, None, Some(&e));
        }
        Err(e) => {
            log::error!("Injection task panicked: {e}");
            emit_pipeline_event(
                &app,
                "error",
                None,
                None,
                None,
                Some(&format!("Injection task panicked: {e}")),
            );
        }
    }

    // 9. Return to idle
    hotkey_state.mode.store(MODE_IDLE, Ordering::SeqCst);
}

/// Handle cancel during recording.
async fn handle_cancel(app: AppHandle, hotkey_state: Arc<HotkeyState>) {
    let current_mode = hotkey_state.mode.load(Ordering::SeqCst);
    if current_mode == MODE_HOLD_RECORDING || current_mode == MODE_HANDS_FREE {
        let app_state = app.state::<AppState>();
        if let Err(e) = app_state.recorder.cancel() {
            log::warn!("Cancel recording failed: {e}");
        }
        unregister_cancel_hotkey(&app);
        emit_pipeline_event(&app, "cancelled", None, None, None, None);
        hotkey_state.mode.store(MODE_IDLE, Ordering::SeqCst);
        log::info!("Recording cancelled via Escape");
    }
}

/// Handle paste-last hotkey.
async fn handle_paste_last(app: AppHandle) {
    let app_state = app.state::<AppState>();
    let text = {
        let last = app_state.last_injection.lock().ok();
        last.and_then(|l| l.clone())
    };

    if let Some(t) = text {
        if !t.is_empty() {
            match tokio::task::spawn_blocking(move || {
                crate::injection::injector::inject(&t, false)
            })
            .await
            {
                Ok(Ok(())) => log::info!("Paste-last successful"),
                Ok(Err(e)) => log::error!("Paste-last injection failed: {e}"),
                Err(e) => log::error!("Paste-last task panicked: {e}"),
            }
        }
    }
}

/// Dynamically register Escape as cancel hotkey (only while recording).
fn register_cancel_hotkey(app: &AppHandle, hotkey_state: Arc<HotkeyState>) {
    let handle = app.clone();
    if let Err(e) = app.global_shortcut().on_shortcut("Escape", move |_app, _shortcut, event| {
        if event.state == ShortcutState::Pressed {
            let h = handle.clone();
            let s = hotkey_state.clone();
            tauri::async_runtime::spawn(async move {
                handle_cancel(h, s).await;
            });
        }
    }) {
        log::warn!("Failed to register Escape cancel hotkey: {e}");
    }
}

/// Unregister the Escape cancel hotkey.
fn unregister_cancel_hotkey(app: &AppHandle) {
    if let Err(e) = app.global_shortcut().unregister("Escape") {
        log::warn!("Failed to unregister Escape hotkey: {e}");
    }
}

/// Emit a pipeline state event to the frontend.
fn emit_pipeline_event(
    app: &AppHandle,
    state: &str,
    text: Option<&str>,
    duration_ms: Option<i64>,
    word_count: Option<i64>,
    error: Option<&str>,
) {
    let _ = app.emit(
        "pipeline-state",
        PipelineEvent {
            state: state.to_string(),
            text: text.map(String::from),
            duration_ms,
            word_count,
            error: error.map(String::from),
        },
    );
}
