//! Shared Tauri state.

use crate::controller::ControllerHandle;
use crate::core::stt::manager::ModelManager;
use crate::platform::hotkey::shortcut::HotkeyId;
use rusqlite::Connection;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

pub struct AppState {
    /// SQLite connection shared by IPC commands and the controller.
    pub db: Arc<Mutex<Connection>>,
    /// Sends events to the controller, the only owner of dictation state.
    pub controller: ControllerHandle,
    /// The loaded recogniser, if any (read-only view for IPC).
    pub models: Arc<ModelManager>,
    /// Mirror of the controller's paused flag for IPC and the tray.
    pub paused: Arc<AtomicBool>,
    /// Most recent dictated text, for paste-last.
    pub last_injection: Arc<Mutex<Option<String>>>,
    /// Signal flag to abort an in-progress model download.
    pub download_cancel: Arc<AtomicBool>,
    /// Combos registered now, so reloads unregister exactly these.
    pub hotkeys: Mutex<HashMap<HotkeyId, String>>,
}
