// IPC command handlers -- model download, status, and lifecycle
use std::sync::atomic::Ordering;

use futures_util::StreamExt;
use tauri::{Emitter, Manager};

use crate::models::{ConnectionResult, DownloadProgress, LlmFileStatus, ModelsStatus, OllamaStatus, WhisperFileStatus};
use crate::state::AppState;

/// HuggingFace URL for the Qwen2.5 1.5B Instruct GGUF model.
const MODEL_DOWNLOAD_URL: &str =
    "https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/qwen2.5-1.5b-instruct-q4_k_m.gguf";

/// HuggingFace URL for the Qwen2.5 1.5B Instruct tokenizer.
const TOKENIZER_DOWNLOAD_URL: &str =
    "https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct/resolve/main/tokenizer.json";

// STT model download URLs are in stt::whisper::stt_model_files()

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

/// Begin downloading the Qwen2.5 1.5B Instruct GGUF model + tokenizer to app data dir.
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

/// Download STT model files (Parakeet ONNX + Silero VAD) to app data dir.
///
/// Downloads multiple files into a model-specific subdirectory.
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
    let model_files = crate::stt::whisper::stt_model_files(model_name);

    if model_files.is_empty() {
        return Err(format!("Unknown STT model: {model_name}"));
    }

    let models_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e: tauri::Error| e.to_string())?
        .join("models");

    std::fs::create_dir_all(&models_dir).map_err(|e| format!("Failed to create models dir: {e}"))?;

    // Create model subdirectory
    let model_dir_name = crate::stt::whisper::stt_model_dir_name(model_name);
    let model_dir = models_dir.join(&model_dir_name);
    std::fs::create_dir_all(&model_dir).map_err(|e| format!("Failed to create model dir: {e}"))?;

    // Calculate total size for progress (estimate from known model sizes)
    let estimated_total_bytes: u64 = match model_name {
        "parakeet-110m" => 458 * 1024 * 1024, // ~458MB FP32
        "parakeet-0.6b" => 661 * 1024 * 1024,  // ~661MB
        _ => 100 * 1024 * 1024,
    };
    let total_mb = estimated_total_bytes / (1024 * 1024);
    let mut cumulative_downloaded: u64 = 0;
    let start = std::time::Instant::now();
    let cancel_flag = state.download_cancel.clone();

    // Download each model file
    for (filename, url) in &model_files {
        let dest_path = model_dir.join(filename);

        // Skip if already downloaded and file is not corrupt (> 1 KB for tokens, > 1 MB for model)
        let min_size = if *filename == "tokens.txt" { 100 } else { 1_000_000 };
        if dest_path.exists() {
            let size = std::fs::metadata(&dest_path).map(|m| m.len()).unwrap_or(0);
            if size > min_size {
                log::info!("STT file {} already exists ({} bytes), skipping", filename, size);
                cumulative_downloaded += size;
                continue;
            }
        }

        let temp_path = model_dir.join(format!("{filename}.download"));
        let client = reqwest::Client::new();

        // Check for partial download to support resume
        let existing_bytes = if temp_path.exists() {
            std::fs::metadata(&temp_path).map(|m| m.len()).unwrap_or(0)
        } else {
            0
        };

        let mut req = client.get(url.as_str());
        if existing_bytes > 0 {
            req = req.header("Range", format!("bytes={existing_bytes}-"));
            log::info!("Resuming {} download from {} bytes", filename, existing_bytes);
        }

        let resp = req.send().await
            .map_err(|e| format!("Download {} failed: {e}", filename))?;

        let status = resp.status();
        if !status.is_success() && status != reqwest::StatusCode::PARTIAL_CONTENT {
            return Err(format!("Download {} failed with status: {status}", filename));
        }

        let is_resume = status == reqwest::StatusCode::PARTIAL_CONTENT;
        let mut stream = resp.bytes_stream();

        use std::io::Write;
        let mut file = if existing_bytes > 0 && is_resume {
            std::fs::OpenOptions::new().append(true).open(&temp_path)
                .map_err(|e| format!("Failed to open temp file for {}: {e}", filename))?
        } else {
            std::fs::File::create(&temp_path)
                .map_err(|e| format!("Failed to create temp file for {}: {e}", filename))?
        };

        let mut file_downloaded: u64 = if is_resume { existing_bytes } else { 0 };

        while let Some(chunk) = stream.next().await {
            if cancel_flag.load(Ordering::SeqCst) {
                drop(file);
                let _ = std::fs::remove_file(&temp_path);
                return Err("Download cancelled".to_string());
            }

            let chunk = chunk.map_err(|e| format!("Download stream error for {}: {e}", filename))?;
            file.write_all(&chunk)
                .map_err(|e| format!("Failed to write {}: {e}", filename))?;

            file_downloaded += chunk.len() as u64;
            let total_so_far = cumulative_downloaded + file_downloaded;
            let elapsed = start.elapsed().as_secs_f64();
            let speed_mbps = if elapsed > 0.0 {
                (total_so_far as f64 / (1024.0 * 1024.0)) / elapsed
            } else {
                0.0
            };
            let percent = if estimated_total_bytes > 0 {
                ((total_so_far as f64 / estimated_total_bytes as f64) * 100.0).min(99.0)
            } else {
                0.0
            };

            let _ = app_handle.emit(
                "whisper_download_progress",
                DownloadProgress {
                    percent,
                    downloaded_mb: total_so_far / (1024 * 1024),
                    total_mb,
                    speed_mbps,
                },
            );
        }

        drop(file);
        std::fs::rename(&temp_path, &dest_path)
            .map_err(|e| format!("Failed to rename {}: {e}", filename))?;

        cumulative_downloaded += file_downloaded;
        log::info!("STT file {} downloaded to {}", filename, dest_path.display());
    }

    // Also download Silero VAD model if not present
    let vad_path = models_dir.join(crate::stt::whisper::SILERO_VAD_FILENAME);
    if !vad_path.exists() {
        log::info!("Downloading Silero VAD model...");
        let client = reqwest::Client::new();
        let resp = client.get(crate::stt::whisper::SILERO_VAD_URL).send().await
            .map_err(|e| format!("Silero VAD download failed: {e}"))?;
        if !resp.status().is_success() {
            log::warn!("Silero VAD download failed with status: {}", resp.status());
        } else {
            let bytes = resp.bytes().await.map_err(|e| format!("Silero VAD read failed: {e}"))?;
            std::fs::write(&vad_path, &bytes)
                .map_err(|e| format!("Failed to write Silero VAD: {e}"))?;
            log::info!("Silero VAD downloaded to {}", vad_path.display());
        }
    }

    // Emit 100% completion
    let _ = app_handle.emit(
        "whisper_download_progress",
        DownloadProgress {
            percent: 100.0,
            downloaded_mb: total_mb,
            total_mb,
            speed_mbps: 0.0,
        },
    );

    log::info!("STT model ({}) download complete", model_name);
    Ok(())
}

/// Check if the STT model directory exists on disk.
#[tauri::command]
pub async fn check_whisper_file_exists(
    app_handle: tauri::AppHandle,
    model: Option<String>,
) -> Result<WhisperFileStatus, String> {
    let model_name = model.unwrap_or_else(|| {
        crate::stt::whisper::DEFAULT_WHISPER_MODEL.to_string()
    });
    let dir_name = crate::stt::whisper::stt_model_dir_name(&model_name);

    let models_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e: tauri::Error| e.to_string())?
        .join("models");

    let model_dir = models_dir.join(&dir_name);

    // Check if directory exists and contains at least one .onnx file
    if model_dir.is_dir() {
        let has_onnx = std::fs::read_dir(&model_dir)
            .map(|entries| entries.filter_map(|e| e.ok())
                .any(|e| e.file_name().to_string_lossy().ends_with(".onnx")))
            .unwrap_or(false);

        if has_onnx {
            // Sum up all file sizes in the directory
            let total_size: u64 = std::fs::read_dir(&model_dir)
                .map(|entries| entries.filter_map(|e| e.ok())
                    .map(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
                    .sum())
                .unwrap_or(0);

            return Ok(WhisperFileStatus {
                exists: true,
                size_mb: total_size / (1024 * 1024),
                model_name,
            });
        }
    }

    Ok(WhisperFileStatus {
        exists: false,
        size_mb: 0,
        model_name,
    })
}

/// Delete the STT model directory and unload from AppState.
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

    let dir_name = crate::stt::whisper::stt_model_dir_name(&model);
    let models_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e: tauri::Error| e.to_string())?
        .join("models");

    let model_dir = models_dir.join(&dir_name);
    if model_dir.is_dir() {
        std::fs::remove_dir_all(&model_dir)
            .map_err(|e| format!("Failed to delete STT model directory: {e}"))?;
    }

    log::info!("STT model ({}) deleted", model);
    Ok(())
}

/// Reload models from disk into AppState. Call after downloading new models.
///
/// STT loading is done on a blocking thread since sherpa-onnx
/// performs heavy C FFI operations that need a full OS thread stack.
/// Returns errors for any model that fails to load (all models still get a chance).
#[tauri::command]
pub async fn reload_models(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut errors: Vec<String> = Vec::new();

    // Reload STT if not already loaded
    let whisper_loaded = state.whisper.lock()
        .map(|g| g.is_some())
        .unwrap_or(false);

    println!("RELOAD: STT currently loaded: {whisper_loaded}");

    if !whisper_loaded {
        let model_setting = {
            let db = state.db.lock().map_err(|e| format!("DB lock failed: {e}"))?;
            crate::db::queries::get_setting(&db, "whisper_model")
                .unwrap_or_else(|_| crate::stt::whisper::DEFAULT_WHISPER_MODEL.to_string())
        };
        let candidates = crate::whisper_model_candidates(&app_handle, &model_setting);
        println!("RELOAD: STT model setting='{}', candidates: {:?}", model_setting,
            candidates.iter().map(|p| p.display().to_string()).collect::<Vec<_>>());

        let whisper_mutex = state.whisper.clone();

        let whisper_result = tokio::task::spawn_blocking(move || {
            for candidate in &candidates {
                if candidate.exists() && candidate.is_dir() {
                    println!("RELOAD: Trying STT at {}", candidate.display());
                    match crate::stt::whisper::WhisperEngine::new(candidate) {
                        Ok(engine) => {
                            println!("RELOAD: STT loaded successfully from {}", candidate.display());
                            log::info!("Hot-loaded STT engine from {}", candidate.display());
                            if let Ok(mut g) = whisper_mutex.lock() {
                                *g = Some(std::sync::Arc::new(engine));
                            }
                            return Ok(());
                        }
                        Err(e) => {
                            println!("RELOAD: STT load FAILED from {}: {e}", candidate.display());
                            log::warn!("Failed to load STT from {}: {e}", candidate.display());
                            return Err(format!("Failed to load STT from {}: {e}", candidate.display()));
                        }
                    }
                } else {
                    println!("RELOAD: Directory not found at {}", candidate.display());
                }
            }
            Err("No STT model directory found at any candidate path".to_string())
        })
        .await
        .map_err(|e| format!("STT load task panicked: {e}"))?;

        if let Err(e) = whisper_result {
            errors.push(e);
        }
    }

    // Reload Silero VAD if not already loaded
    let vad_loaded = state.vad.lock()
        .map(|g| g.is_some())
        .unwrap_or(false);

    if !vad_loaded {
        let models_dir = app_handle
            .path()
            .app_data_dir()
            .map_err(|e: tauri::Error| e.to_string())?
            .join("models");

        let vad_path = models_dir.join(crate::stt::whisper::SILERO_VAD_FILENAME);
        if vad_path.exists() {
            let vad_mutex = state.vad.clone();
            let vad_result = tokio::task::spawn_blocking(move || {
                match crate::audio::vad::SileroVad::new(&vad_path) {
                    Ok(vad) => {
                        if let Ok(mut g) = vad_mutex.lock() {
                            *g = Some(vad);
                        }
                        Ok(())
                    }
                    Err(e) => Err(format!("Failed to load Silero VAD: {e}")),
                }
            })
            .await
            .map_err(|e| format!("VAD load task panicked: {e}"))?;

            if let Err(e) = vad_result {
                log::warn!("{e}");
                // VAD failure is non-critical — don't add to errors
            } else {
                println!("RELOAD: Silero VAD loaded successfully");
            }
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
