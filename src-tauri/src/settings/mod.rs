//! Typed settings (plan P5.1, P5.2).
//!
//! Values live in SQLite and in a cache behind an `RwLock` so hot paths
//! never query the database. Every write goes through `apply`: the key
//! must be known, the value must pass its validator, the change is
//! written, the key's hook runs, and `settings-changed` is emitted.
//! A hotkey that fails to register is rolled back.

pub mod keys;

use keys::{Hook, Key};
use std::collections::HashMap;
use std::sync::RwLock;

/// In-memory copy of the settings table.
#[derive(Default)]
pub struct SettingsCache(RwLock<HashMap<String, String>>);

impl SettingsCache {
    pub fn new(map: HashMap<String, String>) -> Self {
        Self(RwLock::new(map))
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.0.read().ok().and_then(|m| m.get(key).cloned())
    }

    /// The value, or the key's default when unset.
    pub fn get_or_default(&self, key: &str) -> String {
        self.get(key)
            .or_else(|| keys::find(key).map(keys::default_of))
            .unwrap_or_default()
    }

    pub fn snapshot(&self) -> HashMap<String, String> {
        self.0.read().map(|m| m.clone()).unwrap_or_default()
    }

    pub fn set(&self, key: &str, value: &str) {
        if let Ok(mut m) = self.0.write() {
            m.insert(key.to_string(), value.to_string());
        }
    }

    pub fn replace(&self, map: HashMap<String, String>) {
        if let Ok(mut m) = self.0.write() {
            *m = map;
        }
    }
}

/// Where values are stored (SQLite in the app, a map in tests).
pub trait Store {
    fn read(&self, key: &str) -> Option<String>;
    fn write(&mut self, key: &str, value: &str) -> Result<(), String>;
}

/// What hooks do (the app's windows, plugins and controller; a recorder
/// in tests).
pub trait Effects {
    fn reload_controller(&self);
    fn reload_hotkeys(&self) -> Result<(), String>;
    fn set_autostart(&self, enabled: bool) -> Result<(), String>;
    fn theme_changed(&self, theme: &str);
    fn speech_changed(&self);
    fn prune_history(&self, days: i64);
    fn changed(&self, key: &str, value: &str);
}

const HOTKEY_KEYS: [&str; 4] = [
    "hotkey_record",
    "hotkey_cancel",
    "hotkey_paste_last",
    "hotkey_open_app",
];

/// Reject a combo another action already uses.
fn ensure_unique_hotkey(store: &dyn Store, key: &str, value: &str) -> Result<(), String> {
    for other in HOTKEY_KEYS.iter().filter(|k| **k != key) {
        let existing = store
            .read(other)
            .or_else(|| keys::find(other).map(keys::default_of))
            .unwrap_or_default();
        if existing.eq_ignore_ascii_case(value) {
            let label = other.trim_start_matches("hotkey_").replace('_', " ");
            return Err(format!("{value} is already used by {label}"));
        }
    }
    Ok(())
}

/// Validate, store and apply one setting. Returns the stored value.
pub fn apply(
    store: &mut dyn Store,
    effects: &dyn Effects,
    key: &str,
    value: &str,
    models: &[String],
) -> Result<String, String> {
    let spec: &Key = keys::find(key).ok_or_else(|| format!("Unknown setting {key:?}"))?;
    let value = keys::validate(spec, value, models)?;
    let previous = store.read(key);
    if spec.hook == Hook::Hotkeys {
        ensure_unique_hotkey(store, key, &value)?;
    }
    store.write(key, &value)?;
    let result = match spec.hook {
        Hook::Controller => {
            effects.reload_controller();
            Ok(())
        }
        Hook::Hotkeys => match effects.reload_hotkeys() {
            Ok(()) => {
                effects.reload_controller();
                Ok(())
            }
            Err(e) => {
                // Roll back to the previous combo and register it again.
                let old = previous.clone().unwrap_or_else(|| keys::default_of(spec));
                store.write(key, &old)?;
                if let Err(e2) = effects.reload_hotkeys() {
                    log::error!("settings: could not restore hotkeys: {e2}");
                }
                Err(e)
            }
        },
        Hook::Autostart => effects.set_autostart(value == "true"),
        Hook::Theme => {
            effects.theme_changed(&value);
            Ok(())
        }
        Hook::Speech => {
            effects.speech_changed();
            effects.reload_controller();
            Ok(())
        }
        Hook::Retention => {
            effects.prune_history(value.parse().unwrap_or(0));
            Ok(())
        }
        Hook::Store => Ok(()),
    };
    if let Err(e) = result {
        if spec.hook == Hook::Autostart {
            if let Some(old) = &previous {
                store.write(key, old)?;
            }
        }
        return Err(e);
    }
    effects.changed(key, &value);
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller::ControllerSettings;
    use crate::core::session::ActivationMode;
    use crate::platform::inject::PasteMethod;
    use crate::platform::overlay::OverlayPosition;
    use std::cell::RefCell;

    #[derive(Default)]
    struct MapStore(HashMap<String, String>);

    impl Store for MapStore {
        fn read(&self, key: &str) -> Option<String> {
            self.0.get(key).cloned()
        }
        fn write(&mut self, key: &str, value: &str) -> Result<(), String> {
            self.0.insert(key.into(), value.into());
            Ok(())
        }
    }

    #[derive(Default)]
    struct Recorder {
        calls: RefCell<Vec<String>>,
        fail_hotkeys: RefCell<bool>,
    }

    impl Recorder {
        fn saw(&self, call: &str) -> bool {
            self.calls.borrow().iter().any(|c| c == call)
        }
    }

    impl Effects for Recorder {
        fn reload_controller(&self) {
            self.calls.borrow_mut().push("controller".into());
        }
        fn reload_hotkeys(&self) -> Result<(), String> {
            self.calls.borrow_mut().push("hotkeys".into());
            if *self.fail_hotkeys.borrow() {
                *self.fail_hotkeys.borrow_mut() = false;
                return Err("taken by another app".into());
            }
            Ok(())
        }
        fn set_autostart(&self, enabled: bool) -> Result<(), String> {
            self.calls.borrow_mut().push(format!("autostart:{enabled}"));
            Ok(())
        }
        fn theme_changed(&self, theme: &str) {
            self.calls.borrow_mut().push(format!("theme:{theme}"));
        }
        fn speech_changed(&self) {
            self.calls.borrow_mut().push("speech".into());
        }
        fn prune_history(&self, days: i64) {
            self.calls.borrow_mut().push(format!("prune:{days}"));
        }
        fn changed(&self, key: &str, value: &str) {
            self.calls
                .borrow_mut()
                .push(format!("changed:{key}={value}"));
        }
    }

    fn models() -> Vec<String> {
        vec!["parakeet-110m".into(), "moonshine-tiny-en".into()]
    }

    /// Apply `key = value` and return the recorder and the settings the
    /// controller would build from the store.
    fn set(key: &str, value: &str) -> (Recorder, ControllerSettings, MapStore) {
        let mut store = MapStore::default();
        if key == "overlay_custom" {
            store.0.insert("overlay_position".into(), "custom".into());
        }
        let rec = Recorder::default();
        apply(&mut store, &rec, key, value, &models()).expect("apply");
        assert!(
            rec.saw(&format!("changed:{key}={value}")),
            "{key}: no settings-changed"
        );
        let cs = ControllerSettings::from_map(&store.0);
        (rec, cs, store)
    }

    // One test per key: the hook runs and, where the controller reads the
    // key, the value reaches the settings it acts on.

    #[test]
    fn hook_auto_start() {
        assert!(set("auto_start", "true").0.saw("autostart:true"));
    }

    #[test]
    fn hook_theme() {
        assert!(set("theme", "dark").0.saw("theme:dark"));
    }

    #[test]
    fn hook_activation_mode() {
        let (r, cs, _) = set("activation_mode", "toggle");
        assert!(r.saw("controller"));
        assert_eq!(cs.session.mode, ActivationMode::Toggle);
    }

    #[test]
    fn hook_hotkey_record() {
        let (r, _, _) = set("hotkey_record", "F9");
        assert!(r.saw("hotkeys") && r.saw("controller"));
    }

    #[test]
    fn hook_hotkey_cancel() {
        let (r, cs, _) = set("hotkey_cancel", "Shift+Escape");
        assert!(r.saw("hotkeys") && r.saw("controller"));
        assert_eq!(cs.hotkey_cancel, "Shift+Escape");
    }

    #[test]
    fn hook_hotkey_paste_last() {
        assert!(set("hotkey_paste_last", "Ctrl+Alt+K").0.saw("hotkeys"));
    }

    #[test]
    fn hook_hotkey_open_app() {
        assert!(set("hotkey_open_app", "Ctrl+Alt+P").0.saw("hotkeys"));
    }

    #[test]
    fn hook_cancel_key_enabled() {
        let (r, cs, _) = set("cancel_key_enabled", "false");
        assert!(r.saw("controller"));
        assert!(!cs.session.cancel_key_enabled);
    }

    #[test]
    fn hook_cancel_key_in_hands_free() {
        let (r, cs, _) = set("cancel_key_in_hands_free", "true");
        assert!(r.saw("controller"));
        assert!(cs.session.cancel_key_in_hands_free);
    }

    #[test]
    fn hook_max_recording_seconds() {
        let (r, cs, _) = set("max_recording_seconds", "45");
        assert!(r.saw("controller"));
        assert_eq!(cs.session.max_recording_ms, 45_000);
    }

    #[test]
    fn hook_trailing_space() {
        let (r, cs, _) = set("trailing_space", "false");
        assert!(r.saw("controller"));
        assert!(!cs.trailing_space);
    }

    #[test]
    fn hook_input_device() {
        let (r, cs, _) = set("input_device", "USB Mic");
        assert!(r.saw("controller"));
        assert_eq!(cs.input_device, "USB Mic");
    }

    #[test]
    fn hook_sound_cues() {
        let (r, cs, _) = set("sound_cues", "true");
        assert!(r.saw("controller"));
        assert!(cs.sound_cues);
    }

    #[test]
    fn hook_paste_method() {
        let (r, cs, _) = set("paste_method", "type");
        assert!(r.saw("controller"));
        assert_eq!(cs.paste_method, PasteMethod::Type);
    }

    #[test]
    fn hook_restore_clipboard() {
        let (r, cs, _) = set("restore_clipboard", "false");
        assert!(r.saw("controller"));
        assert!(!cs.restore_clipboard);
    }

    #[test]
    fn hook_clipboard_restore_delay_ms() {
        let (r, cs, _) = set("clipboard_restore_delay_ms", "500");
        assert!(r.saw("controller"));
        assert_eq!(cs.clipboard_restore_delay_ms, 500);
    }

    #[test]
    fn hook_speech_model() {
        let (r, _, _) = set("speech_model", "moonshine-tiny-en");
        assert!(r.saw("speech") && r.saw("controller"));
    }

    #[test]
    fn hook_idle_unload_seconds() {
        let (r, cs, _) = set("idle_unload_seconds", "30");
        assert!(r.saw("controller"));
        assert_eq!(cs.idle_unload_seconds, 30);
    }

    #[test]
    fn hook_overlay_enabled() {
        let (r, cs, _) = set("overlay_enabled", "false");
        assert!(r.saw("controller"));
        assert!(!cs.overlay.enabled);
    }

    #[test]
    fn hook_overlay_position() {
        let (r, cs, _) = set("overlay_position", "top_center");
        assert!(r.saw("controller"));
        assert_eq!(cs.overlay.position, OverlayPosition::TopCenter);
    }

    #[test]
    fn hook_overlay_custom() {
        let (r, cs, _) = set("overlay_custom", "0.2500,0.7500");
        assert!(r.saw("controller"));
        assert_eq!(
            cs.overlay.position,
            OverlayPosition::Custom { fx: 0.25, fy: 0.75 }
        );
    }

    #[test]
    fn hook_overlay_preview() {
        let (r, cs, _) = set("overlay_preview", "false");
        assert!(r.saw("controller"));
        assert!(!cs.overlay.preview);
    }

    #[test]
    fn hook_history_enabled() {
        let (r, cs, _) = set("history_enabled", "false");
        assert!(r.saw("controller"));
        assert!(!cs.session.history_enabled);
    }

    #[test]
    fn hook_history_retention_days() {
        assert!(set("history_retention_days", "7").0.saw("prune:7"));
    }

    #[test]
    fn hook_setup_complete() {
        let (r, _, store) = set("setup_complete", "true");
        assert_eq!(store.read("setup_complete").as_deref(), Some("true"));
        assert!(!r.saw("controller"));
    }

    #[test]
    fn hook_setup_step() {
        assert_eq!(
            set("setup_step", "hotkey").2.read("setup_step").as_deref(),
            Some("hotkey")
        );
    }

    #[test]
    fn hook_sidebar_collapsed() {
        assert_eq!(
            set("sidebar_collapsed", "true")
                .2
                .read("sidebar_collapsed")
                .as_deref(),
            Some("true")
        );
    }

    #[test]
    fn hook_legacy_model_notice() {
        assert_eq!(
            set("legacy_model_notice", "shown")
                .2
                .read("legacy_model_notice")
                .as_deref(),
            Some("shown")
        );
    }

    #[test]
    fn every_key_has_a_hook_test() {
        let src = include_str!("mod.rs");
        for key in keys::KEYS {
            assert!(
                src.contains(&format!("fn hook_{}()", key.name)),
                "no hook test for {}",
                key.name
            );
        }
    }

    #[test]
    fn unknown_keys_and_bad_values_are_rejected() {
        let mut store = MapStore::default();
        let rec = Recorder::default();
        assert!(apply(&mut store, &rec, "whisper_model", "x", &models()).is_err());
        assert!(apply(&mut store, &rec, "max_recording_seconds", "5", &models()).is_err());
        assert!(store.0.is_empty());
        assert!(rec.calls.borrow().is_empty());
    }

    #[test]
    fn a_hotkey_that_fails_to_register_is_rolled_back() {
        let mut store = MapStore::default();
        store.0.insert("hotkey_record".into(), "F8".into());
        let rec = Recorder::default();
        *rec.fail_hotkeys.borrow_mut() = true;
        let err = apply(&mut store, &rec, "hotkey_record", "F9", &models());
        assert!(err.is_err());
        assert_eq!(store.read("hotkey_record").as_deref(), Some("F8"));
        assert_eq!(
            rec.calls
                .borrow()
                .iter()
                .filter(|c| *c == "hotkeys")
                .count(),
            2
        );
    }

    #[test]
    fn duplicate_hotkeys_are_refused() {
        let mut store = MapStore::default();
        let rec = Recorder::default();
        let err = apply(&mut store, &rec, "hotkey_record", "ctrl+alt+j", &models());
        assert!(err.is_err_and(|e| e.contains("paste last")));
    }
}
