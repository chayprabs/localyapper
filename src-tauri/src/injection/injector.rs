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
    use crate::audio::capture::{AudioRecorder, SAMPLE_RATE};
    use crate::audio::vad::{apply_vad, compute_rms, SileroVad};
    use crate::stt::whisper::{
        stt_model_dir_name, WhisperEngine, DEFAULT_STT_MODEL, SILERO_VAD_FILENAME,
    };
    use arboard::Clipboard;
    use cpal::traits::{DeviceTrait, HostTrait};
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};
    use std::fs;
    use std::path::{Path, PathBuf};
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

    struct TextBoxGuard {
        child: Child,
        output_path: PathBuf,
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

    impl Drop for TextBoxGuard {
        fn drop(&mut self) {
            let _ = self.child.kill();
            let _ = self.child.wait();
            let _ = fs::remove_file(&self.output_path);
        }
    }

    const MIN_CAPTURE_RATIO_NUMERATOR: u64 = 4;
    const MIN_CAPTURE_RATIO_DENOMINATOR: u64 = 5;

    fn env_u64(key: &str, default: u64) -> Result<u64, String> {
        match std::env::var(key) {
            Ok(value) => value
                .parse::<u64>()
                .map_err(|_| format!("{key} must be a positive integer, got {value:?}")),
            Err(_) => Ok(default),
        }
    }

    fn env_i32_range(key: &str, default: i32, min: i32, max: i32) -> Result<i32, String> {
        match std::env::var(key) {
            Ok(value) => {
                let parsed = value
                    .parse::<i32>()
                    .map_err(|_| format!("{key} must be an integer, got {value:?}"))?;
                if parsed < min || parsed > max {
                    return Err(format!(
                        "{key} must be between {min} and {max}, got {parsed}"
                    ));
                }
                Ok(parsed)
            }
            Err(_) => Ok(default),
        }
    }

    fn env_bool(key: &str) -> bool {
        std::env::var(key)
            .map(|value| {
                let value = value.trim();
                value.eq_ignore_ascii_case("1")
                    || value.eq_ignore_ascii_case("true")
                    || value.eq_ignore_ascii_case("yes")
            })
            .unwrap_or(false)
    }

    fn normalized_words(text: &str) -> Vec<String> {
        let mut words = Vec::new();
        let mut current = String::new();

        for ch in text.chars() {
            if ch.is_ascii_alphanumeric() {
                current.push(ch.to_ascii_lowercase());
            } else if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
        }

        if !current.is_empty() {
            words.push(current);
        }

        words
    }

    fn expected_transcript_words() -> Vec<String> {
        std::env::var("LOCALYAPPER_MIC_SMOKE_EXPECTED_WORDS")
            .map(|value| normalized_words(&value))
            .unwrap_or_default()
    }

    fn missing_expected_words(expected_words: &[String], transcript: &str) -> Vec<String> {
        let transcript_words = normalized_words(transcript);
        expected_words
            .iter()
            .filter(|word| {
                !transcript_words
                    .iter()
                    .any(|transcript_word| transcript_word == *word)
            })
            .cloned()
            .collect()
    }

    fn input_device_name(device: &cpal::Device) -> String {
        device
            .name()
            .unwrap_or_else(|e| format!("unknown device name ({e})"))
    }

    fn input_device_config_summary(device: &cpal::Device) -> String {
        match device.default_input_config() {
            Ok(config) => format!(
                "{} Hz, {} channel(s), {:?}",
                config.sample_rate().0,
                config.channels(),
                config.sample_format()
            ),
            Err(e) => format!("config unavailable: {e}"),
        }
    }

    fn print_available_input_devices(host: &cpal::Host) -> Result<(), String> {
        println!("Available input devices:");
        let mut found = false;
        let devices = host
            .input_devices()
            .map_err(|e| format!("Failed to enumerate input devices: {e}"))?;

        for (index, device) in devices.enumerate() {
            found = true;
            println!(
                "  {index}: {} ({})",
                input_device_name(&device),
                input_device_config_summary(&device)
            );
        }

        if !found {
            println!("  none");
        }

        Ok(())
    }

    fn selected_input_device(host: &cpal::Host) -> Result<cpal::Device, String> {
        match std::env::var("LOCALYAPPER_MIC_SMOKE_INPUT_DEVICE") {
            Ok(requested) if !requested.trim().is_empty() => {
                let requested = requested.trim().to_ascii_lowercase();
                let devices = host
                    .input_devices()
                    .map_err(|e| format!("Failed to enumerate input devices: {e}"))?;

                for device in devices {
                    let name = input_device_name(&device);
                    if name.to_ascii_lowercase().contains(&requested) {
                        return Ok(device);
                    }
                }

                Err(format!(
                    "No input device matched LOCALYAPPER_MIC_SMOKE_INPUT_DEVICE={requested:?}"
                ))
            }
            _ => host.default_input_device().ok_or_else(|| {
                "No microphone found. Please connect a microphone and try again.".to_string()
            }),
        }
    }

    fn play_optional_speech_prompt() -> Result<bool, String> {
        let text = match std::env::var("LOCALYAPPER_MIC_SMOKE_WINDOWS_TTS_TEXT") {
            Ok(value) if !value.trim().is_empty() => value,
            _ => return Ok(false),
        };
        let rate = env_i32_range("LOCALYAPPER_MIC_SMOKE_WINDOWS_TTS_RATE", 0, -10, 10)?;
        let volume = env_i32_range("LOCALYAPPER_MIC_SMOKE_WINDOWS_TTS_VOLUME", 100, 0, 100)?;
        let voice = std::env::var("LOCALYAPPER_MIC_SMOKE_WINDOWS_TTS_VOICE")
            .ok()
            .filter(|value| !value.trim().is_empty());

        let script = "Add-Type -AssemblyName System.Speech; \
            $speaker = New-Object System.Speech.Synthesis.SpeechSynthesizer; \
            if ($env:LOCALYAPPER_MIC_SMOKE_WINDOWS_TTS_VOICE) { \
                $speaker.SelectVoice($env:LOCALYAPPER_MIC_SMOKE_WINDOWS_TTS_VOICE); \
            } \
            $speaker.Rate = [int]$env:LOCALYAPPER_MIC_SMOKE_WINDOWS_TTS_RATE; \
            $speaker.Volume = [int]$env:LOCALYAPPER_MIC_SMOKE_WINDOWS_TTS_VOLUME; \
            $speaker.Speak($env:LOCALYAPPER_MIC_SMOKE_WINDOWS_TTS_TEXT); \
            $speaker.Dispose()";
        let status = Command::new("powershell")
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                script,
            ])
            .env("LOCALYAPPER_MIC_SMOKE_WINDOWS_TTS_TEXT", &text)
            .env("LOCALYAPPER_MIC_SMOKE_WINDOWS_TTS_RATE", rate.to_string())
            .env(
                "LOCALYAPPER_MIC_SMOKE_WINDOWS_TTS_VOLUME",
                volume.to_string(),
            )
            .env(
                "LOCALYAPPER_MIC_SMOKE_WINDOWS_TTS_VOICE",
                voice.as_deref().unwrap_or(""),
            )
            .status()
            .map_err(|e| format!("Failed to run Windows speech prompt: {e}"))?;

        if status.success() {
            Ok(true)
        } else {
            Err(format!("Windows speech prompt exited with status {status}"))
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

    fn app_activate_window_by_title(title_fragment: &str) -> Result<(), String> {
        let script = "$shell = New-Object -ComObject WScript.Shell; \
            if ($shell.AppActivate($env:LOCALYAPPER_WINDOW_TITLE_FRAGMENT)) { exit 0 } \
            exit 1";
        let status = Command::new("powershell")
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                script,
            ])
            .env("LOCALYAPPER_WINDOW_TITLE_FRAGMENT", title_fragment)
            .status()
            .map_err(|e| format!("Failed to run AppActivate fallback: {e}"))?;

        if status.success() {
            thread::sleep(Duration::from_millis(250));
            Ok(())
        } else {
            Err(format!(
                "AppActivate did not focus title fragment {title_fragment:?}"
            ))
        }
    }

    fn app_activate_when_available(title_fragment: &str) -> Result<(), String> {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if find_window_by_title(title_fragment).is_ok()
                && app_activate_window_by_title(title_fragment).is_ok()
            {
                return Ok(());
            }

            if Instant::now() >= deadline {
                return Err(format!(
                    "Failed to activate window with title fragment {title_fragment:?}"
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

    fn open_empty_notepad_file(prefix: &str) -> Result<NotepadGuard, String> {
        let path = std::env::temp_dir().join(format!("{}-{}.txt", prefix, std::process::id()));
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

        Ok(guard)
    }

    fn open_external_textbox_target(prefix: &str) -> Result<TextBoxGuard, String> {
        let marker = chrono::Utc::now().timestamp_millis();
        let output_path = std::env::temp_dir().join(format!("{prefix}-output-{marker}.txt"));
        let title_fragment = format!("{prefix}-{marker}");
        fs::write(&output_path, "").map_err(|e| format!("Temp file setup failed: {e}"))?;

        let script = "Add-Type -AssemblyName System.Windows.Forms; \
            Add-Type -AssemblyName System.Drawing; \
            [System.Windows.Forms.Application]::EnableVisualStyles(); \
            $form = New-Object System.Windows.Forms.Form; \
            $form.Text = $env:LOCALYAPPER_TEXTBOX_TITLE; \
            $form.Width = 900; \
            $form.Height = 500; \
            $textBox = New-Object System.Windows.Forms.TextBox; \
            $textBox.Multiline = $true; \
            $textBox.AcceptsReturn = $true; \
            $textBox.AcceptsTab = $true; \
            $textBox.Dock = [System.Windows.Forms.DockStyle]::Fill; \
            $textBox.Font = New-Object System.Drawing.Font('Consolas', 12); \
            $writeText = { Set-Content -LiteralPath $env:LOCALYAPPER_TEXTBOX_OUTPUT_PATH -Value $textBox.Text -Encoding UTF8 }; \
            $textBox.Add_TextChanged($writeText); \
            $form.Controls.Add($textBox); \
            $form.Add_Shown({ $form.Activate(); $textBox.Focus() }); \
            [System.Windows.Forms.Application]::Run($form)";

        let child = Command::new("powershell")
            .args([
                "-NoProfile",
                "-STA",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                script,
            ])
            .env("LOCALYAPPER_TEXTBOX_TITLE", &title_fragment)
            .env("LOCALYAPPER_TEXTBOX_OUTPUT_PATH", &output_path)
            .spawn()
            .map_err(|e| format!("Failed to launch external textbox target: {e}"))?;

        let guard = TextBoxGuard {
            child,
            output_path,
            title_fragment,
        };
        app_activate_when_available(&guard.title_fragment)?;

        Ok(guard)
    }

    fn default_app_data_dir() -> Result<PathBuf, String> {
        if let Ok(path) = std::env::var("LOCALYAPPER_APP_DATA_DIR") {
            return Ok(PathBuf::from(path));
        }

        let app_data = std::env::var("APPDATA")
            .map_err(|_| "APPDATA is not set; set LOCALYAPPER_APP_DATA_DIR".to_string())?;
        Ok(PathBuf::from(app_data).join("com.localyapper.desktop"))
    }

    fn read_u16_le(bytes: &[u8], offset: usize) -> Result<u16, String> {
        let slice = bytes
            .get(offset..offset + 2)
            .ok_or_else(|| format!("WAV ended before u16 at offset {offset}"))?;
        Ok(u16::from_le_bytes([slice[0], slice[1]]))
    }

    fn read_u32_le(bytes: &[u8], offset: usize) -> Result<u32, String> {
        let slice = bytes
            .get(offset..offset + 4)
            .ok_or_else(|| format!("WAV ended before u32 at offset {offset}"))?;
        Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
    }

    fn read_pcm16_mono_16khz_wav(path: &Path) -> Result<Vec<f32>, String> {
        let bytes = fs::read(path).map_err(|e| format!("Failed to read WAV: {e}"))?;
        if bytes.get(0..4) != Some(b"RIFF") || bytes.get(8..12) != Some(b"WAVE") {
            return Err("WAV is not RIFF/WAVE".to_string());
        }

        let mut audio_format = None;
        let mut channels = None;
        let mut sample_rate = None;
        let mut bits_per_sample = None;
        let mut data_range = None;
        let mut offset = 12usize;

        while offset + 8 <= bytes.len() {
            let chunk_id = bytes
                .get(offset..offset + 4)
                .ok_or_else(|| "Missing WAV chunk id".to_string())?;
            let chunk_size = read_u32_le(&bytes, offset + 4)? as usize;
            let chunk_start = offset + 8;
            let chunk_end = chunk_start
                .checked_add(chunk_size)
                .ok_or_else(|| "WAV chunk size overflow".to_string())?;
            if chunk_end > bytes.len() {
                return Err("WAV chunk extends past end of file".to_string());
            }

            match chunk_id {
                b"fmt " => {
                    audio_format = Some(read_u16_le(&bytes, chunk_start)?);
                    channels = Some(read_u16_le(&bytes, chunk_start + 2)?);
                    sample_rate = Some(read_u32_le(&bytes, chunk_start + 4)?);
                    bits_per_sample = Some(read_u16_le(&bytes, chunk_start + 14)?);
                }
                b"data" => data_range = Some(chunk_start..chunk_end),
                _ => {}
            }

            offset = chunk_end + (chunk_size % 2);
        }

        if audio_format != Some(1) {
            return Err(format!("Expected PCM WAV format 1, got {audio_format:?}"));
        }
        if channels != Some(1) {
            return Err(format!("Expected mono WAV, got {channels:?} channels"));
        }
        if sample_rate != Some(16_000) {
            return Err(format!("Expected 16 kHz WAV, got {sample_rate:?}"));
        }
        if bits_per_sample != Some(16) {
            return Err(format!(
                "Expected 16-bit WAV samples, got {bits_per_sample:?}"
            ));
        }

        let data_range = data_range.ok_or_else(|| "WAV has no data chunk".to_string())?;
        let data = &bytes[data_range];
        if data.len() % 2 != 0 {
            return Err("PCM16 data chunk has an odd byte length".to_string());
        }

        Ok(data
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]) as f32 / 32768.0)
            .collect())
    }

    fn generate_windows_tts_wav(path: &Path, text: &str) -> Result<(), String> {
        let script = "Add-Type -AssemblyName System.Speech; \
            $format = New-Object System.Speech.AudioFormat.SpeechAudioFormatInfo(16000, [System.Speech.AudioFormat.AudioBitsPerSample]::Sixteen, [System.Speech.AudioFormat.AudioChannel]::Mono); \
            $speaker = New-Object System.Speech.Synthesis.SpeechSynthesizer; \
            $speaker.SetOutputToWaveFile($env:LOCALYAPPER_TTS_WAV_PATH, $format); \
            $speaker.Speak($env:LOCALYAPPER_TTS_TEXT); \
            $speaker.Dispose()";
        let status = Command::new("powershell")
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                script,
            ])
            .env("LOCALYAPPER_TTS_WAV_PATH", path)
            .env("LOCALYAPPER_TTS_TEXT", text)
            .status()
            .map_err(|e| format!("Failed to generate Windows TTS WAV: {e}"))?;

        if status.success() {
            Ok(())
        } else {
            Err(format!("Windows TTS WAV generation exited with {status}"))
        }
    }

    fn transcribe_generated_speech(text: &str) -> Result<String, String> {
        let wav_path = std::env::temp_dir().join(format!(
            "localyapper-tts-to-notepad-smoke-{}.wav",
            std::process::id()
        ));
        generate_windows_tts_wav(&wav_path, text)?;

        let audio = match read_pcm16_mono_16khz_wav(&wav_path) {
            Ok(audio) => audio,
            Err(e) => {
                let _ = fs::remove_file(&wav_path);
                return Err(e);
            }
        };
        let _ = fs::remove_file(&wav_path);

        let rms = compute_rms(&audio);
        let peak = audio
            .iter()
            .fold(0.0_f32, |max, sample| max.max(sample.abs()));
        println!(
            "Generated TTS WAV: {} samples, RMS {rms:.6}, peak {peak:.6}",
            audio.len()
        );

        let vad_result = apply_vad(&audio, None);
        if !vad_result.has_speech {
            return Err(format!(
                "Generated TTS audio did not pass VAD. RMS {rms:.6}, peak {peak:.6}"
            ));
        }

        let speech_model_dir = default_app_data_dir()?
            .join("models")
            .join(stt_model_dir_name(DEFAULT_STT_MODEL));
        let engine = WhisperEngine::new(&speech_model_dir).map_err(|e| e.to_string())?;
        engine
            .transcribe(&vad_result.trimmed_audio)
            .map_err(|e| e.to_string())
    }

    fn transcribe_microphone_input() -> Result<String, String> {
        let countdown_secs = env_u64("LOCALYAPPER_MIC_SMOKE_COUNTDOWN_SECS", 3)?;
        let record_secs = env_u64("LOCALYAPPER_MIC_SMOKE_RECORD_SECS", 5)?;
        let wait_for_enter = env_bool("LOCALYAPPER_MIC_SMOKE_WAIT_FOR_ENTER");
        let expected_words = expected_transcript_words();
        if record_secs == 0 {
            return Err("LOCALYAPPER_MIC_SMOKE_RECORD_SECS must be greater than 0".to_string());
        }

        let app_data_dir = default_app_data_dir()?;
        let models_dir = app_data_dir.join("models");
        let speech_model_dir = models_dir.join(stt_model_dir_name(DEFAULT_STT_MODEL));
        let vad_path = models_dir.join(SILERO_VAD_FILENAME);

        let host = cpal::default_host();
        print_available_input_devices(&host)?;
        let input_device = selected_input_device(&host)?;
        println!(
            "Selected input device: {} ({})",
            input_device_name(&input_device),
            input_device_config_summary(&input_device)
        );
        println!("Using speech model: {}", speech_model_dir.display());
        println!("Recording for {record_secs}s after a {countdown_secs}s countdown.");
        println!("Speak the expected phrase while recording is active.");
        if !expected_words.is_empty() {
            println!("Expected transcript word(s): {}", expected_words.join(", "));
        }

        if wait_for_enter {
            println!("Press Enter when ready to start the countdown.");
            let mut line = String::new();
            std::io::stdin()
                .read_line(&mut line)
                .map_err(|e| format!("Failed to read Enter confirmation: {e}"))?;
        }

        for remaining in (1..=countdown_secs).rev() {
            println!("Recording starts in {remaining}...");
            thread::sleep(Duration::from_secs(1));
        }

        let recorder = AudioRecorder::new();
        recorder
            .start_for_device(input_device)
            .map_err(|e| e.to_string())?;
        println!("Recording now.");
        if play_optional_speech_prompt()? {
            println!("Windows speech prompt completed.");
        }
        thread::sleep(Duration::from_secs(record_secs));
        let audio = recorder.stop().map_err(|e| e.to_string())?;

        println!("Captured {} samples at 16 kHz.", audio.len());
        let rms = compute_rms(&audio);
        let peak = audio
            .iter()
            .fold(0.0_f32, |max, sample| max.max(sample.abs()));
        println!("Captured audio RMS: {rms:.6}, peak: {peak:.6}");

        let min_expected_samples = (u64::from(SAMPLE_RATE)
            .saturating_mul(record_secs)
            .saturating_mul(MIN_CAPTURE_RATIO_NUMERATOR)
            / MIN_CAPTURE_RATIO_DENOMINATOR) as usize;
        if audio.len() < min_expected_samples {
            return Err(format!(
                "Captured too little audio: {} samples at 16 kHz; expected at least {min_expected_samples}",
                audio.len(),
            ));
        }

        let silero = if vad_path.exists() {
            Some(SileroVad::new(&vad_path).map_err(|e| e.to_string())?)
        } else {
            None
        };
        let vad_result = apply_vad(&audio, silero.as_ref());
        if !vad_result.has_speech {
            return Err(format!(
                "No speech detected; rerun and speak during the recording window. Captured RMS: {rms:.6}, peak: {peak:.6}"
            ));
        }

        let engine = WhisperEngine::new(&speech_model_dir).map_err(|e| e.to_string())?;
        let transcript = engine
            .transcribe(&vad_result.trimmed_audio)
            .map_err(|e| e.to_string())?;

        println!("Transcript: {transcript}");
        if transcript.trim().is_empty() {
            return Err("STT returned an empty transcript for microphone audio".to_string());
        }
        if !expected_words.is_empty() {
            let missing_words = missing_expected_words(&expected_words, &transcript);
            if !missing_words.is_empty() {
                return Err(format!(
                    "Transcript did not contain expected word(s): {}. Transcript was: {transcript:?}",
                    missing_words.join(", ")
                ));
            }
        }

        Ok(transcript)
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
        let guard = open_empty_notepad_file("localyapper-injection-smoke")?;

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

    #[test]
    #[ignore = "requires an interactive Windows desktop and opens an external textbox target"]
    fn manual_windows_textbox_injection_smoke() -> Result<(), String> {
        let marker = format!(
            "LocalYapper textbox injection smoke {}",
            chrono::Utc::now().timestamp_millis()
        );
        let injected_text = format!("{marker} pasted through injector");
        let original_clipboard = format!("{marker} original clipboard");
        let guard = open_external_textbox_target("localyapper-textbox-injection-smoke")?;

        let mut clipboard = Clipboard::new().map_err(|e| format!("Clipboard init failed: {e}"))?;
        clipboard
            .set_text(original_clipboard.clone())
            .map_err(|e| format!("Clipboard setup failed: {e}"))?;

        inject(&injected_text, false)?;
        let saved_text = wait_for_file_text(&guard.output_path, &injected_text)?;

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

    #[test]
    #[ignore = "requires Windows SAPI, installed speech model files, and opens Notepad"]
    fn manual_windows_tts_to_notepad_pipeline_smoke() -> Result<(), String> {
        let phrase = std::env::var("LOCALYAPPER_TTS_TO_NOTEPAD_TEXT").unwrap_or_else(|_| {
            "LocalYapper generated speech to notepad pipeline smoke test.".to_string()
        });
        let transcript = transcribe_generated_speech(&phrase)?;
        println!("Transcript: {transcript}");
        if transcript.trim().is_empty() {
            return Err("STT returned an empty transcript for generated speech".to_string());
        }

        let marker = format!(
            "LocalYapper TTS to Notepad smoke {}",
            chrono::Utc::now().timestamp_millis()
        );
        let injected_text = format!("{marker}: {transcript}");
        let original_clipboard = format!("{marker} original clipboard");
        let guard = open_empty_notepad_file("localyapper-tts-to-notepad-smoke")?;

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

    #[test]
    #[ignore = "requires an interactive Windows desktop, microphone, spoken audio, installed speech model files, and opens an external textbox target"]
    fn manual_windows_microphone_to_textbox_pipeline_smoke() -> Result<(), String> {
        let marker = format!(
            "LocalYapper microphone to textbox smoke {}",
            chrono::Utc::now().timestamp_millis()
        );
        let original_clipboard = format!("{marker} original clipboard");
        let guard = open_external_textbox_target("localyapper-mic-to-textbox-smoke")?;

        let mut clipboard = Clipboard::new().map_err(|e| format!("Clipboard init failed: {e}"))?;
        clipboard
            .set_text(original_clipboard.clone())
            .map_err(|e| format!("Clipboard setup failed: {e}"))?;

        let transcript = transcribe_microphone_input()?;
        let injected_text = format!("{marker}: {transcript}");

        app_activate_when_available(&guard.title_fragment)?;
        inject(&injected_text, false)?;
        let saved_text = wait_for_file_text(&guard.output_path, &injected_text)?;

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

    #[test]
    #[ignore = "requires an interactive Windows desktop, microphone, spoken audio, installed speech model files, and opens Notepad"]
    fn manual_windows_microphone_to_notepad_pipeline_smoke() -> Result<(), String> {
        let marker = format!(
            "LocalYapper microphone to Notepad smoke {}",
            chrono::Utc::now().timestamp_millis()
        );
        let original_clipboard = format!("{marker} original clipboard");
        let guard = open_empty_notepad_file("localyapper-mic-to-notepad-smoke")?;

        let mut clipboard = Clipboard::new().map_err(|e| format!("Clipboard init failed: {e}"))?;
        clipboard
            .set_text(original_clipboard.clone())
            .map_err(|e| format!("Clipboard setup failed: {e}"))?;

        let transcript = transcribe_microphone_input()?;
        let injected_text = format!("{marker}: {transcript}");

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
