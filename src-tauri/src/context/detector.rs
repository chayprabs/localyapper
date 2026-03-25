// Context detection -- identifies the currently focused application
/// Returns the clean application name of the currently focused window.
///
/// Examples: "Notepad", "Chrome", "VS Code", "Firefox".
/// Returns `"Unknown"` if detection fails for any reason. Never panics.
pub fn get_focused_window_name() -> String {
    let raw = get_raw_app_name();
    friendly_name(&raw)
}

/// Map executable stems to human-friendly display names.
fn friendly_name(raw: &str) -> String {
    match raw.to_lowercase().as_str() {
        "notepad" => "Notepad".into(),
        "chrome" => "Chrome".into(),
        "code" => "VS Code".into(),
        "firefox" => "Firefox".into(),
        "msedge" => "Edge".into(),
        "winword" => "Word".into(),
        "excel" => "Excel".into(),
        "powerpnt" => "PowerPoint".into(),
        "onenote" => "OneNote".into(),
        "outlook" => "Outlook".into(),
        "slack" => "Slack".into(),
        "discord" => "Discord".into(),
        "telegram" => "Telegram".into(),
        "spotify" => "Spotify".into(),
        "windowsterminal" | "wt" => "Terminal".into(),
        "cmd" => "Command Prompt".into(),
        "powershell" | "pwsh" => "PowerShell".into(),
        "explorer" => "Explorer".into(),
        "teams" | "ms-teams" => "Teams".into(),
        "cursor" => "Cursor".into(),
        "notion" => "Notion".into(),
        "obsidian" => "Obsidian".into(),
        "figma" => "Figma".into(),
        "unknown" => "Unknown".into(),
        _ => title_case(raw),
    }
}

/// Title-case a string: first letter uppercase, rest as-is.
fn title_case(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => {
            let mut result = c.to_uppercase().to_string();
            result.extend(chars);
            result
        }
        None => "Unknown".into(),
    }
}

/// Platform-specific: get the raw app/process name.
fn get_raw_app_name() -> String {
    #[cfg(target_os = "windows")]
    {
        get_process_name_windows()
    }
    #[cfg(target_os = "macos")]
    {
        get_app_name_macos()
    }
    #[cfg(target_os = "linux")]
    {
        get_process_name_linux()
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        "Unknown".to_string()
    }
}

#[cfg(target_os = "windows")]
fn get_process_name_windows() -> String {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

    // SAFETY: GetForegroundWindow returns a window handle or null — no memory unsafety.
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.0.is_null() {
        return "Unknown".to_string();
    }

    let mut pid: u32 = 0;
    // SAFETY: GetWindowThreadProcessId writes to our stack variable.
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    if pid == 0 {
        return "Unknown".to_string();
    }

    // SAFETY: OpenProcess with limited query rights is safe — we only read process image name.
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) };
    let handle = match handle {
        Ok(h) => h,
        Err(_) => return "Unknown".to_string(),
    };

    let mut buf = [0u16; 512];
    let mut len = buf.len() as u32;
    // SAFETY: QueryFullProcessImageNameW writes into our stack buffer with bounded length.
    let ok = unsafe {
        QueryFullProcessImageNameW(handle, PROCESS_NAME_FORMAT(0), windows::core::PWSTR(buf.as_mut_ptr()), &mut len)
    };
    // SAFETY: CloseHandle on a valid process handle.
    unsafe { let _ = CloseHandle(handle); }

    if ok.is_err() || len == 0 {
        return "Unknown".to_string();
    }

    let path = String::from_utf16_lossy(&buf[..len as usize]);
    // Extract filename stem: "C:\...\notepad.exe" → "notepad"
    std::path::Path::new(&path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown")
        .to_string()
}

#[cfg(target_os = "macos")]
fn get_app_name_macos() -> String {
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
fn get_process_name_linux() -> String {
    // Check if running under Wayland
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        return "Unknown".to_string();
    }

    // X11: get PID of active window, then read process name from /proc
    let output = std::process::Command::new("xdotool")
        .args(["getactivewindow", "getwindowpid"])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let pid_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if pid_str.is_empty() {
                return "Unknown".to_string();
            }
            // Read /proc/<pid>/comm for the process name
            let comm_path = format!("/proc/{}/comm", pid_str);
            match std::fs::read_to_string(&comm_path) {
                Ok(name) => {
                    let name = name.trim().to_string();
                    if name.is_empty() { "Unknown".to_string() } else { name }
                }
                Err(_) => "Unknown".to_string(),
            }
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

    #[test]
    fn friendly_name_mapping() {
        assert_eq!(friendly_name("notepad"), "Notepad");
        assert_eq!(friendly_name("chrome"), "Chrome");
        assert_eq!(friendly_name("Code"), "VS Code");
        assert_eq!(friendly_name("msedge"), "Edge");
        assert_eq!(friendly_name("WINWORD"), "Word");
        assert_eq!(friendly_name("wt"), "Terminal");
        assert_eq!(friendly_name("WindowsTerminal"), "Terminal");
    }

    #[test]
    fn friendly_name_unknown_gets_title_cased() {
        assert_eq!(friendly_name("myapp"), "Myapp");
        assert_eq!(friendly_name("Unknown"), "Unknown");
    }

    #[test]
    fn title_case_works() {
        assert_eq!(title_case("hello"), "Hello");
        assert_eq!(title_case(""), "Unknown");
        assert_eq!(title_case("A"), "A");
    }
}
