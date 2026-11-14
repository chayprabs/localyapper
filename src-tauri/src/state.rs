//! Shared Tauri state.

use crate::controller::ControllerHandle;
use crate::core::stt::manager::ModelManager;
use crate::platform::hotkey::HotkeyBackend;
use rusqlite::Connection;
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
    /// Model downloads in progress: one at a time, each cancellable.
    pub downloads: crate::commands::models::Downloads,
    /// Global hotkey back end (shortcut plugin, or the Wayland portal).
    pub hotkeys: Arc<dyn HotkeyBackend>,
    /// Every setting, read from SQLite once at start and kept in step by
    /// `set_setting`, so hot paths never query the database (P5.1).
    pub settings: Arc<crate::settings::SettingsCache>,
}
