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
    use crate::audio::vad::{apply_vad, compute_rms};
    use crate::stt::whisper::{stt_model_dir_name, WhisperEngine, DEFAULT_STT_MODEL};
    use arboard::Clipboard;
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
}
