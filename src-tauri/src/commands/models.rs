use crate::models::{ConnectionResult, OllamaStatus};
use crate::state::AppState;

/// Check if Ollama is running and return available models.
#[tauri::command]
pub async fn check_ollama() -> Result<OllamaStatus, String> {
    Ok(OllamaStatus {
        running: false,
        models: vec![],
    })
}

/// Begin downloading the bundled LLM model.
#[tauri::command]
pub async fn download_model(
    _app_handle: tauri::AppHandle,
) -> Result<(), String> {
    Err("Not implemented: Phase 6".to_string())
}

/// Cancel an in-progress model download.
#[tauri::command]
pub async fn cancel_model_download(
    _state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    Err("Not implemented: Phase 6".to_string())
}

/// Get list of available Ollama models.
#[tauri::command]
pub async fn get_ollama_models() -> Result<Vec<String>, String> {
    Ok(vec![])
}

/// Test BYOK API key connection.
#[tauri::command]
pub async fn test_byok_connection(
    _provider: String,
    _api_key: String,
) -> Result<ConnectionResult, String> {
    Err("Not implemented: Phase 6".to_string())
}
