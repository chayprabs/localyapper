// Application state -- shared Tauri state container
use crate::audio::capture::AudioRecorder;
use crate::audio::vad::SileroVad;
use crate::stt::lifecycle::ModelLifecycle;
use crate::stt::whisper::WhisperEngine;
use rusqlite::Connection;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

/// State machine mode: no active recording, ready for input.
pub const HOTKEY_MODE_IDLE: u8 = 0;
/// State machine mode: key is held down, recording in progress.
pub const HOTKEY_MODE_HOLD_RECORDING: u8 = 1;
/// State machine mode: hands-free toggle recording is active.
pub const HOTKEY_MODE_HANDS_FREE: u8 = 2;
/// State machine mode: recording stopped, pipeline is running.
pub const HOTKEY_MODE_PROCESSING: u8 = 3;
/// State machine mode: a quick tap was released; waiting for a second tap.
pub const HOTKEY_MODE_TAP_PENDING: u8 = 4;

/// Shared hotkey state machine tracking recording mode.
pub struct HotkeyState {
    pub mode: AtomicU8,
    pub press_time_ms: AtomicI64,
}

impl HotkeyState {
    pub fn new() -> Self {
        Self {
            mode: AtomicU8::new(HOTKEY_MODE_IDLE),
            press_time_ms: AtomicI64::new(0),
        }
    }

    pub fn is_recording(&self) -> bool {
        matches!(
            self.mode.load(Ordering::SeqCst),
            HOTKEY_MODE_HOLD_RECORDING | HOTKEY_MODE_TAP_PENDING | HOTKEY_MODE_HANDS_FREE
        )
    }
}

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
    /// Focused app name captured when recording starts, before the overlay shows.
    pub recording_target_app: Arc<Mutex<Option<String>>>,
    /// Normalized speech model id currently loaded in `whisper`, if any.
    pub loaded_speech_model: Arc<Mutex<Option<String>>>,
    /// Shared hotkey state machine — survives hotkey reloads.
    pub hotkey_state: Arc<HotkeyState>,
}
