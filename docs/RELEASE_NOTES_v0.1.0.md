# LocalYapper v0.1.0 Release Notes

Status: release candidate
Date: 2026-05-16

## Summary

LocalYapper v0.1.0 is a local-first desktop dictation app for Windows, macOS,
and Linux. This release is speech-recognition-only by design: microphone audio
is captured locally, filtered with VAD, transcribed with a local Parakeet speech
model through `sherpa-onnx`, and pasted into the previously focused app through
the clipboard save -> paste -> restore flow.

No cloud processing is used. Audio is processed in memory and is not written to
disk.

## Included Features

- Tauri 2 desktop shell with separate `main` settings and `overlay` windows.
- First-launch wizard for speech model download and hotkey setup.
- Global hotkeys:
  - Record: `F8`
  - Hands-free: `Ctrl+F8`
  - Cancel: `Escape`
  - Paste last dictation: `Ctrl+Alt+J`
  - Open app: `Ctrl+Alt+O`
- Hotkey changes reject empty or duplicate shortcuts, report OS registration
  failures, and roll back to the previous shortcuts if reload fails.
- 16 kHz mono microphone capture through `cpal`.
- Silero VAD when available, with energy-based fallback.
- Parakeet speech recognition through `sherpa-onnx`.
- Floating overlay for listening, stopping soon, processing, long recording,
  transcribed, and no-speech feedback.
- Backend recording watchdog emits the 105-second stopping-soon warning and
  automatically stops/processes active hotkey recordings at the 120-second cap.
- Clipboard-based cross-platform text injection.
- Paste-failure recovery: if text injection fails after a successful
  transcription, the overlay keeps the transcript visible and copyable.
- Dashboard and History refresh after paste-failure recovery when the transcript
  has still been saved.
- Pipeline errors before transcript text exists, such as missing model files,
  now show a visible overlay error instead of silently hiding the overlay.
- Best-effort system permission checks and OS settings shortcuts for microphone
  and accessibility/injection setup.
- Release logging uses the configured logger and does not include transcript
  content.
- Unused Tauri FS/Shell plugin permissions are not enabled.
- Branch release workflows cancel older in-progress runs for the same ref, while
  tag release runs are preserved.
- Unsupported legacy speech model settings fall back to the current Parakeet
  default.
- Speech model downloads validate completed temp files and recover from stale
  incomplete destination files before installing the downloaded file.
- Startup preload and speech model status require both a valid ONNX file and
  `tokens.txt`, so partial model directories are reported as not installed
  instead of ready.
- Dashboard, History, Hotkeys, Speech, and Wizard setup operations show inline
  errors when user-facing actions fail.
- Speech model setup distinguishes failed downloads from successful downloads
  that need engine-start attention.
- The History empty-state action opens Hotkeys instead of doing nothing.
- Repository ignore rules exclude local agent state, private progress/PRD/source
  notes, cloud provider state, signing keys, model binaries, databases, build
  output, and packaged release artifacts.
- Manual release QA checklist covers real microphone capture, overlay behavior,
  hotkeys, model handling, and external-app injection.
- An ignored Windows Notepad smoke test can exercise the real clipboard
  injector against an external app and verify clipboard restoration, though
  Windows foreground focus can be inconsistent in non-user-driven test runs.
- An ignored Windows external textbox smoke test can exercise the same real
  clipboard injector against a separate focused GUI process when Notepad focus
  is blocked by the desktop session.
- An ignored microphone transcription smoke test can exercise default mic
  capture, VAD, and local STT with a human speaker.
- An ignored Windows synthetic speech-file smoke test can exercise VAD and local
  STT against a generated 16 kHz mono speech WAV.
- SQLite-backed history and settings.
- Dashboard, History, Hotkeys, and Speech model pages.
- System tray with open, pause/resume dictation, and quit actions.
- Autostart enabled by default.

## Removed Before v0.1.0

The following older product paths are intentionally not included:

- Dictionary page and training flow.
- Correction engine and learned correction cleanup.
- Local LLM cleanup.
- Ollama integration.
- BYOK/API-key cleanup providers.
- Processing modes.
- App profile routing.

## Verification Completed

Run on Windows in `C:\Users\chait\Project\localyapper`:

```bash
npm run lint
npm audit --omit=dev
npx tsc --noEmit
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
npm run build
npm run tauri build -- --bundles nsis
```

Results:

- ESLint: passed.
- Production npm audit: passed.
- TypeScript strict check: passed.
- Rust formatting check: passed.
- Rust clippy with `--all-targets -- -D warnings`: passed.
- Rust tests: 35 passed, 0 failed, 7 ignored manual desktop smokes.
- Ignored Windows Notepad injection smoke: available with
  `cargo test --manifest-path src-tauri/Cargo.toml manual_windows_notepad_injection_smoke -- --ignored --nocapture`.
  It passed in the interactive Windows desktop session and verified clipboard
  save -> paste -> restore against Notepad. It is still not treated as
  release-completing evidence by itself because it does not validate spoken
  microphone capture or the full hotkey-driven dictation flow.
- Ignored microphone transcription smoke: added for manual execution with
  `cargo test --manifest-path src-tauri/Cargo.toml manual_microphone_transcription_smoke -- --ignored --nocapture`.
  It supports configurable timing, optional wait-for-Enter start, optional input
  device selection, an optional expected-word check for release signoff, an
  optional Windows TTS prompt for speaker-to-microphone setups, and prints input
  devices, native capture format, RMS, and peak level.
- Ignored Windows synthetic speech-file smoke: passed locally with
  `cargo test --manifest-path src-tauri/Cargo.toml manual_windows_tts_file_transcription_smoke -- --ignored --nocapture`.
- Ignored Windows generated speech to Notepad smoke: available with
  `cargo test --manifest-path src-tauri/Cargo.toml manual_windows_tts_to_notepad_pipeline_smoke -- --ignored --nocapture`.
  It chains generated speech, VAD, local STT, real Notepad injection, file
  save, and clipboard restoration in one interactive check. It passed locally
  on Windows.
- Ignored Windows external textbox injection smoke: available with
  `cargo test --manifest-path src-tauri/Cargo.toml manual_windows_textbox_injection_smoke -- --ignored --nocapture`.
  It launches a separate WinForms textbox process, uses the real injector,
  verifies pasted text through that process's output file, and checks clipboard
  restoration. It passed locally on Windows.
- Ignored Windows microphone to Notepad smoke: available with
  `cargo test --manifest-path src-tauri/Cargo.toml manual_windows_microphone_to_notepad_pipeline_smoke -- --ignored --nocapture`.
  It chains selected microphone capture, an optional tunable Windows speech
  prompt, VAD, local STT, optional expected-word validation, real Notepad
  injection, file save, and clipboard restoration in one interactive check. It
  is the remaining human-spoken or physical speaker-to-microphone release
  signoff path.
- Ignored Windows microphone to textbox smoke: available with
  `cargo test --manifest-path src-tauri/Cargo.toml manual_windows_microphone_to_textbox_pipeline_smoke -- --ignored --nocapture`.
  It uses the same microphone and STT path but targets the external WinForms
  textbox process when Notepad focus is blocked.
- Frontend production build: passed.
- Tauri release build on Windows: passed.
- Windows artifact produced:
  `src-tauri/target/release/localyapper.exe`
- Windows NSIS installer build: passed.
- Linux AppImage build in WSL: passed.
- GitHub Actions release workflow: passed for code-bearing checkpoint
  `9b59c39` in run `25969254251`, including production audit,
  all-targets clippy, Verify, Windows, Linux, macOS Intel, and macOS Apple
  Silicon jobs.
- Most recent local gates at release-note time passed: `npm audit --omit=dev`,
  ESLint, TypeScript, Rust clippy with `--all-targets -- -D warnings`, Rust
  tests, controlled `npm run tauri dev` launch smoke, and generated speech STT
  smoke validation.
- The most recent local Rust test run passed 35 tests with 7 ignored manual
  desktop smokes.
- Fresh database setup tests cover current default settings, legacy feature
  cleanup, and stale speech model normalization.
- History and dashboard query tests cover reverse-chronological paging,
  aggregate statistics, missing-entry deletion errors, and clear-all behavior.
- GitHub Actions artifacts uploaded:
  - `localyapper-windows-x64`
  - `localyapper-macos-aarch64`
  - `localyapper-macos-x64`
  - `localyapper-linux-x64`

## Platform Notes

Windows packaging and Linux AppImage packaging were verified locally. macOS and
Linux release artifacts are built by the GitHub Actions workflow on native CI
runners.

Runtime injection dependencies on Linux:

- X11: `xclip` and `xdotool`
- Wayland: `wl-copy`, `wl-paste`, and `wtype`

## Known Limitations

- The overlay uses frontend timers for display countdowns and auto-hide while
  backend events drive major pipeline state transitions.
- First launch is tracked with the `setup_complete` setting, not a separate
  onboarding table.
