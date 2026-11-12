//! `tauri-plugin-global-shortcut` back end for Windows, macOS and X11.
//!
//! On Linux the record key passes through `RepeatFilter` (Appendix C):
//! X11 auto-repeat sends Release/Press pairs while a key is held, which
//! would end hold-to-talk after the keyboard repeat delay (F21).

use super::{dispatch, HotkeyBackend, HotkeyId};
use crate::controller::ControllerHandle;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

pub struct ShortcutBackend {
    app: AppHandle,
    ctl: ControllerHandle,
    registered: Mutex<HashMap<HotkeyId, String>>,
    warning: Option<String>,
    #[cfg(target_os = "linux")]
    gate: std::sync::Arc<linux::RepeatGate>,
}

impl ShortcutBackend {
    pub fn new(app: AppHandle, ctl: ControllerHandle, warning: Option<String>) -> Self {
        Self {
            #[cfg(target_os = "linux")]
            gate: std::sync::Arc::new(linux::RepeatGate::new(ctl.clone())),
            app,
            ctl,
            registered: Mutex::new(HashMap::new()),
            warning,
        }
    }
}

impl HotkeyBackend for ShortcutBackend {
    fn register(&self, id: HotkeyId, combo: &str) -> Result<(), String> {
        self.unregister(id)?;
        let ctl = self.ctl.clone();
        #[cfg(target_os = "linux")]
        let gate = self.gate.clone();
        self.app
            .global_shortcut()
            .on_shortcut(combo, move |_app, _shortcut, event| {
                let pressed = event.state == ShortcutState::Pressed;
                let at_ms = ctl.now_ms();
                #[cfg(target_os = "linux")]
                if id == HotkeyId::Record {
                    gate.on_key(pressed, at_ms);
                    return;
                }
                dispatch(&ctl, id, pressed, at_ms);
            })
            .map_err(|e| format!("Failed to register {} hotkey '{combo}': {e}", id.label()))?;
        if let Ok(mut r) = self.registered.lock() {
            r.insert(id, combo.to_string());
        }
        Ok(())
    }

    fn unregister(&self, id: HotkeyId) -> Result<(), String> {
        let previous = self.registered.lock().ok().and_then(|mut r| r.remove(&id));
        if let Some(combo) = previous {
            self.app
                .global_shortcut()
                .unregister(combo.as_str())
                .map_err(|e| format!("Failed to unregister '{combo}': {e}"))?;
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "global-shortcut"
    }

    fn warning(&self) -> Option<String> {
        self.warning.clone()
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use super::super::repeat_filter::{Key, RepeatFilter};
    use super::{dispatch, HotkeyId};
    use crate::controller::ControllerHandle;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    /// Holds a release back for 30 ms; a 10 ms poll runs only while a
    /// release is pending.
    pub struct RepeatGate {
        filter: Arc<Mutex<RepeatFilter>>,
        ctl: ControllerHandle,
        polling: Arc<AtomicBool>,
    }

    fn forward(ctl: &ControllerHandle, keys: Vec<Key>) {
        for k in keys {
            match k {
                Key::Down { at_ms } => dispatch(ctl, HotkeyId::Record, true, at_ms),
                Key::Up { at_ms } => dispatch(ctl, HotkeyId::Record, false, at_ms),
            }
        }
    }

    impl RepeatGate {
        pub fn new(ctl: ControllerHandle) -> Self {
            Self {
                filter: Arc::new(Mutex::new(RepeatFilter::new(30))),
                ctl,
                polling: Arc::new(AtomicBool::new(false)),
            }
        }

        pub fn on_key(&self, pressed: bool, at_ms: u64) {
            // The polling flag only changes under the filter lock, so a
            // pending release can never be left without a poll thread.
            let (out, start_poll) = match self.filter.lock() {
                Ok(mut f) => {
                    let out = if pressed {
                        f.on_press(at_ms)
                    } else {
                        f.on_release(at_ms)
                    };
                    let pending = f.next_deadline().is_some();
                    (out, pending && !self.polling.swap(true, Ordering::AcqRel))
                }
                Err(_) => return,
            };
            forward(&self.ctl, out);
            if start_poll {
                self.spawn_poll();
            }
        }

        fn spawn_poll(&self) {
            let (filter, ctl, polling) =
                (self.filter.clone(), self.ctl.clone(), self.polling.clone());
            let spawned = std::thread::Builder::new()
                .name("hotkey-repeat".into())
                .spawn(move || loop {
                    std::thread::sleep(Duration::from_millis(10));
                    let (out, done) = match filter.lock() {
                        Ok(mut f) => {
                            let out = f.poll(ctl.now_ms());
                            let done = f.next_deadline().is_none();
                            if done {
                                polling.store(false, Ordering::Release);
                            }
                            (out, done)
                        }
                        Err(_) => {
                            polling.store(false, Ordering::Release);
                            (Vec::new(), true)
                        }
                    };
                    forward(&ctl, out);
                    if done {
                        return;
                    }
                });
            if spawned.is_err() {
                self.polling.store(false, Ordering::Release);
            }
        }
    }
}
