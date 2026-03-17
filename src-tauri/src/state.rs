use rusqlite::Connection;
use std::sync::{Arc, Mutex};

/// Global application state managed by Tauri.
pub struct AppState {
    pub db: Arc<Mutex<Connection>>,
}
