//! Linux. X11: keys through enigo's xdo back end (never `raw()`, which
//! panics there) and a modifier wait through XQueryKeymap. Wayland: enigo's
//! virtual keyboard, then `wtype`, then `ydotool`; if none works the text
//! stays on the clipboard.

use super::{Chord, Held, PasteError};
use enigo::{Enigo, Key, Keyboard, Settings};
use std::process::Command;
use std::time::{Duration, Instant};

pub fn is_wayland() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some()
        || std::env::var("XDG_SESSION_TYPE").is_ok_and(|v| v.eq_ignore_ascii_case("wayland"))
}

/// Enigo bound to one back end, so a key is never sent twice (to the
/// Wayland compositor and to XWayland).
fn enigo() -> Result<Enigo, String> {
    let mut s = Settings::default();
    if is_wayland() {
        s.x11_display = Some(":localyapper-no-x11".into());
    } else {
        s.wayland_display = Some("localyapper-no-wayland".into());
    }
    Enigo::new(&s).map_err(|e| format!("keyboard unavailable: {e}"))
}

mod x11 {
    use std::time::Duration;
    use x11_dl::xlib::{Display, Xlib};

    /// Whether any of Ctrl, Alt, Shift or Super is down. `None` when X11
    /// is not reachable.
    pub fn modifiers_down() -> Option<bool> {
        let xlib = Xlib::open().ok()?;
        // SAFETY: plain Xlib calls on a display we open and close here;
        // the keymap buffer is the 32 bytes XQueryKeymap writes.
        unsafe {
            let dpy: *mut Display = (xlib.XOpenDisplay)(std::ptr::null());
            if dpy.is_null() {
                return None;
            }
            let mut keys = [0 as std::os::raw::c_char; 32];
            (xlib.XQueryKeymap)(dpy, keys.as_mut_ptr());
            let syms = [
                x11_dl::keysym::XK_Control_L,
                x11_dl::keysym::XK_Control_R,
                x11_dl::keysym::XK_Alt_L,
                x11_dl::keysym::XK_Alt_R,
                x11_dl::keysym::XK_Shift_L,
                x11_dl::keysym::XK_Shift_R,
                x11_dl::keysym::XK_Super_L,
                x11_dl::keysym::XK_Super_R,
                x11_dl::keysym::XK_Meta_L,
                x11_dl::keysym::XK_ISO_Level3_Shift,
            ];
            let down = syms.iter().any(|&sym| {
                let code = (xlib.XKeysymToKeycode)(dpy, u64::from(sym)) as usize;
                code != 0 && (keys[code / 8] as u8) & (1 << (code % 8)) != 0
            });
            (xlib.XCloseDisplay)(dpy);
            Some(down)
        }
    }

    pub fn poll_interval() -> Duration {
        Duration::from_millis(10)
    }
}

pub fn wait_modifiers_up(timeout: Duration) {
    if is_wayland() {
        // No cheap global key-state query on Wayland.
        std::thread::sleep(Duration::from_millis(60));
        return;
    }
    let start = Instant::now();
    loop {
        match x11::modifiers_down() {
            Some(true) if start.elapsed() < timeout => std::thread::sleep(x11::poll_interval()),
            Some(_) => return,
            None => {
                std::thread::sleep(Duration::from_millis(60));
                return;
            }
        }
    }
}

fn enigo_chord(chord: Chord) -> Result<(), String> {
    let mut enigo = enigo()?;
    let mut held = Held::new(&mut enigo);
    match chord {
        Chord::PrimaryV => held
            .press(Key::Control)
            .and_then(|_| held.click(Key::Unicode('v'))),
        Chord::CtrlShiftV => held
            .press(Key::Control)
            .and_then(|_| held.press(Key::Shift))
            .and_then(|_| held.click(Key::Unicode('v'))),
        Chord::ShiftInsert => held.press(Key::Shift).and_then(|_| held.click(Key::Insert)),
    }
}

fn run(program: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .status()
        .map_err(|e| format!("{program}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} exited with {status}"))
    }
}

fn wtype(chord: Chord) -> Result<(), String> {
    match chord {
        Chord::PrimaryV => run("wtype", &["-M", "ctrl", "v", "-m", "ctrl"]),
        Chord::CtrlShiftV => run(
            "wtype",
            &[
                "-M", "ctrl", "-M", "shift", "v", "-m", "shift", "-m", "ctrl",
            ],
        ),
        Chord::ShiftInsert => run("wtype", &["-M", "shift", "-k", "Insert", "-m", "shift"]),
    }
}

fn ydotool(chord: Chord) -> Result<(), String> {
    // Linux input event codes: ctrl 29, shift 42, v 47, insert 110.
    match chord {
        Chord::PrimaryV => run("ydotool", &["key", "29:1", "47:1", "47:0", "29:0"]),
        Chord::CtrlShiftV => run(
            "ydotool",
            &["key", "29:1", "42:1", "47:1", "47:0", "42:0", "29:0"],
        ),
        Chord::ShiftInsert => run("ydotool", &["key", "42:1", "110:1", "110:0", "42:0"]),
    }
}

pub fn send_chord(chord: Chord) -> Result<(), PasteError> {
    if !is_wayland() {
        return enigo_chord(chord).map_err(PasteError::Failed);
    }
    let mut errors = Vec::new();
    for attempt in [
        enigo_chord as fn(Chord) -> Result<(), String>,
        wtype,
        ydotool,
    ] {
        match attempt(chord) {
            Ok(()) => return Ok(()),
            Err(e) => errors.push(e),
        }
    }
    log::warn!("inject: no Wayland key path worked: {}", errors.join("; "));
    Err(PasteError::Unavailable(
        "Could not send the paste keys on this Wayland desktop, so the text was copied instead. Install wtype or ydotool to paste automatically.".into(),
    ))
}

pub fn type_text(text: &str) -> Result<(), String> {
    let mut enigo = enigo()?;
    enigo.text(text).map_err(|e| format!("typing failed: {e}"))
}
