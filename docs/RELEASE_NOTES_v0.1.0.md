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
- Best-effort system permission checks and OS settings shortcuts for microphone
  and accessibility/injection setup.
- Release logging uses the configured logger and does not include transcript
  content.
- Unused Tauri FS/Shell plugin permissions are not enabled.
- Branch release workflows cancel older in-progress runs for the same ref, while
  tag release runs are preserved.
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
npm run tauri build -- --bundles nsis
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
- Windows NSIS installer build: passed.
- Linux AppImage build in WSL: passed.
- GitHub Actions release workflow: passed on code-verification run
  `25927183008` for commit `2634165`.
- Post-baseline code hardening through commit `478ea3d` passed the relevant local
  gates: ESLint, TypeScript, frontend production build, Rust formatting check,
  Rust clippy with `-D warnings`, Rust tests, `npm audit --omit=dev`, and a
  Windows NSIS Tauri build after plugin-surface reduction.
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
