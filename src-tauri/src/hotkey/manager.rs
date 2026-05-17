// Hotkey manager -- global shortcut registration and state machine
use std::sync::atomic::{AtomicI64, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::commands::recording::{execute_pipeline, save_history_entry};
use crate::context::detector;
use crate::db::queries;
use crate::models::PipelineEvent;
use crate::state::AppState;

/// State machine mode: no active recording, ready for input.
const MODE_IDLE: u8 = 0;
/// State machine mode: key is held down, recording in progress.
const MODE_HOLD_RECORDING: u8 = 1;
/// State machine mode: hands-free toggle recording is active.
const MODE_HANDS_FREE: u8 = 2;
/// State machine mode: recording stopped, pipeline is running.
const MODE_PROCESSING: u8 = 3;
/// State machine mode: a quick tap was released; waiting briefly for a
/// second tap that would promote the session into hands-free.
const MODE_TAP_PENDING: u8 = 4;

/// Recording duration where the overlay should switch to the final warning state.
const RECORDING_WARNING_SECONDS: u64 = 105;
/// Hard cap for one recording session. The session is stopped and processed at this limit.
const MAX_RECORDING_SECONDS: u64 = 120;
/// How long the transcribed overlay remains visible before accepting a new recording.
const TRANSCRIBED_OVERLAY_SECONDS: u64 = 3;

/// A press shorter than this many milliseconds is treated as a tap rather
/// than a hold. Holds run normal hold-to-talk; taps may compose into a
/// double-tap that engages hands-free mode.
const TAP_THRESHOLD_MS: i64 = 250;
/// After a tap, this is how long the manager waits for a second tap before
/// cancelling the in-flight recording and returning to idle.
const DOUBLE_TAP_WINDOW_MS: u64 = 350;

/// Shared hotkey state machine tracking recording mode.
struct HotkeyState {
    /// Current mode: idle, hold-recording, tap-pending, hands-free, or processing.
    mode: AtomicU8,
    /// Wall-clock millis since UNIX epoch of the most recent record-press.
    /// Zero when no press is pending.
    press_time_ms: AtomicI64,
}

/// Returns true when the given mode keeps the recorder running.
fn is_recording_mode(mode: u8) -> bool {
    matches!(
        mode,
        MODE_HOLD_RECORDING | MODE_TAP_PENDING | MODE_HANDS_FREE
    )
}

/// Wall-clock millis since UNIX epoch, saturating to 0 if the system clock
/// is set before 1970 (extremely unlikely but better than crashing).
fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn get_hotkey_setting(app: &AppHandle, key: &str, default: &str) -> String {
    let state = app.state::<AppState>();
    let conn = match state.db.lock() {
        Ok(conn) => conn,
        Err(_) => return default.to_string(),
    };

    queries::get_setting(&conn, key).unwrap_or_else(|_| default.to_string())
}

/// Initialize global hotkeys. Must be called from Tauri setup() after AppState is managed.
///
/// Hands-free is NOT registered as a separate global shortcut. Instead it is
/// engaged by double-tapping the configured record hotkey within
/// `DOUBLE_TAP_WINDOW_MS` of the first release.
pub fn register_hotkeys(app: &AppHandle) -> Result<(), String> {
    log::info!("HOTKEY: Registering hotkeys...");
    let mut failures = Vec::new();

    let hotkey_state = Arc::new(HotkeyState {
        mode: AtomicU8::new(MODE_IDLE),
        press_time_ms: AtomicI64::new(0),
    });

    let record_hotkey = get_hotkey_setting(app, "hotkey_record", "F8");
    let paste_last_hotkey = get_hotkey_setting(app, "hotkey_paste_last", "Ctrl+Alt+J");
    let open_app_hotkey = get_hotkey_setting(app, "hotkey_open_app", "Ctrl+Alt+O");

    log::info!(
        "HOTKEY: record='{}', paste_last='{}', open_app='{}'",
        record_hotkey,
        paste_last_hotkey,
        open_app_hotkey
    );

    // Register the record hotkey. Press starts (or stops) recording; release
    // either stops it or hands the state machine to the double-tap watchdog.
    let state_clone = hotkey_state.clone();
    let app_handle = app.clone();
    match app.global_shortcut().on_shortcut(
        record_hotkey.as_str(),
        move |_app, _shortcut, event| {
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
        },
    ) {
        Ok(()) => log::info!("HOTKEY: Record hotkey registered: {record_hotkey}"),
        Err(e) => {
            let message = format!("Failed to register Record hotkey '{record_hotkey}': {e}");
            log::warn!("HOTKEY: {message}");
            failures.push(message);
        }
    }

    let app_handle = app.clone();
    match app.global_shortcut().on_shortcut(
        paste_last_hotkey.as_str(),
        move |_app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                log::info!("HOTKEY: Paste-last triggered");
                let handle = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    handle_paste_last(handle).await;
                });
            }
        },
    ) {
        Ok(()) => log::info!("HOTKEY: Paste-last hotkey registered: {paste_last_hotkey}"),
        Err(e) => {
            let message =
                format!("Failed to register Paste Last hotkey '{paste_last_hotkey}': {e}");
            log::warn!("HOTKEY: {message}");
            failures.push(message);
        }
    }

    let app_handle = app.clone();
    match app.global_shortcut().on_shortcut(
        open_app_hotkey.as_str(),
        move |_app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                log::info!("HOTKEY: Open-app triggered");
                if let Some(window) = app_handle.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        },
    ) {
        Ok(()) => log::info!("HOTKEY: Open-app hotkey registered: {open_app_hotkey}"),
        Err(e) => {
            let message = format!("Failed to register Open App hotkey '{open_app_hotkey}': {e}");
            log::warn!("HOTKEY: {message}");
            failures.push(message);
        }
    }

    // NOTE: the cancel hotkey is registered dynamically when recording starts
    // so we do not capture it system-wide while idle.

    if failures.is_empty() {
        log::info!("HOTKEY: Registration complete");
        Ok(())
    } else {
        Err(failures.join("; "))
    }
}

/// Unregister all global shortcuts and re-register from current DB settings.
pub fn reload_hotkeys(app: &AppHandle) -> Result<(), String> {
    app.global_shortcut()
        .unregister_all()
        .map_err(|e| format!("Failed to unregister shortcuts: {e}"))?;
    register_hotkeys(app)
}

/// Start a recording session and emit the listening overlay event.
async fn start_recording_session(app: &AppHandle, state: &Arc<HotkeyState>) {
    if state
        .mode
        .compare_exchange(
            MODE_IDLE,
            MODE_HOLD_RECORDING,
            Ordering::SeqCst,
            Ordering::SeqCst,
        )
        .is_err()
    {
        return;
    }

    log::info!("OVERLAY: Showing listening state");
    emit_pipeline_event(app, "listening", None, None, None, None);

    let app_state = app.state::<AppState>();
    if let Err(e) = app_state.recorder.start() {
        log::error!("Failed to start recording: {e}");
        let _ = app_state.recorder.cancel();
        emit_pipeline_event(app, "error", None, None, None, Some(&e.to_string()));
        state.mode.store(MODE_IDLE, Ordering::SeqCst);
        return;
    }

    log::info!("AUDIO: Capture started (hold-to-talk)");
    register_cancel_hotkey(app, state.clone());
    spawn_recording_limit_watchdog(app.clone(), state.clone());
}

/// Watchdog that emits the 105-second warning state and force-stops the session
/// at the 120-second cap. It treats every recording-mode (hold, tap-pending,
/// hands-free) as live so that a session promoted to hands-free still respects
/// the cap.
fn spawn_recording_limit_watchdog(app: AppHandle, hotkey_state: Arc<HotkeyState>) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(RECORDING_WARNING_SECONDS)).await;

        if !is_recording_mode(hotkey_state.mode.load(Ordering::SeqCst)) {
            return;
        }

        emit_pipeline_event(
            &app,
            "stopping-soon",
            None,
            Some((RECORDING_WARNING_SECONDS * 1000) as i64),
            None,
            None,
        );

        tokio::time::sleep(Duration::from_secs(
            MAX_RECORDING_SECONDS - RECORDING_WARNING_SECONDS,
        ))
        .await;

        // Race-safe transition: re-read mode each time and try the cas. If the
        // user already released or stopped, this will harmlessly bail out.
        loop {
            let current = hotkey_state.mode.load(Ordering::SeqCst);
            if !is_recording_mode(current) {
                return;
            }
            if hotkey_state
                .mode
                .compare_exchange(current, MODE_PROCESSING, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                break;
            }
        }

        log::info!("HOTKEY: Recording reached {MAX_RECORDING_SECONDS}s cap; stopping");
        unregister_cancel_hotkey(&app);
        run_pipeline_and_inject(app, hotkey_state).await;
    });
}

/// Watchdog that fires after the double-tap window has elapsed. If no second
/// tap arrived, we cancel the in-flight recording and return to idle.
fn spawn_double_tap_watchdog(app: AppHandle, hotkey_state: Arc<HotkeyState>) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(DOUBLE_TAP_WINDOW_MS)).await;
        if hotkey_state
            .mode
            .compare_exchange(
                MODE_TAP_PENDING,
                MODE_IDLE,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .is_err()
        {
            return;
        }

        log::info!("HOTKEY: Double-tap window elapsed without a second tap; cancelling");
        let app_state = app.state::<AppState>();
        if let Err(e) = app_state.recorder.cancel() {
            log::warn!("Failed to cancel pending recording: {e}");
        }
        unregister_cancel_hotkey(&app);
        emit_pipeline_event(&app, "cancelled", None, None, None, None);
    });
}

/// Handle record hotkey pressed.
async fn handle_record_pressed(app: AppHandle, state: Arc<HotkeyState>) {
    {
        let app_state = app.state::<AppState>();
        if app_state.paused.load(Ordering::SeqCst) {
            return;
        }
    }

    let mode = state.mode.load(Ordering::SeqCst);
    match mode {
        MODE_IDLE => {
            log::info!("HOTKEY: Press detected");
            state.press_time_ms.store(now_ms(), Ordering::SeqCst);
            start_recording_session(&app, &state).await;
        }
        // Second tap of a double-tap. The recorder is still running from the
        // first tap; we just promote the state machine to hands-free so
        // releases stop being a "stop" gesture. If the CAS fails the mode
        // moved out from under us; fall through to the catch-all arm.
        MODE_TAP_PENDING
            if state
                .mode
                .compare_exchange(
                    MODE_TAP_PENDING,
                    MODE_HANDS_FREE,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                )
                .is_ok() =>
        {
            log::info!("HOTKEY: Double-tap detected, hands-free engaged");
        }
        // Single tap inside hands-free stops the session. Same fall-through
        // behaviour if another thread already transitioned us out.
        MODE_HANDS_FREE
            if state
                .mode
                .compare_exchange(
                    MODE_HANDS_FREE,
                    MODE_PROCESSING,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                )
                .is_ok() =>
        {
            log::info!("HOTKEY: Hands-free toggle off");
            unregister_cancel_hotkey(&app);
            run_pipeline_and_inject(app, state).await;
        }
        _ => {
            // Ignore presses while the pipeline is running or while the recorder
            // is in an unexpected state.
        }
    }
}

/// Handle record hotkey released.
async fn handle_record_released(app: AppHandle, state: Arc<HotkeyState>) {
    if state.mode.load(Ordering::SeqCst) != MODE_HOLD_RECORDING {
        // Releases in hands-free, processing, or tap-pending are no-ops.
        return;
    }

    let press_time = state.press_time_ms.load(Ordering::SeqCst);
    let duration_ms = if press_time > 0 {
        now_ms().saturating_sub(press_time)
    } else {
        i64::MAX
    };

    if duration_ms > TAP_THRESHOLD_MS {
        if state
            .mode
            .compare_exchange(
                MODE_HOLD_RECORDING,
                MODE_PROCESSING,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .is_ok()
        {
            log::info!("HOTKEY: Long-release after {duration_ms}ms");
            unregister_cancel_hotkey(&app);
            run_pipeline_and_inject(app, state).await;
        }
        return;
    }

    if state
        .mode
        .compare_exchange(
            MODE_HOLD_RECORDING,
            MODE_TAP_PENDING,
            Ordering::SeqCst,
            Ordering::SeqCst,
        )
        .is_ok()
    {
        log::info!("HOTKEY: Tap detected ({duration_ms}ms), waiting for second tap");
        spawn_double_tap_watchdog(app.clone(), state.clone());
    }
}

/// Run the full pipeline: stop recording -> VAD -> speech recognition -> inject.
async fn run_pipeline_and_inject(app: AppHandle, hotkey_state: Arc<HotkeyState>) {
    let pipeline_start = Instant::now();
    log::info!("run_pipeline_and_inject: starting");
    let app_state = app.state::<AppState>();

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
    log::info!(
        "PIPELINE: Starting for {} samples ({:.1}s audio)",
        raw_audio.len(),
        raw_audio.len() as f64 / 16000.0
    );
    log::info!(
        "Recording stopped. {} samples captured ({}ms audio)",
        raw_audio.len(),
        audio_duration_ms
    );

    emit_pipeline_event(
        &app,
        "processing",
        None,
        Some(audio_duration_ms),
        None,
        None,
    );

    log::info!("Running pipeline (VAD -> speech recognition)...");
    let result = match tokio::time::timeout(
        Duration::from_secs(30),
        execute_pipeline(raw_audio, app_state.inner(), &app),
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
            emit_pipeline_event(
                &app,
                "error",
                None,
                None,
                None,
                Some("Pipeline timed out (30s)"),
            );
            hotkey_state.mode.store(MODE_IDLE, Ordering::SeqCst);
            return;
        }
    };

    log::info!(
        "Pipeline complete: {} chars, {} words",
        result.final_text.len(),
        result.word_count
    );

    if result.final_text.is_empty() {
        log::info!("PIPELINE: No speech detected");
        log::info!("No speech detected, returning to idle");
        emit_pipeline_event(
            &app,
            "no-speech",
            Some("No speech detected"),
            None,
            None,
            None,
        );
        hotkey_state.mode.store(MODE_IDLE, Ordering::SeqCst);
        return;
    }

    emit_pipeline_event(
        &app,
        "transcribed",
        Some(&result.final_text),
        Some(result.duration_ms),
        Some(result.word_count),
        None,
    );

    if let Ok(mut last) = app_state.last_injection.lock() {
        *last = Some(result.final_text.clone());
    }

    let app_name = detector::get_focused_window_name();
    save_history_entry(app_state.inner(), &result, &app_name);
    log::info!("HISTORY: Saved entry");

    log::info!("INJECT: Injecting into [{}]", app_name);
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
            log::info!(
                "PIPELINE: Complete in {}ms",
                pipeline_start.elapsed().as_millis()
            );
            log::info!("Text injected: {} chars", result.final_text.len());
        }
        Ok(Err(e)) => {
            log::error!("Injection failed: {e}");
            emit_pipeline_event(
                &app,
                "error",
                Some(&result.final_text),
                Some(result.duration_ms),
                Some(result.word_count),
                Some(&e),
            );
        }
        Err(e) => {
            log::error!("Injection task panicked: {e}");
            emit_pipeline_event(
                &app,
                "error",
                Some(&result.final_text),
                Some(result.duration_ms),
                Some(result.word_count),
                Some(&format!("Injection task panicked: {e}")),
            );
        }
    }

    tokio::time::sleep(Duration::from_secs(TRANSCRIBED_OVERLAY_SECONDS)).await;
    hotkey_state.mode.store(MODE_IDLE, Ordering::SeqCst);
}

/// Handle cancel during recording.
async fn handle_cancel(app: AppHandle, hotkey_state: Arc<HotkeyState>) {
    let current_mode = hotkey_state.mode.load(Ordering::SeqCst);
    if !is_recording_mode(current_mode) {
        return;
    }

    let app_state = app.state::<AppState>();
    if let Err(e) = app_state.recorder.cancel() {
        log::warn!("Cancel recording failed: {e}");
    }
    unregister_cancel_hotkey(&app);
    emit_pipeline_event(&app, "cancelled", None, None, None, None);
    hotkey_state.mode.store(MODE_IDLE, Ordering::SeqCst);
    log::info!("Recording cancelled via hotkey");
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
            match tokio::task::spawn_blocking(move || crate::injection::injector::inject(&t, false))
                .await
            {
                Ok(Ok(())) => log::info!("Paste-last successful"),
                Ok(Err(e)) => log::error!("Paste-last injection failed: {e}"),
                Err(e) => log::error!("Paste-last task panicked: {e}"),
            }
        }
    }
}

/// Dynamically register the configured cancel hotkey (only while recording).
fn register_cancel_hotkey(app: &AppHandle, hotkey_state: Arc<HotkeyState>) {
    let cancel_hotkey = get_hotkey_setting(app, "hotkey_cancel", "Escape");
    let handle = app.clone();
    if let Err(e) =
        app.global_shortcut()
            .on_shortcut(cancel_hotkey.as_str(), move |_app, _shortcut, event| {
                if event.state == ShortcutState::Pressed {
                    let h = handle.clone();
                    let s = hotkey_state.clone();
                    tauri::async_runtime::spawn(async move {
                        handle_cancel(h, s).await;
                    });
                }
            })
    {
        log::warn!("Failed to register cancel hotkey '{cancel_hotkey}': {e}");
    }
}

/// Unregister the configured cancel hotkey.
fn unregister_cancel_hotkey(app: &AppHandle) {
    let cancel_hotkey = get_hotkey_setting(app, "hotkey_cancel", "Escape");
    if let Err(e) = app.global_shortcut().unregister(cancel_hotkey.as_str()) {
        log::warn!("Failed to unregister cancel hotkey '{cancel_hotkey}': {e}");
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

    let tooltip = match state {
        "listening" | "stopping-soon" => "LocalYapper \u{2014} Recording...",
        "processing" | "long-recording" => "LocalYapper \u{2014} Processing...",
        _ => "LocalYapper",
    };
    crate::tray::update_tray_tooltip(app, tooltip);
}
