// IPC command handlers -- speech model download, status, and lifecycle
use std::sync::atomic::Ordering;

use futures_util::StreamExt;
use tauri::{Emitter, Manager};

use crate::models::{DownloadProgress, ModelsStatus, SpeechModelFileStatus};
use crate::state::AppState;
use crate::stt::whisper::WhisperEngine;

fn selected_speech_model_name(state: &AppState) -> String {
    let model = state
        .db
        .lock()
        .ok()
        .and_then(|db| crate::db::queries::get_setting(&db, "speech_model").ok())
        .unwrap_or_else(|| crate::stt::whisper::DEFAULT_STT_MODEL.to_string());

    crate::stt::whisper::normalize_stt_model_name(&model).to_string()
}

pub(crate) async fn ensure_speech_model_loaded(
    app_handle: &tauri::AppHandle,
    state: &AppState,
) -> Result<std::sync::Arc<WhisperEngine>, String> {
    let desired_model = selected_speech_model_name(state);

    if let Ok(guard) = state.whisper.lock() {
        if let Some(engine) = guard.as_ref() {
            let loaded = state
                .loaded_speech_model
                .lock()
                .ok()
                .and_then(|g| g.clone());
            if loaded.as_deref() == Some(desired_model.as_str()) {
                return Ok(engine.clone());
            }
        }
    }

    // Wrong model cached or none loaded — drop the stale engine first.
    if let Ok(mut guard) = state.whisper.lock() {
        if guard.take().is_some() {
            if let Ok(mut loaded) = state.loaded_speech_model.lock() {
                *loaded = None;
            }
        }
    }

    // First touch since startup or eviction: tell the frontend so it can
    // surface a brief "loading" indicator instead of looking frozen.
    let _ = app_handle.emit(
        crate::stt::lifecycle::MODEL_STATE_EVENT,
        crate::stt::lifecycle::ModelStatePayload {
            loaded: false,
            state: "loading".to_string(),
        },
    );

    let model_name = selected_speech_model_name(state);
    let model_name_for_load = model_name.clone();
    let load_handle = app_handle.clone();
    let loaded_engine = tokio::task::spawn_blocking(move || {
        crate::load_speech_model_from_setting(&load_handle, &model_name_for_load)
    })
    .await
    .map_err(|e| format!("STT load task panicked: {e}"))??;

    let mut guard = state
        .whisper
        .lock()
        .map_err(|e| format!("STT lock failed: {e}"))?;
    if guard.is_none() {
        *guard = Some(loaded_engine.clone());
        if let Ok(mut loaded) = state.loaded_speech_model.lock() {
            *loaded = Some(model_name);
        }
    }
    drop(guard);

    state.lifecycle.emit_state(app_handle, true);

    if let Ok(guard) = state.whisper.lock() {
        if let Some(engine) = guard.as_ref() {
            return Ok(engine.clone());
        }
    }
    Ok(loaded_engine)
}

/// Lazily load the Silero VAD model if a model file is present on disk.
/// Returns Ok(()) if Silero is loaded or unavailable (energy fallback is
/// always usable). Idempotent and cheap to call repeatedly.
pub(crate) async fn ensure_vad_loaded(
    app_handle: &tauri::AppHandle,
    state: &AppState,
) -> Result<(), String> {
    if state.vad.lock().map(|g| g.is_some()).unwrap_or(false) {
        return Ok(());
    }

    let vad_path = app_handle
        .path()
        .app_data_dir()
        .map_err(|e: tauri::Error| e.to_string())?
        .join("models")
        .join(crate::stt::whisper::SILERO_VAD_FILENAME);

    if !vad_path.exists() {
        return Ok(());
    }

    let vad_mutex = state.vad.clone();
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        match crate::audio::vad::SileroVad::new(&vad_path) {
            Ok(vad) => {
                if let Ok(mut g) = vad_mutex.lock() {
                    if g.is_none() {
                        *g = Some(vad);
                    }
                }
                Ok(())
            }
            Err(e) => Err(format!("Failed to load Silero VAD: {e}")),
        }
    })
    .await
    .map_err(|e| format!("VAD load task panicked: {e}"))??;

    Ok(())
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
        let min_size = crate::stt::whisper::minimum_valid_stt_model_file_size(filename);

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
        let downloaded_size = std::fs::metadata(&temp_path)
            .map_err(|e| format!("Failed to inspect downloaded {}: {e}", filename))?
            .len();
        if downloaded_size <= min_size {
            let _ = std::fs::remove_file(&temp_path);
            return Err(format!(
                "Downloaded {} was incomplete ({} bytes)",
                filename, downloaded_size
            ));
        }
        if dest_path.exists() {
            let metadata = std::fs::metadata(&dest_path)
                .map_err(|e| format!("Failed to inspect existing {}: {e}", filename))?;
            if metadata.is_file() {
                std::fs::remove_file(&dest_path)
                    .map_err(|e| format!("Failed to replace existing {}: {e}", filename))?;
            } else {
                return Err(format!(
                    "Cannot replace {} because the destination path is not a file",
                    filename
                ));
            }
        }
        std::fs::rename(&temp_path, &dest_path)
            .map_err(|e| format!("Failed to rename {}: {e}", filename))?;

        cumulative_downloaded += downloaded_size;
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

    if crate::stt::whisper::is_valid_stt_model_dir(&model_dir) {
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

    state.lifecycle.evict_now(&app_handle);

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

    let has_other_models = std::fs::read_dir(&models_dir)
        .map(|entries| {
            entries
                .filter_map(|entry| entry.ok())
                .any(|entry| crate::stt::whisper::is_valid_stt_model_dir(&entry.path()))
        })
        .unwrap_or(false);

    if !has_other_models {
        let vad_path = models_dir.join(crate::stt::whisper::SILERO_VAD_FILENAME);
        if vad_path.is_file() {
            std::fs::remove_file(&vad_path)
                .map_err(|e| format!("Failed to delete Silero VAD model: {e}"))?;
        }
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
    log::info!("RELOAD: STT currently loaded: {speech_model_loaded}");

    if let Ok(mut guard) = state.whisper.lock() {
        guard.take();
    }
    if let Ok(mut loaded) = state.loaded_speech_model.lock() {
        *loaded = None;
    }

    if let Err(e) = ensure_speech_model_loaded(&app_handle, state.inner()).await {
        errors.push(e);
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
                log::info!("RELOAD: Silero VAD loaded successfully");
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

/// Return the current model load state for the frontend `model-state`
/// listener to hydrate on mount.
#[tauri::command]
pub async fn get_model_state(
    state: tauri::State<'_, AppState>,
) -> Result<crate::stt::lifecycle::ModelStatePayload, String> {
    let loaded = state.whisper.lock().map(|g| g.is_some()).unwrap_or(false);
    Ok(crate::stt::lifecycle::ModelStatePayload {
        loaded,
        state: if loaded { "loaded" } else { "unloaded" }.to_string(),
    })
}
