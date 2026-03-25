// IPC command handlers -- model download, status, and lifecycle
use std::sync::atomic::Ordering;

use futures_util::StreamExt;
use tauri::{Emitter, Manager};

use crate::models::{ConnectionResult, DownloadProgress, LlmFileStatus, ModelsStatus, OllamaStatus, WhisperFileStatus};
use crate::state::AppState;

/// HuggingFace URL for the Qwen3 0.6B GGUF model.
const MODEL_DOWNLOAD_URL: &str =
    "https://huggingface.co/unsloth/Qwen3-0.6B-GGUF/resolve/main/Qwen3-0.6B-Q4_K_M.gguf";

/// HuggingFace URL for the Qwen3 0.6B tokenizer.
const TOKENIZER_DOWNLOAD_URL: &str =
    "https://huggingface.co/Qwen/Qwen3-0.6B/resolve/main/tokenizer.json";

// Whisper download URLs are now dynamic — see stt::whisper::whisper_download_url()

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

/// Begin downloading the Qwen3 0.6B GGUF model + tokenizer to app data dir.
///
/// Emits `model_download_progress` events with `DownloadProgress` payload.
/// Checks `download_cancel` AtomicBool between chunks to support cancellation.
/// Skips GGUF download if the file already exists.
#[tauri::command]
pub async fn download_model(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state.download_cancel.store(false, Ordering::SeqCst);

    let models_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e: tauri::Error| e.to_string())?
        .join("models");

    std::fs::create_dir_all(&models_dir).map_err(|e| format!("Failed to create models dir: {e}"))?;

    let dest_path = models_dir.join(crate::llm::engine::LLM_MODEL_FILENAME);

    // Skip GGUF if already downloaded
    if dest_path.exists() {
        log::info!("LLM model already exists at {}", dest_path.display());
    } else {
        let temp_path = models_dir.join(format!("{}.download", crate::llm::engine::LLM_MODEL_FILENAME));

        let client = reqwest::Client::new();

        // Check for partial download to support resume
        let existing_bytes = if temp_path.exists() {
            std::fs::metadata(&temp_path).map(|m| m.len()).unwrap_or(0)
        } else {
            0
        };

        let mut req = client.get(MODEL_DOWNLOAD_URL);
        if existing_bytes > 0 {
            req = req.header("Range", format!("bytes={existing_bytes}-"));
            log::info!("Resuming download from {} bytes", existing_bytes);
        }

        let resp = req.send().await
            .map_err(|e| format!("Download request failed: {e}"))?;

        let status = resp.status();
        if !status.is_success() && status != reqwest::StatusCode::PARTIAL_CONTENT {
            return Err(format!("Download failed with status: {status}"));
        }

        let is_resume = status == reqwest::StatusCode::PARTIAL_CONTENT;
        let total_bytes = if is_resume {
            resp.content_length().unwrap_or(0) + existing_bytes
        } else {
            resp.content_length().unwrap_or(0)
        };
        let total_mb = total_bytes / (1024 * 1024);

        let mut stream = resp.bytes_stream();

        use std::io::Write;
        let mut file = if existing_bytes > 0 && is_resume {
            std::fs::OpenOptions::new().append(true).open(&temp_path)
                .map_err(|e| format!("Failed to open temp file for resume: {e}"))?
        } else {
            std::fs::File::create(&temp_path)
                .map_err(|e| format!("Failed to create temp file: {e}"))?
        };

        let mut downloaded: u64 = if is_resume { existing_bytes } else { 0 };
        let start = std::time::Instant::now();
        let cancel_flag = state.download_cancel.clone();

        while let Some(chunk) = stream.next().await {
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
        std::fs::rename(&temp_path, &dest_path)
            .map_err(|e| format!("Failed to rename downloaded model: {e}"))?;

        log::info!("LLM model downloaded to {}", dest_path.display());
    }

    // Download tokenizer alongside the GGUF (small file, no progress needed)
    let tokenizer_path = models_dir.join(crate::llm::engine::LLM_TOKENIZER_FILENAME);
    if !tokenizer_path.exists() {
        log::info!("Downloading tokenizer...");
        let client = reqwest::Client::new();
        let resp = client.get(TOKENIZER_DOWNLOAD_URL).send().await
            .map_err(|e| format!("Tokenizer download failed: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!("Tokenizer download failed with status: {}", resp.status()));
        }
        let bytes = resp.bytes().await.map_err(|e| format!("Tokenizer read failed: {e}"))?;
        std::fs::write(&tokenizer_path, &bytes)
            .map_err(|e| format!("Failed to write tokenizer: {e}"))?;
        log::info!("Tokenizer downloaded to {}", tokenizer_path.display());
    }

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

/// Download a Whisper STT model to app data dir.
///
/// Accepts an optional model variant (e.g. "base.en", "tiny.en"). Defaults to `DEFAULT_WHISPER_MODEL`.
/// Emits `whisper_download_progress` events with `DownloadProgress` payload.
/// Supports resume via HTTP Range headers.
#[tauri::command]
pub async fn download_whisper_model(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    model: Option<String>,
) -> Result<(), String> {
    state.download_cancel.store(false, Ordering::SeqCst);

    let model_name = model.as_deref().unwrap_or(crate::stt::whisper::DEFAULT_WHISPER_MODEL);
    let filename = crate::stt::whisper::whisper_model_filename(model_name);
    let url = crate::stt::whisper::whisper_download_url(model_name);

    let models_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e: tauri::Error| e.to_string())?
        .join("models");

    std::fs::create_dir_all(&models_dir).map_err(|e| format!("Failed to create models dir: {e}"))?;

    let dest_path = models_dir.join(&filename);

    // Skip if already downloaded and file is not corrupt (> 1 MB)
    if dest_path.exists() {
        let size = std::fs::metadata(&dest_path).map(|m| m.len()).unwrap_or(0);
        if size > 1_000_000 {
            log::info!("Whisper model already exists at {} ({} bytes)", dest_path.display(), size);
            return Ok(());
        }
        log::warn!("Whisper model at {} is too small ({} bytes), re-downloading", dest_path.display(), size);
        let _ = std::fs::remove_file(&dest_path);
    }

    let temp_path = models_dir.join(format!("{filename}.download"));

    let client = reqwest::Client::new();

    // Check for partial download to support resume
    let existing_bytes = if temp_path.exists() {
        std::fs::metadata(&temp_path).map(|m| m.len()).unwrap_or(0)
    } else {
        0
    };

    let mut req = client.get(&url);
    if existing_bytes > 0 {
        req = req.header("Range", format!("bytes={existing_bytes}-"));
        log::info!("Resuming Whisper download from {} bytes", existing_bytes);
    }

    let resp = req.send().await
        .map_err(|e| format!("Download request failed: {e}"))?;

    let status = resp.status();
    if !status.is_success() && status != reqwest::StatusCode::PARTIAL_CONTENT {
        return Err(format!("Download failed with status: {status}"));
    }

    let is_resume = status == reqwest::StatusCode::PARTIAL_CONTENT;
    let total_bytes = if is_resume {
        resp.content_length().unwrap_or(0) + existing_bytes
    } else {
        resp.content_length().unwrap_or(0)
    };
    let total_mb = total_bytes / (1024 * 1024);

    let mut stream = resp.bytes_stream();

    use std::io::Write;
    let mut file = if existing_bytes > 0 && is_resume {
        std::fs::OpenOptions::new().append(true).open(&temp_path)
            .map_err(|e| format!("Failed to open temp file for resume: {e}"))?
    } else {
        std::fs::File::create(&temp_path)
            .map_err(|e| format!("Failed to create temp file: {e}"))?
    };

    let mut downloaded: u64 = if is_resume { existing_bytes } else { 0 };
    let start = std::time::Instant::now();
    let cancel_flag = state.download_cancel.clone();

    while let Some(chunk) = stream.next().await {
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
            "whisper_download_progress",
            DownloadProgress {
                percent,
                downloaded_mb: downloaded / (1024 * 1024),
                total_mb,
                speed_mbps,
            },
        );
    }

    drop(file);

    std::fs::rename(&temp_path, &dest_path)
        .map_err(|e| format!("Failed to rename downloaded model: {e}"))?;

    log::info!("Whisper model ({}) downloaded to {}", model_name, dest_path.display());
    Ok(())
}

/// Check if a Whisper model file exists on disk.
#[tauri::command]
pub async fn check_whisper_file_exists(
    app_handle: tauri::AppHandle,
    model: Option<String>,
) -> Result<WhisperFileStatus, String> {
    let model_name = model.unwrap_or_else(|| {
        crate::stt::whisper::DEFAULT_WHISPER_MODEL.to_string()
    });
    let filename = crate::stt::whisper::whisper_model_filename(&model_name);

    let models_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e: tauri::Error| e.to_string())?
        .join("models");

    let path = models_dir.join(&filename);

    if path.exists() {
        let size_bytes = std::fs::metadata(&path)
            .map(|m| m.len())
            .unwrap_or(0);
        Ok(WhisperFileStatus {
            exists: true,
            size_mb: size_bytes / (1024 * 1024),
            model_name,
        })
    } else {
        Ok(WhisperFileStatus {
            exists: false,
            size_mb: 0,
            model_name,
        })
    }
}

/// Delete a Whisper model file and unload from AppState.
#[tauri::command]
pub async fn delete_whisper_model(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    model: String,
) -> Result<(), String> {
    // Unload from AppState
    if let Ok(mut g) = state.whisper.lock() {
        *g = None;
    }

    let filename = crate::stt::whisper::whisper_model_filename(&model);
    let models_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e: tauri::Error| e.to_string())?
        .join("models");

    let path = models_dir.join(&filename);
    if path.exists() {
        std::fs::remove_file(&path)
            .map_err(|e| format!("Failed to delete Whisper model: {e}"))?;
    }

    log::info!("Whisper model ({}) deleted", model);
    Ok(())
}

/// Reload models from disk into AppState. Call after downloading new models.
///
/// Model loading is done on a blocking thread since whisper-rs and llama-cpp
/// perform heavy C FFI operations that need a full OS thread stack.
/// Returns errors for any model that fails to load (both models still get a chance).
#[tauri::command]
pub async fn reload_models(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut errors: Vec<String> = Vec::new();

    // Reload Whisper if not already loaded
    let whisper_loaded = state.whisper.lock()
        .map(|g| g.is_some())
        .unwrap_or(false);

    println!("RELOAD: Whisper currently loaded: {whisper_loaded}");

    if !whisper_loaded {
        let model_setting = {
            let db = state.db.lock().map_err(|e| format!("DB lock failed: {e}"))?;
            crate::db::queries::get_setting(&db, "whisper_model")
                .unwrap_or_else(|_| crate::stt::whisper::DEFAULT_WHISPER_MODEL.to_string())
        };
        let candidates = crate::whisper_model_candidates(&app_handle, &model_setting);
        println!("RELOAD: Whisper model setting='{}', candidates: {:?}", model_setting,
            candidates.iter().map(|p| p.display().to_string()).collect::<Vec<_>>());

        let whisper_mutex = state.whisper.clone();

        let whisper_result = tokio::task::spawn_blocking(move || {
            for candidate in &candidates {
                if candidate.exists() {
                    let size = std::fs::metadata(candidate).map(|m| m.len()).unwrap_or(0);
                    println!("RELOAD: Trying Whisper at {} ({} bytes)", candidate.display(), size);
                    match crate::stt::whisper::WhisperEngine::new(candidate) {
                        Ok(engine) => {
                            println!("RELOAD: Whisper loaded successfully from {}", candidate.display());
                            log::info!("Hot-loaded Whisper engine from {}", candidate.display());
                            if let Ok(mut g) = whisper_mutex.lock() {
                                *g = Some(std::sync::Arc::new(engine));
                            }
                            return Ok(());
                        }
                        Err(e) => {
                            println!("RELOAD: Whisper load FAILED from {}: {e}", candidate.display());
                            log::warn!("Failed to load Whisper from {}: {e}", candidate.display());
                            return Err(format!("Failed to load Whisper from {}: {e}", candidate.display()));
                        }
                    }
                } else {
                    println!("RELOAD: File not found at {}", candidate.display());
                }
            }
            Err("No Whisper model file found at any candidate path".to_string())
        })
        .await
        .map_err(|e| format!("Whisper load task panicked: {e}"))?;

        if let Err(e) = whisper_result {
            errors.push(e);
        }
    }

    // Reload LLM if not already loaded (async — mistral.rs model loading is async)
    let llm_loaded = state.llm.lock()
        .map(|g| g.is_some())
        .unwrap_or(false);

    println!("RELOAD: LLM currently loaded: {llm_loaded}");

    if !llm_loaded {
        let models_dir = app_handle
            .path()
            .app_data_dir()
            .map_err(|e: tauri::Error| e.to_string())?
            .join("models");

        let gguf_path = models_dir.join(crate::llm::engine::LLM_MODEL_FILENAME);
        let tokenizer_path = models_dir.join(crate::llm::engine::LLM_TOKENIZER_FILENAME);

        if gguf_path.exists() && tokenizer_path.exists() {
            println!("RELOAD: Loading LLM from {}", models_dir.display());
            let llm_mutex = state.llm.clone();
            match crate::llm::engine::LlmEngine::new(&models_dir).await {
                Ok(engine) => {
                    println!("RELOAD: LLM loaded successfully");
                    log::info!("Hot-loaded LLM engine from {}", models_dir.display());
                    if let Ok(mut g) = llm_mutex.lock() {
                        *g = Some(std::sync::Arc::new(engine));
                    }
                }
                Err(e) => {
                    println!("RELOAD: LLM load FAILED: {e}");
                    log::warn!("Failed to load LLM from {}: {e}", models_dir.display());
                    errors.push(format!("Failed to load LLM: {e}"));
                }
            }
        } else {
            println!("RELOAD: LLM files not found in {}", models_dir.display());
            log::info!("LLM model files not found in {}, skipping", models_dir.display());
        }
    }

    if !errors.is_empty() {
        return Err(errors.join("; "));
    }
    Ok(())
}

/// Check which models are currently loaded.
#[tauri::command]
pub async fn check_models_status(
    state: tauri::State<'_, AppState>,
) -> Result<ModelsStatus, String> {
    let whisper_loaded = state.whisper.lock()
        .map(|g| g.is_some())
        .unwrap_or(false);
    let llm_loaded = state.llm.lock()
        .map(|g| g.is_some())
        .unwrap_or(false);

    Ok(ModelsStatus {
        whisper_loaded,
        llm_loaded,
    })
}

/// Check if the LLM GGUF file exists on disk.
#[tauri::command]
pub async fn check_llm_file_exists(
    app_handle: tauri::AppHandle,
) -> Result<LlmFileStatus, String> {
    let models_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e: tauri::Error| e.to_string())?
        .join("models");

    let gguf_path = models_dir.join(crate::llm::engine::LLM_MODEL_FILENAME);

    if gguf_path.exists() {
        let size_bytes = std::fs::metadata(&gguf_path)
            .map(|m| m.len())
            .unwrap_or(0);
        Ok(LlmFileStatus {
            exists: true,
            size_mb: size_bytes / (1024 * 1024),
        })
    } else {
        Ok(LlmFileStatus {
            exists: false,
            size_mb: 0,
        })
    }
}

/// Delete the LLM model files (GGUF + tokenizer) and unload from AppState.
#[tauri::command]
pub async fn delete_llm_model(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // Unload from AppState first
    if let Ok(mut g) = state.llm.lock() {
        *g = None;
    }

    let models_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e: tauri::Error| e.to_string())?
        .join("models");

    let gguf_path = models_dir.join(crate::llm::engine::LLM_MODEL_FILENAME);
    let tokenizer_path = models_dir.join(crate::llm::engine::LLM_TOKENIZER_FILENAME);

    if gguf_path.exists() {
        std::fs::remove_file(&gguf_path)
            .map_err(|e| format!("Failed to delete model: {e}"))?;
    }
    if tokenizer_path.exists() {
        std::fs::remove_file(&tokenizer_path)
            .map_err(|e| format!("Failed to delete tokenizer: {e}"))?;
    }

    log::info!("LLM model files deleted");
    Ok(())
}
