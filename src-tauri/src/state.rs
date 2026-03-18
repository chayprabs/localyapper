use crate::audio::capture::AudioRecorder;
use crate::stt::whisper::WhisperEngine;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

/// Global application state managed by Tauri.
pub struct AppState {
    pub db: Arc<Mutex<Connection>>,
    pub recorder: Arc<AudioRecorder>,
    pub whisper: Option<Arc<WhisperEngine>>,
    pub last_injection: Arc<Mutex<Option<String>>>,
}
