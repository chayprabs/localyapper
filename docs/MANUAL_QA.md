# LocalYapper Manual QA Checklist

Use this checklist for the v0.1.0 release candidate after automated checks and
GitHub Actions builds are green.

## Scope

Manual QA covers the parts automation cannot prove reliably:

- real microphone capture,
- OS microphone/accessibility permissions,
- global hotkeys,
- overlay behavior on the desktop,
- clipboard save -> paste -> restore injection into another app,
- first-launch behavior on a fresh app data directory.

Do not commit app data, model files, databases, screenshots with private text,
or generated installers during this process.

## Preflight

1. Install or run the platform artifact for the commit under test.
2. Confirm the app version is `0.1.0`.
3. Confirm the speech model exists or can be downloaded from Settings > Speech.
4. Confirm microphone permission is granted.
5. On macOS, confirm accessibility permission is granted if injection fails.
6. On Linux, install runtime injection tools:
   - X11: `xclip` and `xdotool`
   - Wayland: `wl-copy`, `wl-paste`, and `wtype`

## Fresh App Data Pass

Before this pass, back up any existing LocalYapper app data directory instead
of deleting it.

Expected app data locations:

- Windows: `%APPDATA%\com.localyapper.desktop`
- macOS: `~/Library/Application Support/com.localyapper.desktop`
- Linux: `~/.local/share/com.localyapper.desktop`

Steps:

1. Quit LocalYapper from the tray.
2. Rename the current app data directory to a temporary backup name.
3. Launch LocalYapper.
4. Confirm the first-launch wizard opens.
5. Download or confirm the speech model.
6. Confirm the default hotkey.
7. Finish setup.
8. Quit and relaunch LocalYapper.
9. Confirm the wizard does not re-open.
10. Restore the original app data directory after the pass if needed.

## Dictation Injection Pass

Use a plain text target app:

- Windows: Notepad
- macOS: TextEdit in plain-text mode
- Linux: a simple text editor such as gedit, Kate, or Mousepad

Steps:

1. Launch LocalYapper.
2. Open the target text editor.
3. Type a sentinel prefix manually, for example `before `.
4. Place the cursor after the sentinel text.
5. Hold `F8`.
6. Speak a short sentence clearly.
7. Release `F8`.
8. Confirm the overlay shows listening, then processing, then transcribed.
9. Confirm the final text is pasted into the target editor after the sentinel.
10. Confirm the clipboard contents are restored after paste.
11. Confirm the dictation appears in History.
12. Confirm Dashboard stats update after returning to the app.

Pass criteria:

- The app does not crash.
- The overlay does not get stuck.
- The final text is visible either in the target app or, on injection failure,
  in the overlay with copy support.
- Existing clipboard contents are restored.
- History and Dashboard reflect the completed dictation.

## Optional Windows Injection Smoke

On an interactive Windows desktop, this ignored test launches Notepad, focuses
the temporary file window, calls the real clipboard injector, saves the file,
and confirms that the original clipboard text is restored:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml manual_windows_notepad_injection_smoke -- --ignored --nocapture
```

This does not replace the real microphone dictation pass, but it gives a
repeatable check for the external-app clipboard injection layer.

## Optional Microphone Transcription Smoke

On a machine with the speech model installed, this ignored test records from the
default microphone, runs VAD, loads the local speech model, and asserts that STT
returns a non-empty transcript:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml manual_microphone_transcription_smoke -- --ignored --nocapture
```

The default timing is a 3-second countdown followed by 5 seconds of recording.
Speak a short sentence while the test prints `Recording now.`.
The test prints the selected default input device, native capture format,
captured sample count, RMS, and peak level to help diagnose wrong-device or
too-quiet-input failures.

Optional environment variables:

- `LOCALYAPPER_MIC_SMOKE_COUNTDOWN_SECS`: countdown before recording starts.
- `LOCALYAPPER_MIC_SMOKE_RECORD_SECS`: recording duration in seconds.
- `LOCALYAPPER_APP_DATA_DIR`: app data directory containing the `models` folder
  if it is not in the platform default location.
- `LOCALYAPPER_MIC_SMOKE_WINDOWS_TTS_TEXT`: Windows-only optional spoken prompt
  played through the system speaker during recording. Use this only when speaker
  audio can be heard by the default microphone.

## Optional Windows Synthetic Speech STT Smoke

This ignored Windows-only test generates a 16 kHz mono WAV with Windows SAPI,
runs it through VAD and the installed speech model, and asserts that STT returns
a non-empty transcript:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml manual_windows_tts_file_transcription_smoke -- --ignored --nocapture
```

Optional environment variable:

- `LOCALYAPPER_TTS_FILE_SMOKE_TEXT`: phrase to synthesize into the temporary WAV.

This validates VAD plus local STT with generated speech audio, but it does not
replace the real microphone capture pass.

## Hotkey Pass

1. Open Settings > Hotkeys.
2. Change the record hotkey to an available shortcut.
3. Confirm the UI shows the saved shortcut.
4. Use the new shortcut in the target text editor.
5. Reset hotkeys.
6. Confirm `F8` works again.
7. Try setting a duplicate shortcut.
8. Confirm the duplicate is rejected with an inline error.

## Overlay Pass

1. Start a normal recording and confirm the listening state.
2. Stop with speech and confirm processing and transcribed states.
3. Record silence and confirm the no-speech state.
4. Make a longer recording and confirm the long-recording duration state if
   transcription takes more than three seconds.
5. For the 105-second warning path, run one long recording and confirm the red
   stopping-soon countdown appears before the 120-second cap.

## Paste Last And Cancel Pass

1. Complete one successful dictation.
2. Move the cursor to a new line in the target editor.
3. Press `Ctrl+Alt+J`.
4. Confirm the last dictation is pasted.
5. Start another recording.
6. Press `Escape`.
7. Confirm recording cancels and no new history entry is written.

## Model Pass

1. Open Settings > Speech.
2. Confirm the installed speech model status is accurate.
3. Reload the model.
4. Confirm success is visible in the UI.
5. Delete the speech model only if a re-download is acceptable for the machine.
6. Confirm the UI prompts for download and dictation is blocked gracefully until
   the model is restored.

## Result Template

Record results outside the repo unless the user asks to commit a QA report.

```text
Commit:
Platform:
Artifact:
Fresh app data pass:
Dictation injection pass:
Hotkey pass:
Overlay pass:
Paste last and cancel pass:
Model pass:
Notes:
```
