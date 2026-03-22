use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

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
pub fn register_hotkeys(app: &AppHandle) -> Result<(), String> {
    let hotkey_state = Arc::new(HotkeyState {
        mode: AtomicU8::new(MODE_IDLE),
        last_press_time: TokioMutex::new(None),
    });

    // Read hotkey bindings from DB
    let record_hotkey = {
        let state = app.state::<AppState>();
        let conn = state.db.lock().map_err(|e| format!("DB lock error: {e}"))?;
        queries::get_setting(&conn, "hotkey_record").unwrap_or_else(|_| "Ctrl+Shift+Space".to_string())
    };


    let paste_last_hotkey = {
        let state = app.state::<AppState>();
        let conn = state.db.lock().map_err(|e| format!("DB lock error: {e}"))?;
        queries::get_setting(&conn, "hotkey_paste_last")
            .unwrap_or_else(|_| "Alt+Shift+V".to_string())
    };

    let open_app_hotkey = {
        let state = app.state::<AppState>();
        let conn = state.db.lock().map_err(|e| format!("DB lock error: {e}"))?;
        queries::get_setting(&conn, "hotkey_open_app").unwrap_or_else(|_| "Alt+L".to_string())
    };

    // Register the record hotkey (hold-to-talk + double-tap hands-free)
    let state_clone = hotkey_state.clone();
    let app_handle = app.clone();
    match app.global_shortcut()
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
        }) {
        Ok(()) => log::info!("Record hotkey registered: {record_hotkey}"),
        Err(e) => log::error!("Failed to register record hotkey '{record_hotkey}': {e}"),
    }

    // Register paste-last hotkey
    let app_handle = app.clone();
    match app.global_shortcut()
        .on_shortcut(paste_last_hotkey.as_str(), move |_app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                let handle = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    handle_paste_last(handle).await;
                });
            }
        }) {
        Ok(()) => log::info!("Paste-last hotkey registered: {paste_last_hotkey}"),
        Err(e) => log::error!("Failed to register paste-last hotkey '{paste_last_hotkey}': {e}"),
    }

    // Register open-app hotkey
    let app_handle = app.clone();
    match app.global_shortcut()
        .on_shortcut(open_app_hotkey.as_str(), move |_app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                if let Some(window) = app_handle.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        }) {
        Ok(()) => log::info!("Open-app hotkey registered: {open_app_hotkey}"),
        Err(e) => log::error!("Failed to register open-app hotkey '{open_app_hotkey}': {e}"),
    }

    // NOTE: Escape is registered dynamically when recording starts to avoid
    // capturing all Escape keypresses system-wide.

    Ok(())
}

/// Unregister all global shortcuts and re-register from current DB settings.
pub fn reload_hotkeys(app: &AppHandle) -> Result<(), String> {
    app.global_shortcut()
        .unregister_all()
        .map_err(|e| format!("Failed to unregister shortcuts: {e}"))?;
    register_hotkeys(app)
}

/// Handle record hotkey pressed — start recording or stop hands-free.
async fn handle_record_pressed(app: AppHandle, state: Arc<HotkeyState>) {
    // Check if dictation is paused via tray menu
    {
        let app_state = app.state::<AppState>();
        if app_state.paused.load(Ordering::SeqCst) {
            return;
        }
    }

    let current_mode = state.mode.load(Ordering::SeqCst);

    match current_mode {
        MODE_IDLE => {
            println!("HOTKEY: Press detected");
            // Atomically claim the transition from IDLE — only one press wins
            if state.mode.compare_exchange(MODE_IDLE, MODE_HOLD_RECORDING, Ordering::SeqCst, Ordering::SeqCst).is_err() {
                return; // Another press already transitioned
            }

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
                state.mode.store(MODE_IDLE, Ordering::SeqCst);
                return;
            }
            println!("AUDIO: Capture started");

            if is_double_tap {
                state.mode.store(MODE_HANDS_FREE, Ordering::SeqCst);
                log::info!("Hands-free recording started (double-tap)");
            } else {
                log::info!("Hold-to-talk recording started");
            }

            println!("OVERLAY: Showing listening state");
            emit_pipeline_event(&app, "listening", None, None, None, None);

            // Register Escape as cancel while recording
            register_cancel_hotkey(&app, state.clone());
        }
        MODE_HANDS_FREE => {
            // Atomically transition from hands-free to processing
            if state.mode.compare_exchange(MODE_HANDS_FREE, MODE_PROCESSING, Ordering::SeqCst, Ordering::SeqCst).is_err() {
                return;
            }
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
    // Atomically transition from hold-recording to processing — only one release wins
    if state.mode.compare_exchange(MODE_HOLD_RECORDING, MODE_PROCESSING, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
        println!("HOTKEY: Release detected");
        unregister_cancel_hotkey(&app);
        run_pipeline_and_inject(app, state).await;
    }
    // In hands-free or other modes, release is a no-op
}

/// Run the full pipeline: stop recording -> VAD -> whisper -> correction -> LLM -> inject.
async fn run_pipeline_and_inject(app: AppHandle, hotkey_state: Arc<HotkeyState>) {
    let pipeline_start = Instant::now();
    log::info!("run_pipeline_and_inject: starting");
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

    let audio_duration_ms = (raw_audio.len() as f64 / 16000.0 * 1000.0) as i64;
    println!("PIPELINE: Starting for {} samples ({:.1}s audio)", raw_audio.len(), raw_audio.len() as f64 / 16000.0);
    log::info!("Recording stopped. {} samples captured ({}ms audio)", raw_audio.len(), audio_duration_ms);

    // 2. Emit processing state with audio duration for frontend countdown
    emit_pipeline_event(&app, "processing", None, Some(audio_duration_ms), None, None);

    // 3. Run pipeline (VAD -> whisper -> correction -> LLM) with 30s safety timeout
    log::info!("Running pipeline (VAD -> whisper -> correction -> LLM)...");
    let result = match tokio::time::timeout(
        Duration::from_secs(30),
        execute_pipeline(raw_audio, app_state.inner(), Some(&app)),
    )
    .await
    {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => {
            log::error!("Pipeline failed: {e}");
            emit_pipeline_event(&app, "error", None, None, None, Some(&e));
            hotkey_state.mode.store(MODE_IDLE, Ordering::SeqCst);
            return;
        }
        Err(_timeout) => {
            log::error!("Pipeline timed out after 30s");
            emit_pipeline_event(&app, "error", None, None, None, Some("Pipeline timed out (30s)"));
            hotkey_state.mode.store(MODE_IDLE, Ordering::SeqCst);
            return;
        }
    };

    log::info!("Pipeline complete: {} chars, {} words", result.final_text.len(), result.word_count);

    // 4. Check if there's any text to inject
    if result.final_text.is_empty() {
        println!("PIPELINE: No speech detected");
        log::info!("No speech detected, returning to idle");
        emit_pipeline_event(&app, "no-speech", Some("No speech detected"), None, None, None);
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
    println!("HISTORY: Saved entry");

    // 8. Inject text into focused app
    println!("INJECT: Injecting into [{}]", app_name);
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
            println!("PIPELINE: Complete in {}ms", pipeline_start.elapsed().as_millis());
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

    // 9. Wait for transcribed overlay to dismiss (3s), then return to idle.
    // Keeps MODE_PROCESSING active so new recordings are blocked during display.
    tokio::time::sleep(Duration::from_secs(3)).await;
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

/// Emit a pipeline state event to the frontend and update tray tooltip.
fn emit_pipeline_event(
    app: &AppHandle,
    state: &str,
    text: Option<&str>,
    duration_ms: Option<i64>,
    word_count: Option<i64>,
    error: Option<&str>,
) {
    if let Err(e) = app.emit(
        "pipeline-state",
        PipelineEvent {
            state: state.to_string(),
            text: text.map(String::from),
            duration_ms,
            word_count,
            error: error.map(String::from),
        },
    ) {
        log::error!("Failed to emit pipeline-state '{state}': {e}");
    }

    // Update tray tooltip to reflect pipeline state
    let tooltip = match state {
        "listening" => "LocalYapper \u{2014} Recording...",
        "processing" => "LocalYapper \u{2014} Processing...",
        _ => "LocalYapper",
    };
    crate::tray::update_tray_tooltip(app, tooltip);
}
