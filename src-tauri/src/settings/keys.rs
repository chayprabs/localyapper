//! Every setting the app knows: type, default, validator and the hook
//! that applies a change (plan section 10). `set_setting` rejects any key
//! not listed here and any value its validator refuses.

use crate::platform::hotkey::{default_combo, HotkeyId};

/// How a value is checked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Bool,
    /// One of a fixed set of words.
    Choice(&'static [&'static str]),
    /// An integer in an inclusive range.
    Range(i64, i64),
    /// One of a fixed set of integers.
    IntSet(&'static [i64]),
    /// A hotkey such as "Ctrl+Alt+J".
    Combo,
    /// `default` or an input device name.
    Device,
    /// `auto` or a catalog model id.
    Model,
    /// Two fractions "0.50,0.85" (custom overlay position).
    Fraction2,
    /// Free text kept for the app's own bookkeeping.
    Text,
}

/// What must happen when a value changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hook {
    /// Re-read settings in the controller (session config, injector,
    /// audio device, cues, overlay, eviction, history).
    Controller,
    /// Re-register global hotkeys, with rollback on failure.
    Hotkeys,
    /// Enable or disable launch at login.
    Autostart,
    /// Tell both windows the theme changed.
    Theme,
    /// Evict the loaded model; the next dictation loads the new one.
    Speech,
    /// Prune history to the new retention.
    Retention,
    /// Stored only (wizard progress, UI state).
    Store,
}

#[derive(Clone, Copy, Debug)]
pub struct Key {
    pub name: &'static str,
    pub kind: Kind,
    pub default: &'static str,
    pub hook: Hook,
}

const fn k(name: &'static str, kind: Kind, default: &'static str, hook: Hook) -> Key {
    Key {
        name,
        kind,
        default,
        hook,
    }
}

pub const PASTE_METHODS: &[&str] = &[
    "auto",
    "ctrl_v",
    "ctrl_shift_v",
    "shift_insert",
    "type",
    "clipboard_only",
];

/// The record default differs per OS, so it is filled in by `default_of`.
pub const KEYS: &[Key] = &[
    // General
    k("auto_start", Kind::Bool, "false", Hook::Autostart),
    k(
        "theme",
        Kind::Choice(&["system", "light", "dark"]),
        "system",
        Hook::Theme,
    ),
    // Dictation
    k(
        "activation_mode",
        Kind::Choice(&["hold", "toggle", "hybrid"]),
        "hybrid",
        Hook::Controller,
    ),
    k("hotkey_record", Kind::Combo, "", Hook::Hotkeys),
    k("hotkey_cancel", Kind::Combo, "Escape", Hook::Hotkeys),
    k(
        "hotkey_paste_last",
        Kind::Combo,
        "Ctrl+Alt+J",
        Hook::Hotkeys,
    ),
    k("hotkey_open_app", Kind::Combo, "Ctrl+Alt+O", Hook::Hotkeys),
    k("cancel_key_enabled", Kind::Bool, "true", Hook::Controller),
    k(
        "cancel_key_in_hands_free",
        Kind::Bool,
        "false",
        Hook::Controller,
    ),
    k(
        "max_recording_seconds",
        Kind::Range(30, 3600),
        "600",
        Hook::Controller,
    ),
    k("trailing_space", Kind::Bool, "true", Hook::Controller),
    // Audio
    k("input_device", Kind::Device, "default", Hook::Controller),
    k("sound_cues", Kind::Bool, "false", Hook::Controller),
    // Output
    k(
        "paste_method",
        Kind::Choice(PASTE_METHODS),
        "auto",
        Hook::Controller,
    ),
    k("restore_clipboard", Kind::Bool, "true", Hook::Controller),
    k(
        "clipboard_restore_delay_ms",
        Kind::Range(100, 2000),
        "250",
        Hook::Controller,
    ),
    // Speech
    k("speech_model", Kind::Model, "auto", Hook::Speech),
    k(
        "idle_unload_seconds",
        Kind::IntSet(&[0, 30, 120, 300, 900]),
        "120",
        Hook::Controller,
    ),
    // Overlay
    k("overlay_enabled", Kind::Bool, "true", Hook::Controller),
    k(
        "overlay_position",
        Kind::Choice(&["bottom_center", "top_center", "custom"]),
        "bottom_center",
        Hook::Controller,
    ),
    k(
        "overlay_custom",
        Kind::Fraction2,
        "0.50,0.85",
        Hook::Controller,
    ),
    k("overlay_preview", Kind::Bool, "true", Hook::Controller),
    // Privacy
    k("history_enabled", Kind::Bool, "true", Hook::Controller),
    k(
        "history_retention_days",
        Kind::IntSet(&[0, 7, 30, 90]),
        "0",
        Hook::Retention,
    ),
    // App bookkeeping
    k("setup_complete", Kind::Bool, "false", Hook::Store),
    k(
        "setup_step",
        Kind::Choice(&[
            "welcome",
            "microphone",
            "accessibility",
            "hotkey",
            "files",
            "try",
            "done",
        ]),
        "welcome",
        Hook::Store,
    ),
    k("sidebar_collapsed", Kind::Bool, "false", Hook::Store),
    k("legacy_model_notice", Kind::Text, "", Hook::Store),
];

pub fn find(name: &str) -> Option<&'static Key> {
    KEYS.iter().find(|k| k.name == name)
}

/// The default value of a key on this OS.
pub fn default_of(key: &Key) -> String {
    if key.name == "hotkey_record" {
        default_combo(HotkeyId::Record).to_string()
    } else {
        key.default.to_string()
    }
}

const MODIFIERS: &[&str] = &[
    "ctrl",
    "control",
    "alt",
    "option",
    "shift",
    "super",
    "meta",
    "cmd",
    "command",
    "cmdorctrl",
];

fn valid_combo(v: &str) -> bool {
    let parts: Vec<&str> = v.split('+').map(str::trim).collect();
    if parts.iter().any(|p| p.is_empty()) || parts.len() > 4 {
        return false;
    }
    let Some((last, mods)) = parts.split_last() else {
        return false;
    };
    let mods_ok = mods
        .iter()
        .all(|m| MODIFIERS.contains(&m.to_lowercase().as_str()));
    // The main key must not itself be a modifier (bare modifiers are out
    // of scope, plan section 15).
    mods_ok && !MODIFIERS.contains(&last.to_lowercase().as_str())
}

/// Check and normalise a value for `key`. `models` lists the catalog ids
/// accepted by `Kind::Model`.
pub fn validate(key: &Key, value: &str, models: &[String]) -> Result<String, String> {
    let v = value.trim();
    let bad = || Err(format!("{v:?} is not a valid value for {}", key.name));
    match key.kind {
        Kind::Bool => match v.to_ascii_lowercase().as_str() {
            "true" | "1" => Ok("true".into()),
            "false" | "0" => Ok("false".into()),
            _ => bad(),
        },
        Kind::Choice(options) => {
            if options.contains(&v) {
                Ok(v.into())
            } else {
                bad()
            }
        }
        Kind::Range(lo, hi) => match v.parse::<i64>() {
            Ok(n) if (lo..=hi).contains(&n) => Ok(n.to_string()),
            _ => Err(format!(
                "{} must be a whole number from {lo} to {hi}",
                key.name
            )),
        },
        Kind::IntSet(options) => match v.parse::<i64>() {
            Ok(n) if options.contains(&n) => Ok(n.to_string()),
            _ => bad(),
        },
        Kind::Combo => {
            if valid_combo(v) {
                Ok(v.into())
            } else {
                Err(format!("{v:?} is not a key combination"))
            }
        }
        Kind::Device => {
            if v.is_empty() || v.len() > 256 {
                bad()
            } else {
                Ok(v.into())
            }
        }
        Kind::Model => {
            if v == "auto" || models.iter().any(|m| m == v) {
                Ok(v.into())
            } else {
                Err(format!("Unknown speech model {v:?}"))
            }
        }
        Kind::Fraction2 => {
            let parsed: Option<(f64, f64)> = v
                .split_once(',')
                .and_then(|(a, b)| Some((a.trim().parse().ok()?, b.trim().parse().ok()?)));
            match parsed {
                Some((x, y)) if (0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y) => {
                    Ok(format!("{x:.2},{y:.2}"))
                }
                _ => bad(),
            }
        }
        Kind::Text => {
            if v.len() <= 1024 {
                Ok(v.into())
            } else {
                bad()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn models() -> Vec<String> {
        vec!["parakeet-110m".into(), "moonshine-tiny-en".into()]
    }

    #[test]
    fn every_default_is_valid() {
        for key in KEYS {
            let d = default_of(key);
            if key.kind == Kind::Text {
                continue;
            }
            assert_eq!(
                validate(key, &d, &models()).as_deref(),
                Ok(d.as_str()),
                "{}",
                key.name
            );
        }
    }

    #[test]
    fn names_are_unique() {
        for (i, a) in KEYS.iter().enumerate() {
            assert!(KEYS[i + 1..].iter().all(|b| b.name != a.name), "{}", a.name);
        }
    }

    #[test]
    fn validators_reject_bad_values() {
        let get = |n| find(n).expect("key");
        assert!(validate(get("max_recording_seconds"), "29", &models()).is_err());
        assert_eq!(
            validate(get("max_recording_seconds"), " 3600 ", &models()).as_deref(),
            Ok("3600")
        );
        assert!(validate(get("idle_unload_seconds"), "60", &models()).is_err());
        assert!(validate(get("paste_method"), "ctrl+v", &models()).is_err());
        assert!(validate(get("speech_model"), "whisper", &models()).is_err());
        assert!(validate(get("hotkey_record"), "Ctrl", &models()).is_err());
        assert!(validate(get("hotkey_record"), "Ctrl++J", &models()).is_err());
        assert_eq!(
            validate(get("hotkey_record"), "Ctrl+Alt+J", &models()).as_deref(),
            Ok("Ctrl+Alt+J")
        );
        assert_eq!(
            validate(get("overlay_custom"), "0.5,0.9", &models()).as_deref(),
            Ok("0.50,0.90")
        );
        assert!(validate(get("overlay_custom"), "1.5,0.9", &models()).is_err());
        assert_eq!(
            validate(get("auto_start"), "TRUE", &models()).as_deref(),
            Ok("true")
        );
        assert!(find("whisper_model").is_none());
    }
}
