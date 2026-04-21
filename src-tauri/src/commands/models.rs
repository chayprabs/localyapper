// IPC command handlers -- speech model download, status, and lifecycle
use std::sync::atomic::Ordering;

use futures_util::StreamExt;
use tauri::{Emitter, Manager};

use crate::models::{DownloadProgress, ModelsStatus, SpeechModelFileStatus};
use crate::state::AppState;

fn selected_speech_model_name(state: &AppState) -> String {
    state
        .db
        .lock()
        .ok()
        .and_then(|db| crate::db::queries::get_setting(&db, "speech_model").ok())
        .unwrap_or_else(|| crate::stt::whisper::DEFAULT_WHISPER_MODEL.to_string())
}

/// Cancel an in-progress model download.
#[tauri::command]
pub async fn cancel_model_download(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.download_cancel.store(true, Ordering::SeqCst);
    log::info!("Speech model download cancellation requested");
    Ok(())
}

/// Download STT model files (Parakeet ONNX + Silero VAD) to app data dir.
///
/// Downloads multiple files into a model-specific subdirectory.
/// Emits `speech_model_download_progress` events with `DownloadProgress` payload.
/// Supports resume via HTTP Range headers.
#[tauri::command]
pub async fn download_speech_model(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state.download_cancel.store(false, Ordering::SeqCst);

    let model_name = selected_speech_model_name(state.inner());
    let model_files = crate::stt::whisper::stt_model_files(&model_name);

    if model_files.is_empty() {
        return Err(format!("Unknown STT model: {model_name}"));
    }

    let models_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e: tauri::Error| e.to_string())?
        .join("models");

    std::fs::create_dir_all(&models_dir)
        .map_err(|e| format!("Failed to create models dir: {e}"))?;

    let model_dir_name = crate::stt::whisper::stt_model_dir_name(&model_name);
    let model_dir = models_dir.join(&model_dir_name);
    std::fs::create_dir_all(&model_dir).map_err(|e| format!("Failed to create model dir: {e}"))?;

    let estimated_total_bytes: u64 = match model_name.as_str() {
        "parakeet-110m" => 458 * 1024 * 1024,
        "parakeet-0.6b" => 661 * 1024 * 1024,
        _ => 100 * 1024 * 1024,
    };
    let total_mb = estimated_total_bytes / (1024 * 1024);
    let mut cumulative_downloaded: u64 = 0;
    let start = std::time::Instant::now();
    let cancel_flag = state.download_cancel.clone();

    for (filename, url) in &model_files {
        let dest_path = model_dir.join(filename);
        let min_size = if *filename == "tokens.txt" {
            100
        } else {
            1_000_000
        };

        if dest_path.exists() {
            let size = std::fs::metadata(&dest_path).map(|m| m.len()).unwrap_or(0);
            if size > min_size {
                log::info!(
                    "STT file {} already exists ({} bytes), skipping",
                    filename,
                    size
                );
                cumulative_downloaded += size;
                continue;
            }
        }

        let temp_path = model_dir.join(format!("{filename}.download"));
        let client = reqwest::Client::new();

        let existing_bytes = if temp_path.exists() {
            std::fs::metadata(&temp_path).map(|m| m.len()).unwrap_or(0)
        } else {
            0
        };

        let mut req = client.get(url.as_str());
        if existing_bytes > 0 {
            req = req.header("Range", format!("bytes={existing_bytes}-"));
            log::info!(
                "Resuming {} download from {} bytes",
                filename,
                existing_bytes
            );
        }

        let resp = req
            .send()
            .await
            .map_err(|e| format!("Download {} failed: {e}", filename))?;

        let status = resp.status();
        if !status.is_success() && status != reqwest::StatusCode::PARTIAL_CONTENT {
            return Err(format!(
                "Download {} failed with status: {status}",
                filename
            ));
        }

        let is_resume = status == reqwest::StatusCode::PARTIAL_CONTENT;
        let mut stream = resp.bytes_stream();

        use std::io::Write;
        let mut file = if existing_bytes > 0 && is_resume {
            std::fs::OpenOptions::new()
                .append(true)
                .open(&temp_path)
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

            let chunk =
                chunk.map_err(|e| format!("Download stream error for {}: {e}", filename))?;
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
                "speech_model_download_progress",
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
        log::info!(
            "STT file {} downloaded to {}",
            filename,
            dest_path.display()
        );
    }

    let vad_path = models_dir.join(crate::stt::whisper::SILERO_VAD_FILENAME);
    if !vad_path.exists() {
        log::info!("Downloading Silero VAD model...");
        let client = reqwest::Client::new();
        let resp = client
            .get(crate::stt::whisper::SILERO_VAD_URL)
            .send()
            .await
            .map_err(|e| format!("Silero VAD download failed: {e}"))?;
        if !resp.status().is_success() {
            log::warn!("Silero VAD download failed with status: {}", resp.status());
        } else {
            let bytes = resp
                .bytes()
                .await
                .map_err(|e| format!("Silero VAD read failed: {e}"))?;
            std::fs::write(&vad_path, &bytes)
                .map_err(|e| format!("Failed to write Silero VAD: {e}"))?;
            log::info!("Silero VAD downloaded to {}", vad_path.display());
        }
    }

    let _ = app_handle.emit(
        "speech_model_download_progress",
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
pub async fn check_speech_model_file_exists(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<SpeechModelFileStatus, String> {
    let model_name = selected_speech_model_name(state.inner());
    let dir_name = crate::stt::whisper::stt_model_dir_name(&model_name);

    let models_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e: tauri::Error| e.to_string())?
        .join("models");

    let model_dir = models_dir.join(&dir_name);

    if model_dir.is_dir() {
        let has_onnx = std::fs::read_dir(&model_dir)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .any(|e| e.file_name().to_string_lossy().ends_with(".onnx"))
            })
            .unwrap_or(false);

        if has_onnx {
            let total_size: u64 = std::fs::read_dir(&model_dir)
                .map(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .map(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
                        .sum()
                })
                .unwrap_or(0);

            return Ok(SpeechModelFileStatus {
                exists: true,
                size_mb: total_size / (1024 * 1024),
                model_name,
            });
        }
    }

    Ok(SpeechModelFileStatus {
        exists: false,
        size_mb: 0,
        model_name,
    })
}

/// Delete the STT model directory and unload from AppState.
#[tauri::command]
pub async fn delete_speech_model(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let model = selected_speech_model_name(state.inner());

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

/// Reload speech models from disk into AppState.
#[tauri::command]
pub async fn reload_models(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut errors: Vec<String> = Vec::new();

    let speech_model_loaded = state.whisper.lock().map(|g| g.is_some()).unwrap_or(false);
    println!("RELOAD: STT currently loaded: {speech_model_loaded}");

    if !speech_model_loaded {
        let model_setting = {
            let db = state
                .db
                .lock()
                .map_err(|e| format!("DB lock failed: {e}"))?;
            crate::db::queries::get_setting(&db, "speech_model")
                .unwrap_or_else(|_| crate::stt::whisper::DEFAULT_WHISPER_MODEL.to_string())
        };
        let candidates = crate::speech_model_candidates(&app_handle, &model_setting);
        println!(
            "RELOAD: STT model setting='{}', candidates: {:?}",
            model_setting,
            candidates
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
        );

        let whisper_mutex = state.whisper.clone();

        let whisper_result = tokio::task::spawn_blocking(move || {
            for candidate in &candidates {
                if candidate.exists() && candidate.is_dir() {
                    println!("RELOAD: Trying STT at {}", candidate.display());
                    match crate::stt::whisper::WhisperEngine::new(candidate) {
                        Ok(engine) => {
                            println!(
                                "RELOAD: STT loaded successfully from {}",
                                candidate.display()
                            );
                            log::info!("Hot-loaded STT engine from {}", candidate.display());
                            if let Ok(mut g) = whisper_mutex.lock() {
                                *g = Some(std::sync::Arc::new(engine));
                            }
                            return Ok(());
                        }
                        Err(e) => {
                            println!("RELOAD: STT load FAILED from {}: {e}", candidate.display());
                            log::warn!("Failed to load STT from {}: {e}", candidate.display());
                            return Err(format!(
                                "Failed to load STT from {}: {e}",
                                candidate.display()
                            ));
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

    let vad_loaded = state.vad.lock().map(|g| g.is_some()).unwrap_or(false);

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
            } else {
                println!("RELOAD: Silero VAD loaded successfully");
            }
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
    let speech_model_loaded = state.whisper.lock().map(|g| g.is_some()).unwrap_or(false);

    Ok(ModelsStatus {
        speech_model_loaded,
    })
}
