# LocalYapper v0.1.0 Release Notes

Status: release candidate
Date: 2026-05-15

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
- 16 kHz mono microphone capture through `cpal`.
- Silero VAD when available, with energy-based fallback.
- Parakeet speech recognition through `sherpa-onnx`.
- Floating overlay for listening, stopping soon, processing, long recording,
  transcribed, and no-speech feedback.
- Clipboard-based cross-platform text injection.
- Paste-failure recovery: if text injection fails after a successful
  transcription, the overlay keeps the transcript visible and copyable.
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
npx tsc --noEmit
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
npm run build
npm run tauri build
```

Results:

- ESLint: passed.
- TypeScript strict check: passed.
- Rust formatting check: passed.
- Rust clippy with `-D warnings`: passed.
- Rust tests: 21 passed, 0 failed.
- Frontend production build: passed.
- Tauri release build on Windows: passed.
- Windows artifact produced:
  `src-tauri/target/release/localyapper.exe`
- GitHub Actions workflow added for verify plus cross-platform Tauri builds on
  Windows, macOS, and Linux.

## Platform Notes

Windows packaging was verified locally. macOS and Linux builds require native
hosts or CI runners with the prerequisites listed in `docs/BUILD.md`.

Runtime injection dependencies on Linux:

- X11: `xclip` and `xdotool`
- Wayland: `wl-copy`, `wl-paste`, and `wtype`

## Known Limitations

- macOS and Linux release artifacts have not been built in this local Windows
  run; they are covered by the GitHub Actions release workflow and need a green
  run before publishing v0.1.0.
- Permission status commands are registered but currently return placeholder
  values and are not surfaced by the simplified UI.
- The overlay uses frontend timers for display countdowns and auto-hide while
  backend events drive major pipeline state transitions.
- First launch is tracked with the `setup_complete` setting, not a separate
  onboarding table.
