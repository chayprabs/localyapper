// Application state -- shared Tauri state container
use crate::audio::capture::AudioRecorder;
use crate::audio::vad::SileroVad;
use crate::stt::lifecycle::ModelLifecycle;
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
    /// Silero VAD engine — loaded from models dir, optional (falls back to energy-based).
    pub vad: Arc<Mutex<Option<SileroVad>>>,
    /// Idle-driven model eviction coordinator. Wired in by the recording
    /// pipeline; held here so it is reachable from any IPC handler.
    pub lifecycle: ModelLifecycle,
    /// Most recent injected text, used by paste_last command.
    pub last_injection: Arc<Mutex<Option<String>>>,
    /// Signal flag to abort an in-progress model download.
    pub download_cancel: Arc<AtomicBool>,
    /// When true, hotkeys are disabled (dictation paused via tray menu).
    pub paused: Arc<AtomicBool>,
}
