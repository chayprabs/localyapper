//! Windows: virtual-key paste (VK_V is layout independent) and a
//! modifier wait through GetAsyncKeyState.

use super::{Chord, Held, PasteError};
use enigo::{Enigo, Key, Keyboard, Settings};
use std::time::{Duration, Instant};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VIRTUAL_KEY, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
};

const VK_V: u32 = 0x56;
/// Unassigned virtual key. Tapping it while Alt or Win is held stops the
/// release of that key from opening the window menu or the Start menu
/// (the "menu mask key" trick).
const VK_MASK: u32 = 0xE8;

fn down(vks: &[VIRTUAL_KEY]) -> bool {
    vks.iter().any(|vk| {
        // SAFETY: GetAsyncKeyState only reads global key state.
        let state = unsafe { GetAsyncKeyState(i32::from(vk.0)) };
        state < 0
    })
}

pub fn wait_modifiers_up(timeout: Duration) {
    let start = Instant::now();
    let mut masked = false;
    while down(&[VK_CONTROL, VK_MENU, VK_SHIFT, VK_LWIN, VK_RWIN]) && start.elapsed() < timeout {
        if !masked && down(&[VK_MENU, VK_LWIN, VK_RWIN]) {
            masked = true;
            if let Ok(mut e) = Enigo::new(&Settings::default()) {
                let _ = e.key(Key::Other(VK_MASK), enigo::Direction::Click);
            }
        }
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
