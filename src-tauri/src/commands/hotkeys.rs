// IPC command handlers -- hotkey updates through the typed settings path
use std::collections::HashMap;

use tauri::AppHandle;

use crate::commands::settings::set_setting_now;
use crate::state::AppState;

const HOTKEY_KEYS: &[&str] = &[
    "hotkey_record",
    "hotkey_cancel",
    "hotkey_paste_last",
    "hotkey_open_app",
];

/// Change one hotkey. Registration failures roll back to the old combo.
#[tauri::command]
pub async fn update_hotkey(app: AppHandle, key: String, value: String) -> Result<(), String> {
    if !HOTKEY_KEYS.contains(&key.as_str()) {
        return Err(format!("Invalid hotkey key: {key}"));
    }
    set_setting_now(&app, &key, &value).map(|_| ())
}

/// Put every hotkey back to its default for this OS.
#[tauri::command]
pub async fn reset_hotkeys(app: AppHandle) -> Result<HashMap<String, String>, String> {
    // Clear first so defaults never collide with the old combos.
    let mut out = HashMap::new();
    for key in HOTKEY_KEYS {
        let spec = crate::settings::keys::find(key).ok_or("unknown hotkey")?;
        let value = crate::settings::keys::default_of(spec);
        set_setting_now(&app, key, &value)?;
        out.insert((*key).to_string(), value);
    }
    Ok(out)
}

/// While enabled, record-key presses emit `hotkey-detected` and start no
/// dictation. The wizard uses it to prove the global hotkey works.
#[tauri::command]
pub async fn set_hotkey_probe(
    state: tauri::State<'_, AppState>,
    enabled: bool,
) -> Result<(), String> {
    state
        .controller
        .send(crate::controller::Msg::HotkeyProbe(enabled));
    Ok(())
}
