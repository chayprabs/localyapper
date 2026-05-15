// Text injection -- clipboard save, paste simulation, clipboard restore
use crate::injection::platform::{self, Platform};
use std::time::Duration;

/// Milliseconds to wait after simulating paste before restoring clipboard.
/// 80ms gives the target application enough time to read the clipboard contents.
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
    let mut enigo =
        Enigo::new(&Settings::default()).map_err(|e| format!("Enigo init failed: {e}"))?;

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
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok()
            } else {
                None
            }
        });

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
    child
        .wait()
        .map_err(|e| format!("xclip wait failed: {e}"))?;

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
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok()
            } else {
                None
            }
        });

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
    child
        .wait()
        .map_err(|e| format!("wl-copy wait failed: {e}"))?;

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

#[cfg(all(test, target_os = "windows"))]
mod manual_tests {
    use super::inject;
    use arboard::Clipboard;
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};
    use std::fs;
    use std::path::PathBuf;
    use std::process::{Child, Command};
    use std::thread;
    use std::time::{Duration, Instant};
    use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
        SetForegroundWindow, ShowWindow, SW_RESTORE,
    };

    struct NotepadGuard {
        child: Child,
        path: PathBuf,
        title_fragment: String,
    }

    impl Drop for NotepadGuard {
        fn drop(&mut self) {
            if let Ok(hwnd) = find_window_by_title(&self.title_fragment) {
                let mut process_id = 0u32;
                unsafe {
                    GetWindowThreadProcessId(hwnd, Some(&mut process_id));
                }
                if process_id != 0 {
                    let _ = Command::new("taskkill")
                        .args(["/PID", &process_id.to_string(), "/F"])
                        .status();
                }
            }
            let _ = self.child.kill();
            let _ = self.child.wait();
            let _ = fs::remove_file(&self.path);
        }
    }

    struct WindowSearch {
        title_fragment: String,
        hwnd: HWND,
    }

    unsafe extern "system" fn enum_window_titles(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let search = &mut *(lparam.0 as *mut WindowSearch);
        if !IsWindowVisible(hwnd).as_bool() {
            return BOOL(1);
        }

        let mut title = [0u16; 512];
        let len = GetWindowTextW(hwnd, &mut title);
        if len <= 0 {
            return BOOL(1);
        }

        let title = String::from_utf16_lossy(&title[..len as usize]);
        if title.contains(&search.title_fragment) {
            search.hwnd = hwnd;
            BOOL(0)
        } else {
            BOOL(1)
        }
    }

    fn find_window_by_title(title_fragment: &str) -> Result<HWND, String> {
        let mut search = WindowSearch {
            title_fragment: title_fragment.to_string(),
            hwnd: HWND(std::ptr::null_mut()),
        };
        unsafe {
            let _ = EnumWindows(
                Some(enum_window_titles),
                LPARAM(&mut search as *mut WindowSearch as isize),
            );
        }

        if search.hwnd.0.is_null() {
            Err(format!(
                "No visible window contained title fragment {title_fragment:?}"
            ))
        } else {
            Ok(search.hwnd)
        }
    }

    fn focus_window_by_title(title_fragment: &str) -> Result<(), String> {
        let deadline = Instant::now() + Duration::from_secs(8);
        loop {
            match find_window_by_title(title_fragment) {
                Ok(hwnd) => unsafe {
                    let _ = ShowWindow(hwnd, SW_RESTORE);
                    if SetForegroundWindow(hwnd).as_bool() {
                        thread::sleep(Duration::from_millis(250));
                        return Ok(());
                    }
                },
                Err(e) if Instant::now() >= deadline => return Err(e),
                Err(_) => {}
            }

            if Instant::now() >= deadline {
                return Err(format!(
                    "Failed to focus window with title fragment {title_fragment:?}"
                ));
            }
            thread::sleep(Duration::from_millis(250));
        }
    }

    fn save_focused_document() -> Result<(), String> {
        let mut enigo =
            Enigo::new(&Settings::default()).map_err(|e| format!("Enigo init failed: {e}"))?;
        enigo
            .key(Key::Control, Direction::Press)
            .map_err(|e| format!("Ctrl press failed: {e}"))?;
        enigo
            .key(Key::Unicode('s'), Direction::Click)
            .map_err(|e| format!("S click failed: {e}"))?;
        enigo
            .key(Key::Control, Direction::Release)
            .map_err(|e| format!("Ctrl release failed: {e}"))?;
        Ok(())
    }

    fn wait_for_file_text(path: &PathBuf, expected: &str) -> Result<String, String> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let text = fs::read_to_string(path).map_err(|e| format!("Read failed: {e}"))?;
            if text.contains(expected) {
                return Ok(text);
            }
            if Instant::now() >= deadline {
                return Err(format!(
                    "Timed out waiting for injected text in {}. Current contents: {text:?}",
                    path.display()
                ));
            }
            thread::sleep(Duration::from_millis(100));
        }
    }

    #[test]
    #[ignore = "requires an interactive Windows desktop and opens Notepad"]
    fn manual_windows_notepad_injection_smoke() -> Result<(), String> {
        let marker = format!(
            "LocalYapper injection smoke {}",
            chrono::Utc::now().timestamp_millis()
        );
        let injected_text = format!("{marker} pasted through injector");
        let original_clipboard = format!("{marker} original clipboard");
        let path = std::env::temp_dir().join(format!(
            "localyapper-injection-smoke-{}.txt",
            std::process::id()
        ));
        let title_fragment = path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| "Temp file name is not valid UTF-8".to_string())?
            .to_string();
        fs::write(&path, "").map_err(|e| format!("Temp file setup failed: {e}"))?;

        let child = Command::new("notepad.exe")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to launch Notepad: {e}"))?;
        let guard = NotepadGuard {
            child,
            path,
            title_fragment,
        };

        thread::sleep(Duration::from_secs(2));
        focus_window_by_title(&guard.title_fragment)?;

        let mut clipboard = Clipboard::new().map_err(|e| format!("Clipboard init failed: {e}"))?;
        clipboard
            .set_text(original_clipboard.clone())
            .map_err(|e| format!("Clipboard setup failed: {e}"))?;

        inject(&injected_text, false)?;
        thread::sleep(Duration::from_millis(200));
        save_focused_document()?;
        let saved_text = wait_for_file_text(&guard.path, &injected_text)?;

        let restored_clipboard = clipboard
            .get_text()
            .map_err(|e| format!("Clipboard read failed: {e}"))?;
        if restored_clipboard != original_clipboard {
            return Err(format!(
                "Clipboard was not restored. Expected {original_clipboard:?}, got {restored_clipboard:?}"
            ));
        }
        if !saved_text.contains(&injected_text) {
            return Err(format!("Saved text did not contain {injected_text:?}"));
        }

        Ok(())
    }
}
