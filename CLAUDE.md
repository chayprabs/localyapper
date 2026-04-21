# LocalYapper

## What this is
Local-first, open-source, cross-platform voice dictation desktop app.
Privacy-focused alternative to Wispr Flow and SuperWhisper. Fully offline.
Windows 10+ / macOS 12+ / Linux (X11 + Wayland).
License: MIT.

## Stack — exact versions, do not deviate
- **Backend**: Tauri 2, Rust (stable 1.75+), rusqlite 0.31 (bundled), cpal 0.15, sherpa-onnx 1.12, tokio 1.x, serde, enigo 0.2
- **Frontend**: React 19, TypeScript 5, Vite 5, Tailwind CSS 3, shadcn/ui, Jotai 2, Recharts 2
- **IPC**: Tauri command system — frontend calls Rust via `invoke()` from `@tauri-apps/api/core`

## Architecture
- Two Tauri windows: `main` (settings app) and `overlay` (floating pill)
- `src-tauri/src/` — all Rust backend code
- `src/` — React/TypeScript frontend
- All commands use `#[tauri::command]` and register in `generate_handler![]` in `lib.rs`
- SQLite stores active app data in `transcription_history`, `corrections`, `personal_dictionary`, and `settings`
- No cloud processing ever — everything local

## Voice pipeline data flow
`hotkey -> audio/capture.rs -> audio/vad.rs -> stt/whisper.rs -> correction/engine.rs -> injection/injector.rs -> text appears in app`

Notes:
- Silero VAD is optional and falls back to energy-based filtering
- Correction learning still runs after successful dictations
- There is no LLM cleanup stage in the current product

## Models
- STT: Parakeet via `sherpa-onnx`
- VAD: Silero VAD
- No local LLM
- No Ollama
- No BYOK/API-key cleanup path

## Verification — ALWAYS run after changes
1. Frontend: `npm run lint && npx tsc --noEmit`
2. Backend: `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`
3. Run relevant tests for changed code

## Critical rules
- NEVER modify `main.rs` directly — only `lib.rs`
- `rusqlite` must use bundled feature
- Text injection must remain clipboard save -> paste -> restore
- Audio is never written to disk
- Do not reintroduce LLM/Ollama/BYOK paths unless explicitly requested

## Current status
Phase 17 complete. The shipped app is now STT + correction only.
