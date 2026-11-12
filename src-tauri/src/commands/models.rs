// IPC command handlers -- speech model catalog, download, use and delete
use std::sync::atomic::Ordering;
use std::sync::OnceLock;

use serde::Serialize;
use tauri::Emitter;

use crate::core::download::{Downloader, FileSpec};
use crate::core::stt::catalog::{self, Manifest};
use crate::models::{DownloadProgress, ModelsStatus, SpeechModelFileStatus};
use crate::state::AppState;

/// The embedded manifest, parsed once.
pub(crate) fn manifest() -> Result<&'static Manifest, String> {
    static M: OnceLock<Result<Manifest, String>> = OnceLock::new();
    M.get_or_init(catalog::manifest)
        .as_ref()
        .map_err(|e| e.clone())
}

fn speech_setting(state: &AppState) -> String {
    state
        .db
        .lock()
        .ok()
        .and_then(|db| crate::db::queries::get_setting(&db, "speech_model").ok())
        .unwrap_or_else(|| "auto".to_string())
}

/// The model id the `speech_model` setting resolves to on this machine.
pub(crate) fn active_model_id(state: &AppState) -> Result<String, String> {
    Ok(catalog::resolve_id(
        &speech_setting(state),
        crate::platform::memory::total_ram_bytes(),
        manifest()?,
    ))
}

#[derive(Serialize, Clone, Debug)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub languages: Vec<String>,
    pub license: String,
    pub size_bytes: u64,
    /// `installed`, `partial` (a download was interrupted) or `missing`.
    pub state: String,
    /// The model dictation uses now.
    pub active: bool,
    /// What `auto` picks on this machine.
    pub recommended: bool,
    pub loaded: bool,
}

#[tauri::command]
pub async fn list_models(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ModelInfo>, String> {
    let m = manifest()?;
    let dir = crate::models_dir(&app_handle)?;
    let active = active_model_id(state.inner())?;
    let recommended = catalog::auto_choice(crate::platform::memory::total_ram_bytes(), m);
    let loaded = state.models.loaded_id();
    Ok(catalog::offered(m)
        .into_iter()
        .map(|(id, e)| {
            let mdir = catalog::model_dir(&dir, id);
            let installed = catalog::is_installed(&mdir, e) && catalog::marker_matches(&mdir, e);
            let partial = !installed
                && e.files.iter().any(|f| {
                    crate::core::download::part_path(&mdir.join(&f.name)).exists()
                        || mdir.join(&f.name).exists()
                });
            ModelInfo {
                id: id.clone(),
                name: e.name.clone(),
                languages: e.languages.clone(),
                license: e.license.clone(),
                size_bytes: e.total_size(),
                state: if installed {
                    "installed"
                } else if partial {
                    "partial"
                } else {
                    "missing"
                }
                .into(),
                active: *id == active,
                recommended: *id == recommended,
                loaded: loaded.as_deref() == Some(id.as_str()),
            }
        })
        .collect())
}

#[derive(Serialize, Clone)]
struct DownloadEvent {
    id: String,
    downloaded: u64,
    total: u64,
    percent: f64,
}

/// Download, verify and mark a model as installed. Resumes a partial
/// download. Emits `model-download-progress` with real byte totals.
#[tauri::command]
pub async fn download_model(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let m = manifest()?;
    let entry = m
        .models
        .get(&id)
        .filter(|e| !e.legacy)
        .ok_or_else(|| format!("Unknown speech model: {id}"))?;
    let dir = catalog::model_dir(&crate::models_dir(&app_handle)?, &id);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Could not create {}: {e}", dir.display()))?;
    let files: Vec<FileSpec> = entry
        .files
        .iter()
        .map(|f| FileSpec {
            url: f.url.clone(),
            size: f.size,
            sha256: f.sha256.clone(),
            dest: dir.join(&f.name),
        })
        .collect();
    state.download_cancel.store(false, Ordering::SeqCst);
    let _ = std::fs::remove_file(dir.join(catalog::VERIFIED_MARKER));
    let downloader = Downloader::new()?;
    let app = app_handle.clone();
    let event_id = id.clone();
    let started = std::time::Instant::now();
    downloader
        .fetch_all(&files, &state.download_cancel, &move |p| {
            let secs = started.elapsed().as_secs_f64().max(0.001);
            let _ = app.emit(
                "speech_model_download_progress",
                DownloadProgress {
                    percent: if p.total > 0 {
                        100.0 * p.downloaded as f64 / p.total as f64
                    } else {
                        0.0
                    },
                    downloaded_mb: p.downloaded / 1_000_000,
                    total_mb: p.total / 1_000_000,
                    speed_mbps: p.downloaded as f64 / 1e6 / secs,
                },
            );
            let _ = app.emit(
                "model-download-progress",
                DownloadEvent {
                    id: event_id.clone(),
                    downloaded: p.downloaded,
                    total: p.total,
                    percent: if p.total > 0 {
                        100.0 * p.downloaded as f64 / p.total as f64
                    } else {
                        0.0
                    },
                },
            );
        })
        .await
        .map_err(|e| e.to_string())?;
    std::fs::write(
        dir.join(catalog::VERIFIED_MARKER),
        catalog::marker_text(entry),
    )
    .map_err(|e| format!("Could not finish the install: {e}"))?;
    log::info!("models: {id} downloaded and verified");
    let _ = app_handle.emit("models-changed", ());
    Ok(())
}

/// Cancel an in-progress model download. The partial file is kept.
#[tauri::command]
pub async fn cancel_model_download(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.download_cancel.store(true, Ordering::SeqCst);
    Ok(())
}

/// Make `id` (or `auto`) the dictation model. The old model is evicted.
#[tauri::command]
pub async fn use_model(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let m = manifest()?;
    if id != "auto" && !m.models.get(&id).is_some_and(|e| !e.legacy) {
        return Err(format!("Unknown speech model: {id}"));
    }
    {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        crate::db::queries::set_setting(&conn, "speech_model", &id).map_err(|e| e.to_string())?;
    }
    let models = state.models.clone();
    tokio::task::spawn_blocking(move || models.evict_blocking())
        .await
        .map_err(|e| e.to_string())?;
    let _ = app_handle.emit("models-changed", ());
    Ok(())
}

/// Delete a model's files. Deleting the active model returns the app to
/// "needs download".
#[tauri::command]
pub async fn delete_model(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let m = manifest()?;
    if !m.models.contains_key(&id) {
        return Err(format!("Unknown speech model: {id}"));
    }
    if state.models.loaded_id().as_deref() == Some(id.as_str()) {
        let models = state.models.clone();
        tokio::task::spawn_blocking(move || models.evict_blocking())
            .await
            .map_err(|e| e.to_string())?;
    }
    let dir = catalog::model_dir(&crate::models_dir(&app_handle)?, &id);
    if dir.is_dir() {
        std::fs::remove_dir_all(&dir).map_err(|e| format!("Could not delete {id}: {e}"))?;
    }
    log::info!("models: {id} deleted");
    let _ = app_handle.emit("models-changed", ());
    Ok(())
}

/// Model load state payload for the frontend `model-state` listener.
#[derive(Serialize, Clone)]
pub struct ModelStatePayload {
    pub loaded: bool,
    pub state: String,
}

/// Current model load state, for the `model-state` listener to hydrate.
#[tauri::command]
pub async fn get_model_state(
    state: tauri::State<'_, AppState>,
) -> Result<ModelStatePayload, String> {
    let loaded = state.models.loaded_id().is_some();
    Ok(ModelStatePayload {
        loaded,
        state: if loaded { "loaded" } else { "unloaded" }.to_string(),
    })
}

/// Upgrade path for installs made before the catalog (P4.6): the old
/// `parakeet-tdt-ctc-110m` folder becomes `parakeet-110m` when its files
/// match the manifest, and is hash-checked once in the background.
pub(crate) fn migrate_legacy_install(models_dir: &std::path::Path) {
    let Ok(m) = manifest() else { return };
    let Some(entry) = m.models.get(catalog::DEFAULT_MODEL) else {
        return;
    };
    let legacy = models_dir.join(catalog::LEGACY_DIR);
    let target = catalog::model_dir(models_dir, catalog::DEFAULT_MODEL);
    if !legacy.is_dir() || target.exists() {
        return;
    }
    let sizes_match = entry
        .files
        .iter()
        .all(|f| std::fs::metadata(legacy.join(&f.name)).is_ok_and(|md| md.len() == f.size));
    if !sizes_match {
        log::info!("models: legacy folder does not match the catalog; leaving it");
        return;
    }
    if let Err(e) = std::fs::rename(&legacy, &target) {
        log::warn!("models: could not move the legacy install: {e}");
        return;
    }
    let entry = entry.clone();
    std::thread::spawn(move || {
        let ok = entry.files.iter().all(|f| {
            crate::core::download::sha256_file(&target.join(&f.name)).is_ok_and(|h| h == f.sha256)
        });
        if ok {
            let _ = std::fs::write(
                target.join(catalog::VERIFIED_MARKER),
                catalog::marker_text(&entry),
            );
            log::info!("models: legacy install verified and kept");
        } else {
            log::warn!("models: legacy install failed its checksum; it must be downloaded again");
        }
    });
}

// Commands for the wizard and dashboard, which work on the active model.

/// Download the model the `speech_model` setting resolves to.
#[tauri::command]
pub async fn download_speech_model(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let id = active_model_id(state.inner())?;
    download_model(app_handle, state, id).await
}

/// Whether the active model is installed.
#[tauri::command]
pub async fn check_speech_model_file_exists(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<SpeechModelFileStatus, String> {
    let m = manifest()?;
    let id = active_model_id(state.inner())?;
    let entry = m.models.get(&id).ok_or("unknown model")?;
    let dir = catalog::model_dir(&crate::models_dir(&app_handle)?, &id);
    let exists = catalog::is_installed(&dir, entry) && catalog::marker_matches(&dir, entry);
    Ok(SpeechModelFileStatus {
        exists,
        size_mb: entry.total_size() / 1_000_000,
        model_name: id,
    })
}

/// Delete the active model.
#[tauri::command]
pub async fn delete_speech_model(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let id = active_model_id(state.inner())?;
    delete_model(app_handle, state, id).await
}

/// Drop the loaded model and load the active one again.
#[tauri::command]
pub async fn reload_models(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let spec = crate::resolve_model_spec(&app_handle)?;
    let models = state.models.clone();
    tokio::task::spawn_blocking(move || {
        models.evict_blocking();
        models.ensure_loaded_blocking(&spec).map(|_| ())
    })
    .await
    .map_err(|e| format!("Reload task failed: {e}"))?
}

/// Whether a model is loaded.
#[tauri::command]
pub async fn check_models_status(
    state: tauri::State<'_, AppState>,
) -> Result<ModelsStatus, String> {
    Ok(ModelsStatus {
        speech_model_loaded: state.models.loaded_id().is_some(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_folder_becomes_the_default_model() {
        let m = manifest().expect("manifest");
        let entry = &m.models[catalog::DEFAULT_MODEL];
        let root = std::env::temp_dir().join(format!("ly-legacy-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let legacy = root.join(catalog::LEGACY_DIR);
        std::fs::create_dir_all(&legacy).expect("dir");
        for f in &entry.files {
            let file = std::fs::File::create(legacy.join(&f.name)).expect("file");
            file.set_len(f.size).expect("size");
        }
        migrate_legacy_install(&root);
        let target = catalog::model_dir(&root, catalog::DEFAULT_MODEL);
        assert!(!legacy.exists());
        assert!(entry.files.iter().all(|f| target.join(&f.name).is_file()));
        // Zero-filled files fail the hash check, so no marker appears and
        // the model still counts as not downloaded.
        std::thread::sleep(std::time::Duration::from_secs(3));
        assert!(!catalog::marker_matches(&target, entry));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_legacy_folder_with_other_files_is_left_alone() {
        let root = std::env::temp_dir().join(format!("ly-legacy2-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let legacy = root.join(catalog::LEGACY_DIR);
        std::fs::create_dir_all(&legacy).expect("dir");
        std::fs::write(legacy.join("model.onnx"), b"short").expect("file");
        migrate_legacy_install(&root);
        assert!(legacy.exists());
        let _ = std::fs::remove_dir_all(&root);
    }
}
