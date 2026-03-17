use crate::audio::capture::AudioRecorder;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

/// Global application state managed by Tauri.
pub struct AppState {
    pub db: Arc<Mutex<Connection>>,
    pub recorder: Arc<AudioRecorder>,
}
