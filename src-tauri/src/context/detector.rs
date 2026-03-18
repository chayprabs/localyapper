/// Returns the name of the currently focused application/window.
///
/// Returns `"Unknown"` if detection fails for any reason. Never panics.
pub fn get_focused_window_name() -> String {
    #[cfg(target_os = "windows")]
    {
        get_focused_window_windows()
    }
    #[cfg(target_os = "macos")]
    {
        get_focused_window_macos()
    }
    #[cfg(target_os = "linux")]
    {
        get_focused_window_linux()
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        "Unknown".to_string()
    }
}

#[cfg(target_os = "windows")]
fn get_focused_window_windows() -> String {
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW};

    // SAFETY: GetForegroundWindow returns a window handle or null — no memory unsafety.
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.0.is_null() {
        return "Unknown".to_string();
    }

    let mut buf = [0u16; 512];
    // SAFETY: GetWindowTextW writes into our stack buffer with bounded length.
    let len = unsafe { GetWindowTextW(hwnd, &mut buf) };
    if len <= 0 {
        return "Unknown".to_string();
    }

    String::from_utf16_lossy(&buf[..len as usize])
}

#[cfg(target_os = "macos")]
fn get_focused_window_macos() -> String {
    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to get name of first application process whose frontmost is true")
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if name.is_empty() { "Unknown".to_string() } else { name }
        }
        _ => "Unknown".to_string(),
    }
}

#[cfg(target_os = "linux")]
fn get_focused_window_linux() -> String {
    // Check if running under Wayland
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        // No reliable way to get focused window name on Wayland without special protocols
        return "Unknown".to_string();
    }

    // X11: use xdotool
    let output = std::process::Command::new("xdotool")
        .args(["getactivewindow", "getwindowname"])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if name.is_empty() { "Unknown".to_string() } else { name }
        }
        _ => "Unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_focused_window_returns_string() {
        let name = get_focused_window_name();
        assert!(!name.is_empty());
    }
}
