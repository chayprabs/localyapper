# LocalYapper

## What this is

Local-first, open-source, cross-platform voice dictation desktop app.
Privacy-focused alternative to Wispr Flow and SuperWhisper. Fully offline.
Windows 10+, macOS 12+, Linux (X11 + Wayland).
License: MIT.

## Stack — exact versions, do not deviate

- **Backend**: Tauri 2, Rust (stable 1.75+), `rusqlite` 0.31 (bundled),
  `cpal` 0.15, `sherpa-onnx` 1.12, `tokio` 1.x, `serde`, `enigo` 0.2
- **Frontend**: React 19, TypeScript 5, Vite 5, Tailwind CSS 3, shadcn/ui
  (used minimally), Jotai 2, Recharts 2
- **IPC**: Tauri command system. Frontend calls Rust via `invoke()` from
  `@tauri-apps/api/core` through typed wrappers in `src/lib/commands/`.

## Architecture

- Two Tauri windows: `main` (settings) and `overlay` (floating pill).
- `src-tauri/src/` — all Rust backend code.
- `src/` — React/TypeScript frontend.
- All commands use `#[tauri::command]` and register in
  `tauri::generate_handler![]` in `lib.rs`.
- SQLite stores app data in `transcription_history` and `settings`. That is
  it. No corrections, dictionary, or training tables.
- Nothing leaves the device. No cloud STT, no remote LLM cleanup.

## Voice pipeline data flow

`hotkey -> audio/capture.rs -> audio/vad.rs -> stt/whisper.rs -> injection/injector.rs -> text appears in app`

Notes:

- Audio capture is 16 kHz mono via `cpal`, in memory only.
- VAD uses Silero VAD when present, energy fallback otherwise.
- STT is Parakeet via `sherpa-onnx`.
- Injection is clipboard save -> set -> paste -> restore. The previously
 focused window is captured before the overlay shows.
- There is no correction pass and no LLM cleanup stage.

## Memory / lifecycle

- Parakeet and Silero VAD are NOT preloaded at startup. The pipeline lazy-
 loads them on the first dictation and `stt::lifecycle::ModelLifecycle`
 schedules them to be dropped from `AppState` after the idle window
 (default 60s, override via the `idle_unload_seconds` setting).
- The frontend can listen for `model-state` events
 (`{ loaded: bool, state: "loaded" | "unloaded" | "loading" }`) or call
 the `get_model_state` IPC command to render a loading hint.
- The main settings window is **destroyed** on close (not hidden). It is
 rebuilt on demand by `show_or_create_main_window()` when the tray icon,
 tray menu, or open-app hotkey asks for it. The overlay window stays
 alive so hotkey response is instant.
- mimalloc is the global allocator. sherpa-onnx's alloc/free pattern
 fragments badly under the Windows system allocator.

## Models

- STT: Parakeet via `sherpa-onnx` (`parakeet-110m` default).
- VAD: Silero VAD.
- No local LLM. No Ollama. No BYOK.

## Verification — ALWAYS run after changes

1. Frontend: `npm run lint && npx tsc --noEmit`
2. Backend: `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
3. Run relevant tests for changed code.

## Critical rules

- NEVER modify `main.rs` directly. Only `lib.rs`.
- `rusqlite` must use the bundled feature.
- Text injection must remain clipboard save -> set -> paste -> restore.
- Audio is never written to disk.
- No `unwrap()` / `expect()` in production Rust paths.
- No `any` in TypeScript.
- Do not reintroduce LLM, Ollama, BYOK, or correction-learning paths unless
  explicitly requested. They were removed deliberately.

## Current status

v0.1.0 release candidate. The shipped app is STT-only with Parakeet. See
`docs/PRODUCT_AUDIT.md` for the remaining tracked work and `AGENTS.md` for
the longer contributor guide.
