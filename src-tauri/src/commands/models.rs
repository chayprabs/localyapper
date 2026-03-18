use std::sync::atomic::Ordering;

use futures_util::StreamExt;
use tauri::{Emitter, Manager};

use crate::models::{ConnectionResult, DownloadProgress, OllamaStatus};
use crate::state::AppState;

/// HuggingFace URL for the bundled LLM model.
const MODEL_DOWNLOAD_URL: &str =
    "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF/resolve/main/qwen2.5-0.5b-instruct-q4_k_m.gguf";

/// Check if Ollama is running and return available models.
#[tauri::command]
pub async fn check_ollama() -> Result<OllamaStatus, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .map_err(|e| e.to_string())?;

    match client.get("http://localhost:11434/api/tags").send().await {
        Ok(resp) if resp.status().is_success() => {
            let body = resp
                .json::<serde_json::Value>()
                .await
                .map_err(|e| e.to_string())?;
            let models: Vec<String> = body
                .get("models")
                .and_then(|m| m.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|m| m.get("name").and_then(|n| n.as_str()).map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            Ok(OllamaStatus {
                running: true,
                models,
            })
        }
        _ => Ok(OllamaStatus {
            running: false,
            models: vec![],
        }),
    }
}

/// Begin downloading the bundled LLM model to app data dir.
///
/// Emits `model_download_progress` events with `DownloadProgress` payload.
/// Checks `download_cancel` AtomicBool between chunks to support cancellation.
#[tauri::command]
pub async fn download_model(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // Reset cancel flag
    state.download_cancel.store(false, Ordering::SeqCst);

    let models_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e: tauri::Error| e.to_string())?
        .join("models");

    std::fs::create_dir_all(&models_dir).map_err(|e| format!("Failed to create models dir: {e}"))?;

    let dest_path = models_dir.join(crate::llm::engine::LLM_MODEL_FILENAME);
    let temp_path = models_dir.join(format!("{}.download", crate::llm::engine::LLM_MODEL_FILENAME));

    let client = reqwest::Client::new();
    let resp = client
        .get(MODEL_DOWNLOAD_URL)
        .send()
        .await
        .map_err(|e| format!("Download request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Download failed with status: {}", resp.status()));
    }

    let total_bytes = resp.content_length().unwrap_or(0);
    let total_mb = total_bytes / (1024 * 1024);

    let mut stream = resp.bytes_stream();
    let mut file = std::fs::File::create(&temp_path)
        .map_err(|e| format!("Failed to create temp file: {e}"))?;

    let mut downloaded: u64 = 0;
    let start = std::time::Instant::now();
    let cancel_flag = state.download_cancel.clone();

    use std::io::Write;
    while let Some(chunk) = stream.next().await {
        // Check cancellation
        if cancel_flag.load(Ordering::SeqCst) {
            drop(file);
            let _ = std::fs::remove_file(&temp_path);
            return Err("Download cancelled".to_string());
        }

        let chunk = chunk.map_err(|e| format!("Download stream error: {e}"))?;
        file.write_all(&chunk)
            .map_err(|e| format!("Failed to write chunk: {e}"))?;

        downloaded += chunk.len() as u64;
        let elapsed = start.elapsed().as_secs_f64();
        let speed_mbps = if elapsed > 0.0 {
            (downloaded as f64 / (1024.0 * 1024.0)) / elapsed
        } else {
            0.0
        };
        let percent = if total_bytes > 0 {
            (downloaded as f64 / total_bytes as f64) * 100.0
        } else {
            0.0
        };

        let _ = app_handle.emit(
            "model_download_progress",
            DownloadProgress {
                percent,
                downloaded_mb: downloaded / (1024 * 1024),
                total_mb,
                speed_mbps,
            },
        );
    }

    drop(file);

    // Rename temp file to final destination
    std::fs::rename(&temp_path, &dest_path)
        .map_err(|e| format!("Failed to rename downloaded model: {e}"))?;

    log::info!("LLM model downloaded to {}", dest_path.display());
    Ok(())
}

/// Cancel an in-progress model download.
#[tauri::command]
pub async fn cancel_model_download(
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state.download_cancel.store(true, Ordering::SeqCst);
    log::info!("Model download cancellation requested");
    Ok(())
}

/// Get list of available Ollama models.
#[tauri::command]
pub async fn get_ollama_models() -> Result<Vec<String>, String> {
    let status = check_ollama().await?;
    Ok(status.models)
}

/// Test BYOK API key connection to a provider.
///
/// Sends a minimal test request and measures latency.
#[tauri::command]
pub async fn test_byok_connection(
    provider: String,
    api_key: String,
) -> Result<ConnectionResult, String> {
    let (url, body, auth_header) = match provider.to_lowercase().as_str() {
        "openai" => (
            "https://api.openai.com/v1/chat/completions",
            serde_json::json!({
                "model": "gpt-3.5-turbo",
                "messages": [{"role": "user", "content": "Hi"}],
                "max_tokens": 1
            }),
            format!("Bearer {api_key}"),
        ),
        "anthropic" => (
            "https://api.anthropic.com/v1/messages",
            serde_json::json!({
                "model": "claude-haiku-4-5-20251001",
                "max_tokens": 1,
                "messages": [{"role": "user", "content": "Hi"}]
            }),
            api_key.clone(),
        ),
        "groq" => (
            "https://api.groq.com/openai/v1/chat/completions",
            serde_json::json!({
                "model": "llama3-8b-8192",
                "messages": [{"role": "user", "content": "Hi"}],
                "max_tokens": 1
            }),
            format!("Bearer {api_key}"),
        ),
        _ => {
            return Ok(ConnectionResult {
                success: false,
                latency_ms: 0,
                error: Some(format!("Unknown provider: {provider}")),
            });
        }
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let start = std::time::Instant::now();

    let is_anthropic = provider.to_lowercase() == "anthropic";
    let mut req = client.post(url).json(&body);

    if is_anthropic {
        req = req
            .header("x-api-key", &auth_header)
            .header("anthropic-version", "2023-06-01");
    } else {
        req = req.header("Authorization", &auth_header);
    }

    match req.send().await {
        Ok(resp) => {
            let latency_ms = start.elapsed().as_millis() as u64;
            let success = resp.status().is_success();
            if success {
                Ok(ConnectionResult {
                    success: true,
                    latency_ms,
                    error: None,
                })
            } else {
                let status = resp.status();
                let body_text = resp.text().await.unwrap_or_default();
                Ok(ConnectionResult {
                    success: false,
                    latency_ms,
                    error: Some(format!("HTTP {status}: {body_text}")),
                })
            }
        }
        Err(e) => Ok(ConnectionResult {
            success: false,
            latency_ms: start.elapsed().as_millis() as u64,
            error: Some(format!("Request failed: {e}")),
        }),
    }
}
