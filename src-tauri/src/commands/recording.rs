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
