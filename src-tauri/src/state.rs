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
    pub db: Arc<Mutex<Connection>>,
    pub recorder: Arc<AudioRecorder>,
    /// Hot-reloadable: locked briefly to clone the inner Arc, then released.
    pub whisper: Arc<Mutex<Option<Arc<WhisperEngine>>>>,
    /// Hot-reloadable: locked briefly to clone the inner Arc, then released.
    pub llm: Arc<Mutex<Option<Arc<LlmEngine>>>>,
    pub last_injection: Arc<Mutex<Option<String>>>,
    pub correction_engine: Arc<CorrectionEngine>,
    pub download_cancel: Arc<AtomicBool>,
    /// When true, hotkeys are disabled (dictation paused via tray menu).
    pub paused: Arc<AtomicBool>,
}
