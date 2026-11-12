# LocalYapper

## What this is

Offline, open-source voice dictation for the desktop: press a key, speak,
let go, and the text is pasted into the focused app. Windows 10+, macOS
12+, Linux (X11; Wayland with documented limits). MIT licence.

## Stack

- **Back end**: Tauri 2, Rust stable, `sherpa-onnx` 2.0 (Parakeet and
  Moonshine models, Silero VAD), `cpal` 0.15, `ringbuf` 0.4, `rusqlite`
  0.31 (bundled), `tokio`, `enigo` 0.2, `arboard`, `mimalloc`, `sha2`,
  `reqwest` (model downloads only). Plugins: single-instance (first),
  global-shortcut, autostart, dialog, log, opener; `tauri-nspanel` on
  macOS, `ashpd` on Linux (Wayland portal).
- **Front end**: React 19, TypeScript 5, Vite 5, Tailwind CSS 3, Jotai 2,
  lucide-react icons. Typed IPC wrappers in `src/lib/commands/`.

## Layout

- `src-tauri/src/core/`: platform-free logic. `session.rs` is the
  dictation state machine (pure, fully unit-tested); `audio/` segmenter,
  rolling buffer, conversions; `stt/` engine, worker, model catalog;
  `download.rs` verified downloader.
- `src-tauri/src/platform/`: OS code (audio input, hotkeys, injection,
  overlay, focus, permissions, memory).
- `src-tauri/src/controller.rs`: the only owner of dictation state.
- `src-tauri/src/settings/`: typed settings, validators and apply hooks.
- `src/`: settings window (React). `overlay.html` + `src/overlay-main.ts`:
  overlay pill (no React).
- `docs/ARCHITECTURE.md` explains the flow in detail.

## Rules

- Never edit `src-tauri/src/main.rs`; wire things up in `lib.rs`.
- Audio is never written to disk. Logs never contain dictated or
  clipboard text.
- Injection stays clipboard save → set → paste → restore.
- Nothing leaves the device except model downloads (pinned SHA-256) and
  the user opening the Releases page.
- No `unwrap()` / `expect()` in production Rust; no `any` in TypeScript.
- Every setting goes through `settings/keys.rs` with a validator, a hook
  and a hook test.
- No LLM, Ollama, BYOK or correction-learning features.
- GTK and window geometry calls run on the main thread (calling them from
  other threads corrupts GTK's X connection on Linux).

## Verify after changes

```bash
npm run lint && npx tsc --noEmit
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

Model, pipeline and smoke tests are described in `docs/BUILD.md`; CI
runs them on Windows, macOS and Linux.

## Status

Version 2.0.0, rebuilt per `docs/REBUILD_PLAN.md`. Results are in
`docs/BUILD_REPORT.md`, measurements in `docs/PERFORMANCE.md` and
`docs/MODEL_BENCHMARK.md`.
