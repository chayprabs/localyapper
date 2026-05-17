// Idle-driven model eviction.
//
// Parakeet (`OfflineRecognizer`) and Silero VAD together pin hundreds of
// megabytes of ONNX weights in process memory. The pipeline uses them in
// short bursts; in between, holding them resident is pure waste.
//
// This module exposes:
//
// * `ModelLifecycle::mark_used()`   — call when a transcription starts.
// * `ModelLifecycle::schedule_evict()` — call when a transcription ends.
// * `ModelLifecycle::evict_now()`   — force eviction (settings change, delete).
//
// The eviction task wakes once per scheduling, checks that no use has
// happened since, and drops the engines from `AppState`. If a new
// transcription begins before the timer fires, `mark_used()` invalidates
// the pending run by bumping a generation counter.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};

use crate::state::AppState;

/// Default idle period before models are unloaded.
pub const DEFAULT_IDLE_UNLOAD_SECONDS: u64 = 60;

/// Settings key that overrides the default idle timeout.
pub const IDLE_UNLOAD_SETTING_KEY: &str = "idle_unload_seconds";

/// Event name emitted when the model loaded/unloaded state changes.
pub const MODEL_STATE_EVENT: &str = "model-state";

#[derive(Clone)]
pub struct ModelLifecycle {
    inner: Arc<ModelLifecycleInner>,
}

struct ModelLifecycleInner {
    /// Bumped on every `mark_used()` and `schedule_evict()` call so any
    /// in-flight eviction task can detect that it has been superseded.
    generation: AtomicU64,
    /// Track whether the current resident state is loaded/unloaded so we
    /// only emit `model-state` transitions, not duplicates.
    last_emitted_loaded: Mutex<Option<bool>>,
}

impl ModelLifecycle {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(ModelLifecycleInner {
                generation: AtomicU64::new(0),
                last_emitted_loaded: Mutex::new(None),
            }),
        }
    }

    /// Cancels any pending eviction by bumping the generation counter.
    /// Call at the start of every pipeline run.
    pub fn mark_used(&self) {
        self.inner.generation.fetch_add(1, Ordering::SeqCst);
    }

    /// Schedule eviction after `idle_seconds` of inactivity. Reads the
    /// override from settings if available, otherwise uses the default.
    /// Cancels any previously scheduled eviction.
    pub fn schedule_evict(&self, app: AppHandle) {
        let idle_seconds = resolve_idle_seconds(&app);
        if idle_seconds == 0 {
            return;
        }

        let gen_when_scheduled = self.inner.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let inner = self.inner.clone();
        let lifecycle = self.clone();

        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_secs(idle_seconds)).await;

            if inner.generation.load(Ordering::SeqCst) != gen_when_scheduled {
                return;
            }

            lifecycle.evict_now(&app);
        });
    }

    /// Immediately clear the speech engine and VAD from `AppState`. Safe to
    /// call from any thread. Emits the `MODEL_STATE_EVENT` if the loaded
    /// state actually changes.
    pub fn evict_now(&self, app: &AppHandle) {
        self.inner.generation.fetch_add(1, Ordering::SeqCst);

        let Some(state) = app.try_state::<AppState>() else {
            return;
        };

        let mut changed = false;
        if let Ok(mut guard) = state.whisper.lock() {
            if guard.take().is_some() {
                changed = true;
            }
        }
        if let Ok(mut guard) = state.vad.lock() {
            if guard.take().is_some() {
                changed = true;
            }
        }

        if changed {
            log::info!("Idle eviction: dropped STT + VAD from memory");
            self.emit_state(app, false);
        }
    }

    /// Emit a `model-state` event with the given loaded flag. Deduplicates
    /// so the frontend only sees real transitions.
    pub fn emit_state(&self, app: &AppHandle, loaded: bool) {
        if let Ok(mut last) = self.inner.last_emitted_loaded.lock() {
            if *last == Some(loaded) {
                return;
            }
            *last = Some(loaded);
        }
        let _ = app.emit(
            MODEL_STATE_EVENT,
            ModelStatePayload {
                loaded,
                state: if loaded { "loaded" } else { "unloaded" }.to_string(),
            },
        );
    }
}

impl Default for ModelLifecycle {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, serde::Serialize)]
pub struct ModelStatePayload {
    pub loaded: bool,
    /// Coarse state string: "loaded" | "unloaded" | "loading".
    pub state: String,
}

fn resolve_idle_seconds(app: &AppHandle) -> u64 {
    let Some(state) = app.try_state::<AppState>() else {
        return DEFAULT_IDLE_UNLOAD_SECONDS;
    };
    let Ok(conn) = state.db.lock() else {
        return DEFAULT_IDLE_UNLOAD_SECONDS;
    };
    crate::db::queries::get_setting(&conn, IDLE_UNLOAD_SETTING_KEY)
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(DEFAULT_IDLE_UNLOAD_SECONDS)
}
