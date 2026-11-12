//! Windows: virtual-key paste (VK_V is layout independent) and a
//! modifier wait through GetAsyncKeyState.

use super::{Chord, Held, PasteError};
use enigo::{Enigo, Key, Keyboard, Settings};
use std::time::{Duration, Instant};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
};

const VK_V: u32 = 0x56;

fn any_modifier_down() -> bool {
    [VK_CONTROL, VK_MENU, VK_SHIFT, VK_LWIN, VK_RWIN]
        .iter()
        .any(|vk| {
            // SAFETY: GetAsyncKeyState only reads global key state.
            let state = unsafe { GetAsyncKeyState(i32::from(vk.0)) };
            state < 0
        })
}

pub fn wait_modifiers_up(timeout: Duration) {
    let start = Instant::now();
    while any_modifier_down() && start.elapsed() < timeout {
        std::thread::sleep(Duration::from_millis(10));
    }
}

pub fn send_chord(chord: Chord) -> Result<(), PasteError> {
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| PasteError::Failed(format!("keyboard unavailable: {e}")))?;
    let mut held = Held::new(&mut enigo);
    let r = match chord {
        Chord::PrimaryV => held
            .press(Key::Control)
            .and_then(|_| held.click(Key::Other(VK_V))),
        Chord::CtrlShiftV => held
            .press(Key::Control)
            .and_then(|_| held.press(Key::Shift))
            .and_then(|_| held.click(Key::Other(VK_V))),
        Chord::ShiftInsert => held.press(Key::Shift).and_then(|_| held.click(Key::Insert)),
    };
    r.map_err(PasteError::Failed)
}

pub fn type_text(text: &str) -> Result<(), String> {
    let mut enigo =
        Enigo::new(&Settings::default()).map_err(|e| format!("keyboard unavailable: {e}"))?;
    enigo.text(text).map_err(|e| format!("typing failed: {e}"))
}
