// Application state -- shared Tauri state container
use crate::audio::capture::AudioRecorder;
use crate::correction::engine::CorrectionEngine;
use crate::llm::engine::LlmEngine;
use crate::stt::whisper::WhisperEngine;
use rusqlite::Connection;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

/// Global application state managed by Tauri.
pub struct AppState {
    /// SQLite connection shared across all IPC commands.
    pub db: Arc<Mutex<Connection>>,
    /// Audio recorder singleton — manages cpal stream lifecycle.
    pub recorder: Arc<AudioRecorder>,
    /// Hot-reloadable: locked briefly to clone the inner Arc, then released.
    pub whisper: Arc<Mutex<Option<Arc<WhisperEngine>>>>,
    /// Hot-reloadable: locked briefly to clone the inner Arc, then released.
    pub llm: Arc<Mutex<Option<Arc<LlmEngine>>>>,
    /// Most recent injected text, used by paste_last command.
    pub last_injection: Arc<Mutex<Option<String>>>,
    /// In-memory correction lookup, refreshed after learner writes.
    pub correction_engine: Arc<CorrectionEngine>,
    /// Signal flag to abort an in-progress model download.
    pub download_cancel: Arc<AtomicBool>,
    /// When true, hotkeys are disabled (dictation paused via tray menu).
    pub paused: Arc<AtomicBool>,
}
