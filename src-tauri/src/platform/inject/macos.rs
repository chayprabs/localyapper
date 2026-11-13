//! macOS: Cmd+V by hardware key code (kVK_ANSI_V), modifier wait through
//! CGEventSourceFlagsState. Key events need the Accessibility grant.

use super::{Chord, Held, PasteError};
use enigo::{Enigo, Key, Keyboard, Settings};
use std::time::{Duration, Instant};

const KVK_ANSI_V: u16 = 9;
const HID_SYSTEM_STATE: i32 = 1;
const MASK_SHIFT: u64 = 0x0002_0000;
const MASK_CONTROL: u64 = 0x0004_0000;
const MASK_ALTERNATE: u64 = 0x0008_0000;
const MASK_COMMAND: u64 = 0x0010_0000;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventSourceFlagsState(state_id: i32) -> u64;
}

fn any_modifier_down() -> bool {
    // SAFETY: reads the global modifier flags; no pointers.
    let flags = unsafe { CGEventSourceFlagsState(HID_SYSTEM_STATE) };
    flags & (MASK_SHIFT | MASK_CONTROL | MASK_ALTERNATE | MASK_COMMAND) != 0
}

pub fn wait_modifiers_up(timeout: Duration) {
    let start = Instant::now();
    while any_modifier_down() && start.elapsed() < timeout {
        std::thread::sleep(Duration::from_millis(10));
    }
}

pub fn send_chord(chord: Chord) -> Result<(), PasteError> {
    if !crate::platform::permissions::accessibility_trusted(false) {
        return Err(PasteError::Unavailable(
            "Accessibility access is off, so the text was copied instead. Allow LocalYapper in System Settings > Privacy & Security > Accessibility.".into(),
        ));
    }
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| PasteError::Failed(format!("keyboard unavailable: {e}")))?;
    let mut held = Held::new(&mut enigo);
    let r = match chord {
        // No Insert key on a Mac: Shift+Insert falls back to Cmd+V.
        Chord::PrimaryV | Chord::ShiftInsert => held
            .press(Key::Meta)
            .and_then(|_| held.click_raw(KVK_ANSI_V)),
        Chord::CtrlShiftV => held
            .press(Key::Control)
            .and_then(|_| held.press(Key::Shift))
            .and_then(|_| held.click_raw(KVK_ANSI_V)),
    };
    r.map_err(PasteError::Failed)
}

pub fn type_text(text: &str) -> Result<(), String> {
    let mut enigo =
        Enigo::new(&Settings::default()).map_err(|e| format!("keyboard unavailable: {e}"))?;
    enigo.text(text).map_err(|e| format!("typing failed: {e}"))
}
