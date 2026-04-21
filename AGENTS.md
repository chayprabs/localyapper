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
- Two Tauri windows: `main` (settings app, 900x650) and `overlay` (floating pill)
- `src-tauri/src/` — all Rust backend code
- `src/` — React/TypeScript frontend
- All commands use `#[tauri::command]` and register in `generate_handler![]` in `lib.rs`
- SQLite stores app data in 4 active tables: `transcription_history`, `corrections`, `personal_dictionary`, `settings`
- No cloud processing ever — everything local

## Voice pipeline data flow
`hotkey -> audio/capture.rs -> audio/vad.rs -> stt/whisper.rs -> correction/engine.rs -> injection/injector.rs -> text appears in app`

Current pipeline details:
- Capture: `cpal` 16kHz mono with pre-roll
- VAD: Silero VAD when available, energy fallback otherwise
- STT: Parakeet via `sherpa-onnx`
- Cleanup: correction engine only
- Injection: clipboard save -> paste simulation -> clipboard restore

## Models
- STT: Parakeet speech model downloaded to app data directory
- VAD: Silero VAD model downloaded alongside the speech model
- No local LLM
- No Ollama integration
- No BYOK/API-key cleanup path

## Session limits
- Max recording: 120 seconds
- Warning state at 105 seconds (last 15s = red countdown)
- Overlay countdown timer: max 15s safety cap
- Auto-inject delay: 10 seconds after transcription complete

## Commands
- `npm run dev` — Vite dev server
- `npm run tauri dev` — full Tauri dev mode
- `npm run build` — production build
- `cargo test --manifest-path src-tauri/Cargo.toml` — Rust tests
- `npm run lint` — ESLint
- `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` — Rust linter

## Verification — ALWAYS run after changes
1. Frontend: `npm run lint && npx tsc --noEmit`
2. Backend: `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`
3. Run relevant tests for changed code

## Implementation phases
18 phases total. See `docs/PROGRESS.md` for the current phase.
Always read `docs/PROGRESS.md` before starting work.
Only work on the current phase unless the user explicitly redirects you.

## Critical rules — NEVER break these
- NEVER modify `main.rs` directly — only modify `lib.rs`
- All commands must register in `generate_handler![]`
- IPC permissions must be in `src-tauri/capabilities/`
- `rusqlite` must use bundled feature — never system SQLite
- Text injection = clipboard save -> paste simulation -> clipboard restore
- No cloud processing ever — everything local
- No `unwrap()` in production Rust code — use `?` or explicit handling
- No `any` type in TypeScript — strict mode always
- Windows, macOS, Linux are all first-class platforms
- Audio is never written to disk — RAM only during processing
- Do not reintroduce Ollama, BYOK, or local LLM cleanup paths unless the user explicitly asks for them

## Design system
- See `DESIGN_SYSTEM.md` for colors, typography, spacing, and component specs
- Light mode only
- Apple macOS HIG design language
- Primary accent: `#0058bc`
- Font: SF Pro / Inter, two weights only: 400 regular, 600 bold

## Current status
Phase 17 complete. The current app is speech-recognition-only plus correction learning.
