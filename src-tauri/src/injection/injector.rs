use crate::injection::platform::{self, Platform};
use std::time::Duration;

const PASTE_DELAY_MS: u64 = 80;

/// Inject text into the focused application via clipboard.
///
/// Flow: save clipboard → set text → simulate paste → wait → restore clipboard.
/// If `auto_send` is true, simulates Enter after pasting.
pub fn inject(text: &str, auto_send: bool) -> Result<(), String> {
    let platform = platform::detect();

    match platform {
        Platform::LinuxWayland => inject_wayland(text, auto_send),
        Platform::LinuxX11 => inject_x11(text, auto_send),
        _ => inject_native(text, auto_send, platform),
    }
}

/// Native injection using arboard + enigo (Windows, macOS, Linux X11 fallback).
fn inject_native(text: &str, auto_send: bool, platform: Platform) -> Result<(), String> {
    use arboard::Clipboard;
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};

    let mut clipboard = Clipboard::new().map_err(|e| format!("Clipboard init failed: {e}"))?;

    // Save current clipboard contents
    let saved = clipboard.get_text().ok();

    // Set new text
    clipboard
        .set_text(text.to_owned())
        .map_err(|e| format!("Clipboard set failed: {e}"))?;

    // Simulate paste keystroke
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| format!("Enigo init failed: {e}"))?;

    let modifier = match platform {
        Platform::MacOS => Key::Meta,
        _ => Key::Control,
    };

    enigo
        .key(modifier, Direction::Press)
        .map_err(|e| format!("Key press failed: {e}"))?;
    enigo
        .key(Key::Unicode('v'), Direction::Click)
        .map_err(|e| format!("Key click failed: {e}"))?;
    enigo
        .key(modifier, Direction::Release)
        .map_err(|e| format!("Key release failed: {e}"))?;

    // Wait for paste to complete
    std::thread::sleep(Duration::from_millis(PASTE_DELAY_MS));

    // Simulate Enter if auto_send
    if auto_send {
        enigo
            .key(Key::Return, Direction::Click)
            .map_err(|e| format!("Enter key failed: {e}"))?;
        std::thread::sleep(Duration::from_millis(30));
    }

    // Restore previous clipboard contents
    if let Some(prev) = saved {
        let _ = clipboard.set_text(prev);
    }

    Ok(())
}

/// Linux X11 fallback using xclip + xdotool shell commands.
fn inject_x11(text: &str, auto_send: bool) -> Result<(), String> {
    use std::process::Command;

    // Save current clipboard
    let saved = Command::new("xclip")
        .args(["-selection", "clipboard", "-o"])
        .output()
        .ok()
        .and_then(|o| if o.status.success() { String::from_utf8(o.stdout).ok() } else { None });

    // Set new text via xclip
    let mut child = Command::new("xclip")
        .args(["-selection", "clipboard"])
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("xclip spawn failed: {e}"))?;

    if let Some(ref mut stdin) = child.stdin {
        use std::io::Write;
        stdin
            .write_all(text.as_bytes())
            .map_err(|e| format!("xclip write failed: {e}"))?;
    }
    child.wait().map_err(|e| format!("xclip wait failed: {e}"))?;

    // Simulate Ctrl+V via xdotool
    Command::new("xdotool")
        .args(["key", "ctrl+v"])
        .status()
        .map_err(|e| format!("xdotool paste failed: {e}"))?;

    std::thread::sleep(Duration::from_millis(PASTE_DELAY_MS));

    if auto_send {
        Command::new("xdotool")
            .args(["key", "Return"])
            .status()
            .map_err(|e| format!("xdotool enter failed: {e}"))?;
        std::thread::sleep(Duration::from_millis(30));
    }

    // Restore clipboard
    if let Some(prev) = saved {
        let mut child = Command::new("xclip")
            .args(["-selection", "clipboard"])
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("xclip restore spawn failed: {e}"))?;
        if let Some(ref mut stdin) = child.stdin {
            use std::io::Write;
            let _ = stdin.write_all(prev.as_bytes());
        }
        let _ = child.wait();
    }

    Ok(())
}

/// Linux Wayland fallback using wl-copy / wl-paste + wtype.
fn inject_wayland(text: &str, auto_send: bool) -> Result<(), String> {
    use std::process::Command;

    // Save current clipboard
    let saved = Command::new("wl-paste")
        .arg("--no-newline")
        .output()
        .ok()
        .and_then(|o| if o.status.success() { String::from_utf8(o.stdout).ok() } else { None });

    // Set new text
    let mut child = Command::new("wl-copy")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("wl-copy spawn failed: {e}"))?;

    if let Some(ref mut stdin) = child.stdin {
        use std::io::Write;
        stdin
            .write_all(text.as_bytes())
            .map_err(|e| format!("wl-copy write failed: {e}"))?;
    }
    child.wait().map_err(|e| format!("wl-copy wait failed: {e}"))?;

    // Simulate paste via wtype
    Command::new("wtype")
        .args(["-M", "ctrl", "-P", "v", "-m", "ctrl"])
        .status()
        .map_err(|e| format!("wtype paste failed: {e}"))?;

    std::thread::sleep(Duration::from_millis(PASTE_DELAY_MS));

    if auto_send {
        Command::new("wtype")
            .args(["-k", "Return"])
            .status()
            .map_err(|e| format!("wtype enter failed: {e}"))?;
        std::thread::sleep(Duration::from_millis(30));
    }

    // Restore clipboard
    if let Some(prev) = saved {
        let mut child = Command::new("wl-copy")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("wl-copy restore spawn failed: {e}"))?;
        if let Some(ref mut stdin) = child.stdin {
            use std::io::Write;
            let _ = stdin.write_all(prev.as_bytes());
        }
        let _ = child.wait();
    }

    Ok(())
}
