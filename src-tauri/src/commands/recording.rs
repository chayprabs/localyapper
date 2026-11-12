// IPC command handlers -- dictation actions routed through the controller
use crate::controller::Msg;
use crate::core::session::Event;
use crate::state::AppState;

/// Re-inject the last dictated text.
#[tauri::command]
pub async fn paste_last(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let has_text = state
        .last_injection
        .lock()
        .map(|g| g.as_deref().is_some_and(|t| !t.is_empty()))
        .unwrap_or(false);
    if !has_text {
        return Err("No previous injection to paste".to_string());
    }
    state.controller.send(Msg::PasteLast);
    Ok(())
}

/// Cancel the recording in progress, discarding its audio.
#[tauri::command]
pub async fn cancel_recording(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.controller.send(Msg::Session(Event::Cancel));
    Ok(())
}

/// The overlay's Copy button.
#[tauri::command]
pub async fn overlay_copy(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.controller.send(Msg::OverlayCopy);
    Ok(())
}

/// The overlay's dismiss button.
#[tauri::command]
pub async fn overlay_dismiss(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.controller.send(Msg::OverlayDismiss);
    Ok(())
}

/// The view the overlay should show now. The page asks once when it loads,
/// so a view sent before its listener existed is not lost.
#[tauri::command]
pub async fn overlay_current_view(
    state: tauri::State<'_, AppState>,
) -> Result<crate::platform::overlay::ViewPayload, String> {
    Ok(state
        .controller
        .query()
        .await
        .and_then(|s| s.last_view)
        .unwrap_or(crate::platform::overlay::ViewPayload::Hidden))
}

/// The Audio page's test dictation: `true` starts recording, `false`
/// stops. The result arrives as a `test-dictation-result` event and
/// levels as `audio-level` events; nothing is pasted or saved.
#[tauri::command]
pub fn test_dictation(state: tauri::State<'_, AppState>, start: bool) {
    state.controller.send(Msg::TestDictation(start));
}
