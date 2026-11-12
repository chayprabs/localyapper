//! Test-only hooks (feature `test-hooks`, never in release builds).
//!
//! - LOCALYAPPER_TEST_SETTINGS="key=value;key=value" is written to the
//!   settings table at start-up.
//! - LOCALYAPPER_TEST_PORT opens a line-based control channel on
//!   127.0.0.1 so smoke tests can press the record key, cancel, and read
//!   the controller state, stage timings and what the overlay page shows.
//! - The overlay page reports what it rendered through
//!   `test_overlay_seen` (it is told it runs under test by an
//!   initialization script that only test builds inject).

use crate::controller::{ControllerHandle, Msg};
use crate::core::session::Event;
use crate::platform::hotkey::HotkeyId;
use rusqlite::Connection;
use serde_json::json;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

#[derive(Default)]
struct Seen {
    levels: Vec<f32>,
    kinds: Vec<String>,
}

static SEEN: Mutex<Option<Seen>> = Mutex::new(None);

pub fn apply_test_settings(conn: &Connection) {
    let Ok(spec) = std::env::var("LOCALYAPPER_TEST_SETTINGS") else {
        return;
    };
    for pair in spec.split(';').filter(|p| !p.trim().is_empty()) {
        if let Some((k, v)) = pair.split_once('=') {
            if let Err(e) = crate::db::queries::set_setting(conn, k.trim(), v.trim()) {
                log::warn!("test settings: {k}: {e}");
            }
        }
    }
}

/// Called by the overlay page in test builds.
#[tauri::command]
pub fn test_overlay_seen(kind: Option<String>, level: Option<f32>) {
    if let Ok(mut g) = SEEN.lock() {
        let seen = g.get_or_insert_with(Seen::default);
        if let Some(k) = kind {
            seen.kinds.push(k);
        }
        if let Some(l) = level {
            if !seen.levels.iter().any(|v| (v - l).abs() < 1e-6) {
                seen.levels.push(l);
            }
        }
    }
}

fn seen_json() -> serde_json::Value {
    let g = SEEN.lock().ok();
    let seen = g.as_ref().and_then(|g| g.as_ref());
    json!({
        "distinct_levels": seen.map_or(0, |s| s.levels.len()),
        "page_kinds": seen.map(|s| s.kinds.clone()).unwrap_or_default(),
    })
}

fn reset_seen() {
    if let Ok(mut g) = SEEN.lock() {
        *g = Some(Seen::default());
    }
}

fn handle_line(line: &str, app: &AppHandle, ctl: &ControllerHandle) -> serde_json::Value {
    match line.trim() {
        "down" => {
            ctl.send(Msg::Session(Event::RecordDown {
                at_ms: ctl.now_ms(),
            }));
            json!({"ok": true})
        }
        "up" => {
            ctl.send(Msg::Session(Event::RecordUp {
                at_ms: ctl.now_ms(),
            }));
            json!({"ok": true})
        }
        "cancel" => {
            ctl.send(Msg::Session(Event::Cancel));
            json!({"ok": true})
        }
        "reset_seen" => {
            reset_seen();
            json!({"ok": true})
        }
        "hotkeys" => {
            // Re-register every default hotkey and the cancel key.
            let mut errors = Vec::new();
            if let Err(e) = crate::platform::hotkey::apply_settings(app) {
                errors.push(e);
            }
            let state = app.state::<crate::state::AppState>();
            let cancel = crate::platform::hotkey::default_combo(HotkeyId::Cancel);
            if let Err(e) = state.hotkeys.register(HotkeyId::Cancel, cancel) {
                errors.push(e);
            }
            let _ = state.hotkeys.unregister(HotkeyId::Cancel);
            json!({"ok": errors.is_empty(), "errors": errors, "backend": state.hotkeys.name()})
        }
        "ax" => {
            json!({"ok": true, "trusted": crate::platform::permissions::accessibility_trusted(false)})
        }
        "state" => {
            let snap = tauri::async_runtime::block_on(ctl.query());
            json!({"ok": snap.is_some(), "state": snap, "overlay": seen_json(), "pid": std::process::id()})
        }
        other => json!({"ok": false, "error": format!("unknown command {other}")}),
    }
}

/// Start the control channel if LOCALYAPPER_TEST_PORT is set.
pub fn start(app: AppHandle, ctl: ControllerHandle) {
    let Some(port) = std::env::var("LOCALYAPPER_TEST_PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
    else {
        return;
    };
    let listener = match TcpListener::bind(("127.0.0.1", port)) {
        Ok(l) => l,
        Err(e) => {
            log::error!("test hooks: cannot listen on {port}: {e}");
            return;
        }
    };
    log::info!("test hooks: control channel on 127.0.0.1:{port}");
    let _ = std::thread::Builder::new()
        .name("test-control".into())
        .spawn(move || {
            for stream in listener.incoming().flatten() {
                let ctl = ctl.clone();
                let app = app.clone();
                let _ = std::thread::Builder::new().spawn(move || {
                    let Ok(read) = stream.try_clone() else { return };
                    let mut write = stream;
                    for line in BufReader::new(read).lines().map_while(Result::ok) {
                        let reply = handle_line(&line, &app, &ctl);
                        if writeln!(write, "{reply}").is_err() {
                            break;
                        }
                    }
                });
            }
        });
}
